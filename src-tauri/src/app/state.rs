//! The single owner of "what is DropLAN doing right now".
//!
//! Commands and events both go through here, so there is one place that knows
//! how to start sharing, how to stop it, and what the UI should be shown. It
//! holds no Tauri types: the desktop shell wraps this, not the other way round.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::Serialize;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};
use crate::events::{names, AppEvent, EventBus};
use crate::network::discovery::MdnsAdvertiser;
use crate::network::interfaces::{self, NetworkInterface, NetworkSnapshot};
use crate::network::watcher::{self, NetworkEvent, NetworkWatcher};
use crate::platform::{self, PlatformNotice};
use crate::server::{self, ServerContext, ServerHandle};
use crate::settings::{AppSettings, SettingsStore};
use crate::sharing::{
    AddOutcome, RegistryTotals, SessionInfo, ShareItem, ShareRegistry, ShareSession,
    SharedFilesPayload,
};
use crate::transfer::{ActivitySnapshot, TransferTracker};

/// Above this many shared files, availability is only re-checked on demand
/// rather than on every network event.
const AUTO_REFRESH_FILE_LIMIT: usize = 500;

/// Everything the desktop UI renders, in one snapshot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareState {
    pub sharing: bool,
    pub device_name: String,
    pub network: NetworkSnapshot,
    pub session: Option<SessionInfo>,
    pub port: Option<u16>,
    /// `http://192.168.1.42:8080/s/<token>` — the address to hand out.
    pub share_url: Option<String>,
    /// `http://droplan-macbook-pro.local:8080/s/<token>` when mDNS is up.
    pub friendly_url: Option<String>,
    pub files: Vec<ShareItem>,
    pub totals: RegistryTotals,
    pub settings: AppSettings,
    pub platform_notice: PlatformNotice,
}

struct RunningServer {
    handle: ServerHandle,
    cancel: CancellationToken,
    port: u16,
}

pub struct AppState {
    pub settings: Arc<SettingsStore>,
    pub registry: Arc<ShareRegistry>,
    pub tracker: Arc<TransferTracker>,
    pub events: EventBus,
    pub device_name: Arc<str>,
    platform_notice: PlatformNotice,

    network: RwLock<NetworkSnapshot>,
    /// Shared with the HTTP layer; swapping the contents rotates the link.
    session: Arc<RwLock<ShareSession>>,
    server: Mutex<Option<RunningServer>>,
    mdns: Mutex<Option<MdnsAdvertiser>>,
    watcher: Mutex<Option<NetworkWatcher>>,
}

impl AppState {
    /// Build state from an on-disk config directory. Does not touch the
    /// network or start anything.
    pub fn new(config_dir: &Path) -> Result<Arc<Self>> {
        let events = EventBus::default();
        let settings = Arc::new(SettingsStore::load(config_dir));
        let device_name: Arc<str> = Arc::from(platform::device_name());

        Ok(Arc::new(AppState {
            registry: Arc::new(ShareRegistry::new()),
            tracker: Arc::new(TransferTracker::new(events.clone())),
            platform_notice: platform::firewall_notice(),
            settings,
            events,
            device_name,
            network: RwLock::new(NetworkSnapshot::default()),
            // A session exists from the start so the token is ready the moment
            // sharing begins; it is replaced on every start and regenerate.
            session: Arc::new(RwLock::new(ShareSession::new(false)?)),
            server: Mutex::new(None),
            mdns: Mutex::new(None),
            watcher: Mutex::new(None),
        }))
    }

    pub fn settings_path(&self) -> PathBuf {
        self.settings.path().to_path_buf()
    }

    // ---------------------------------------------------------------- network

    /// Re-detect interfaces and store the result.
    pub fn refresh_network(&self) -> NetworkSnapshot {
        let preferred = self.settings.get().preferred_interface;
        let snapshot = match interfaces::detect(preferred.as_deref()) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                tracing::warn!(target: "droplan", "network detection failed: {err}");
                NetworkSnapshot::default()
            }
        };
        if let Ok(mut guard) = self.network.write() {
            *guard = snapshot.clone();
        }
        snapshot
    }

    pub fn network(&self) -> NetworkSnapshot {
        self.network
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn selected_interface(&self) -> Option<NetworkInterface> {
        self.network().selected
    }

    // ---------------------------------------------------------------- sharing

    pub async fn is_sharing(&self) -> bool {
        self.server.lock().await.is_some()
    }

    /// Start the listener and open a fresh session.
    ///
    /// Starting twice is not an error; the second call just reports the state
    /// that is already live.
    pub async fn start_sharing(self: &Arc<Self>) -> Result<ShareState> {
        let mut server_slot = self.server.lock().await;
        if server_slot.is_some() {
            drop(server_slot);
            return Ok(self.share_state().await);
        }

        let snapshot = self.refresh_network();
        if snapshot.selected.is_none() {
            return Err(Error::NoPrivateNetwork);
        }

        let settings = self.settings.get();

        // A new session on every start: yesterday's link must be dead.
        let session = ShareSession::new(settings.require_pin)?;
        if let Ok(mut guard) = self.session.write() {
            *guard = session;
        }

        let cancel = CancellationToken::new();
        let context = ServerContext {
            registry: Arc::clone(&self.registry),
            session: Arc::clone(&self.session),
            tracker: Arc::clone(&self.tracker),
            events: self.events.clone(),
            device_name: Arc::clone(&self.device_name),
            shutdown: cancel.clone(),
        };

        let handle =
            server::start(context, settings.preferred_port, settings.port_scan_range).await?;
        let port = handle.port;
        *server_slot = Some(RunningServer {
            handle,
            cancel,
            port,
        });
        drop(server_slot);

        if settings.enable_mdns {
            self.start_mdns(port).await;
        }

        let state = self.share_state().await;
        self.events
            .publish(AppEvent::new(names::SHARING_STARTED, &state));
        Ok(state)
    }

    /// Stop the listener and forget the session's activity.
    pub async fn stop_sharing(&self) -> Result<()> {
        self.stop_mdns().await;

        let running = self.server.lock().await.take();
        if let Some(running) = running {
            // Cancelling before shutdown makes in-flight downloads stop too,
            // which is what "stop sharing" has to mean.
            running.cancel.cancel();
            running.handle.shutdown().await;
        }

        self.tracker.reset();
        self.events.publish(AppEvent::bare(names::SHARING_STOPPED));
        Ok(())
    }

    /// Issue a brand-new token, invalidating every link handed out so far.
    pub async fn regenerate_session(self: &Arc<Self>) -> Result<ShareState> {
        let settings = self.settings.get();
        let session = ShareSession::new(settings.require_pin)?;
        if let Ok(mut guard) = self.session.write() {
            *guard = session;
        }

        let sharing = self.is_sharing().await;
        // The advertised TXT record carries the session path, so it has to be
        // republished with the new token.
        if sharing && settings.enable_mdns {
            let port = self.current_port().await;
            self.stop_mdns().await;
            if let Some(port) = port {
                self.start_mdns(port).await;
            }
        }

        let state = self.share_state().await;
        // Only announce a live session. Rotating the token while stopped — as
        // switching the PIN setting does — must not tell the UI or the tray
        // that sharing just started.
        if sharing {
            self.events
                .publish(AppEvent::new(names::SHARING_STARTED, &state));
        }
        Ok(state)
    }

    /// Rebind the listener without touching the session or the file list.
    /// Used when the preferred port changes.
    pub async fn restart_server(self: &Arc<Self>) -> Result<ShareState> {
        if !self.is_sharing().await {
            return Ok(self.share_state().await);
        }

        self.stop_mdns().await;
        if let Some(running) = self.server.lock().await.take() {
            running.cancel.cancel();
            running.handle.shutdown().await;
        }

        let settings = self.settings.get();
        let cancel = CancellationToken::new();
        let context = ServerContext {
            registry: Arc::clone(&self.registry),
            session: Arc::clone(&self.session),
            tracker: Arc::clone(&self.tracker),
            events: self.events.clone(),
            device_name: Arc::clone(&self.device_name),
            shutdown: cancel.clone(),
        };

        let handle =
            server::start(context, settings.preferred_port, settings.port_scan_range).await?;
        let port = handle.port;
        *self.server.lock().await = Some(RunningServer {
            handle,
            cancel,
            port,
        });

        if settings.enable_mdns {
            self.start_mdns(port).await;
        }

        let state = self.share_state().await;
        self.events
            .publish(AppEvent::new(names::SHARING_STARTED, &state));
        Ok(state)
    }

    async fn current_port(&self) -> Option<u16> {
        self.server
            .lock()
            .await
            .as_ref()
            .map(|running| running.port)
    }

    // ------------------------------------------------------------------- mDNS

    async fn start_mdns(&self, port: u16) {
        let Some(selected) = self.selected_interface() else {
            return;
        };
        let Some(ip) = selected.ipv4() else {
            return;
        };
        let path = self
            .session
            .read()
            .map(|session| session.base_path())
            .unwrap_or_default();

        match crate::network::discovery::advertise(&self.device_name, ip, port, &path) {
            Ok(advertiser) => {
                *self.mdns.lock().await = Some(advertiser);
            }
            Err(err) => {
                // Never fatal: the numeric URL is always available.
                tracing::warn!(target: "droplan", "mDNS advertisement failed: {err}");
                self.events.publish(AppEvent::notice(
                    "mdns_unavailable",
                    "The .local name could not be published. The numeric address still works.",
                ));
            }
        }
    }

    async fn stop_mdns(&self) {
        if let Some(advertiser) = self.mdns.lock().await.take() {
            advertiser.stop();
        }
    }

    // ------------------------------------------------------------------ files

    pub fn add_files<P: AsRef<Path>>(&self, paths: &[P]) -> Result<AddOutcome> {
        let outcome = self.registry.add_paths(paths)?;
        if !outcome.added.is_empty() {
            self.publish_files_changed();
        } else if outcome.skipped_duplicates == 0 {
            // Nothing new and nothing already shared: the user picked things
            // we genuinely could not read, and deserves to be told.
            return Err(Error::NoFilesAdded);
        }
        Ok(outcome)
    }

    pub fn remove_file(&self, id: &str) -> Result<bool> {
        let removed = self.registry.remove(id)?;
        if removed {
            self.publish_files_changed();
        }
        Ok(removed)
    }

    pub fn clear_files(&self) -> Result<usize> {
        let cleared = self.registry.clear()?;
        if cleared > 0 {
            self.publish_files_changed();
        }
        Ok(cleared)
    }

    /// Re-stat shared files and report whether anything changed.
    pub fn refresh_files(&self) -> Result<bool> {
        let changed = self.registry.refresh_availability()?;
        if changed {
            self.publish_files_changed();
        }
        Ok(changed)
    }

    fn publish_files_changed(&self) {
        self.events
            .publish(crate::sharing::files_changed_event(&self.registry));
    }

    pub fn files_payload(&self) -> SharedFilesPayload {
        SharedFilesPayload::of(&self.registry)
    }

    // --------------------------------------------------------------- settings

    pub fn update_settings<F>(&self, mutate: F) -> Result<AppSettings>
    where
        F: FnOnce(&mut AppSettings),
    {
        self.settings.update(mutate)
    }

    // --------------------------------------------------------------- activity

    pub fn activity(&self) -> ActivitySnapshot {
        self.tracker.snapshot()
    }

    // ------------------------------------------------------------------ state

    /// The full picture for the UI.
    pub async fn share_state(&self) -> ShareState {
        let sharing = self.is_sharing().await;
        let port = self.current_port().await;
        let network = self.network();
        let settings = self.settings.get();

        let session_info = self
            .session
            .read()
            .ok()
            .map(|session| SessionInfo::from(&*session));

        let share_url = match (
            sharing,
            port,
            network.selected.as_ref(),
            session_info.as_ref(),
        ) {
            (true, Some(port), Some(selected), Some(session)) => Some(format!(
                "http://{}:{}{}",
                selected.address, port, session.base_path
            )),
            _ => None,
        };

        let friendly_url = if sharing {
            let path = session_info
                .as_ref()
                .map(|session| session.base_path.clone())
                .unwrap_or_default();
            self.mdns
                .lock()
                .await
                .as_ref()
                .map(|advertiser| advertiser.friendly_url(&path))
        } else {
            None
        };

        ShareState {
            sharing,
            device_name: self.device_name.to_string(),
            network,
            // Hide the token entirely while stopped, so a stale link cannot be
            // copied out of the UI after sharing has ended.
            session: if sharing { session_info } else { None },
            port: if sharing { port } else { None },
            share_url,
            friendly_url,
            files: self.registry.list().unwrap_or_default(),
            totals: self.registry.totals().unwrap_or_default(),
            settings,
            platform_notice: self.platform_notice.clone(),
        }
    }

    /// Re-select the interface after the user pinned a different one, and
    /// republish anything that carries the address.
    pub async fn apply_interface_change(self: &Arc<Self>) -> ShareState {
        self.refresh_network();

        if self.is_sharing().await && self.settings.get().enable_mdns {
            if let Some(port) = self.current_port().await {
                self.stop_mdns().await;
                self.start_mdns(port).await;
            }
        }

        let state = self.share_state().await;
        self.events
            .publish(AppEvent::new(names::NETWORK_CHANGED, &state));
        state
    }

    // ---------------------------------------------------------------- watcher

    /// Begin watching for network changes. Events are handled internally and
    /// republished to the UI.
    pub async fn start_network_watcher(self: &Arc<Self>) {
        let mut slot = self.watcher.lock().await;
        if slot.is_some() {
            return;
        }

        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let settings = Arc::clone(&self.settings);
        let initial = self.network();

        let watcher = watcher::spawn(
            watcher::DEFAULT_INTERVAL,
            move || settings.get().preferred_interface,
            sender,
            Some(&initial),
        );
        *slot = Some(watcher);
        drop(slot);

        let state = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                state.handle_network_event(event).await;
            }
        });
    }

    pub async fn stop_network_watcher(&self) {
        if let Some(watcher) = self.watcher.lock().await.take() {
            watcher.stop().await;
        }
    }

    async fn handle_network_event(self: &Arc<Self>, event: NetworkEvent) {
        let (snapshot, resumed) = match event {
            NetworkEvent::Changed(snapshot) => (*snapshot, false),
            NetworkEvent::Resumed(snapshot) => (*snapshot, true),
        };

        if let Ok(mut guard) = self.network.write() {
            *guard = snapshot;
        }

        // Files may have moved while we were asleep or while the user was
        // elsewhere; a cheap re-stat keeps the list honest.
        if self.registry.list().map(|list| list.len()).unwrap_or(0) <= AUTO_REFRESH_FILE_LIMIT {
            let _ = self.refresh_files();
        }

        let sharing = self.is_sharing().await;
        let has_network = self.network().has_usable_interface();

        if sharing && !has_network {
            self.events.publish(AppEvent::notice(
                "network_lost",
                "The local network is unavailable. Sharing will resume automatically when you reconnect.",
            ));
        }

        // The address changed, so the .local record points at the wrong IP.
        if sharing && has_network && self.settings.get().enable_mdns {
            if let Some(port) = self.current_port().await {
                self.stop_mdns().await;
                self.start_mdns(port).await;
            }
        }

        let state = self.share_state().await;
        self.events
            .publish(AppEvent::new(names::NETWORK_CHANGED, &state));
        if resumed {
            self.events.publish(AppEvent::bare(names::SYSTEM_RESUMED));
        }
    }

    // --------------------------------------------------------------- shutdown

    /// Tear everything down in the right order. Safe to call more than once.
    pub async fn shutdown(&self) {
        self.stop_network_watcher().await;
        let _ = self.stop_sharing().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> Arc<AppState> {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = AppState::new(dir.path()).expect("state");
        // Keep the directory alive for the lifetime of the test process.
        std::mem::forget(dir);
        state
    }

    #[tokio::test]
    async fn a_fresh_state_is_not_sharing_and_exposes_no_url() {
        let state = state();
        let view = state.share_state().await;

        assert!(!view.sharing);
        assert!(view.share_url.is_none());
        assert!(view.session.is_none(), "no token is exposed while stopped");
        assert!(view.files.is_empty());
        assert_eq!(view.totals.file_count, 0);
        assert!(!view.device_name.is_empty());
    }

    #[tokio::test]
    async fn starting_and_stopping_toggles_the_url() {
        let state = state();
        state.refresh_network();
        if !state.network().has_usable_interface() {
            // No LAN on this machine (a locked-down CI box); nothing to assert.
            return;
        }

        let started = state.start_sharing().await.expect("start");
        assert!(started.sharing);
        let url = started.share_url.expect("a share url");
        assert!(url.starts_with("http://"));
        assert!(url.contains("/s/"));
        assert!(!url.contains("127.0.0.1"));
        assert!(!url.contains("0.0.0.0"));

        // Starting again is idempotent, not an error.
        let again = state.start_sharing().await.expect("start again");
        assert_eq!(again.port, started.port);

        state.stop_sharing().await.expect("stop");
        let stopped = state.share_state().await;
        assert!(!stopped.sharing);
        assert!(stopped.share_url.is_none());
        assert!(stopped.session.is_none());
    }

    #[tokio::test]
    async fn regenerating_replaces_the_token_but_keeps_the_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"x").expect("write");

        let state = state();
        state.add_files(&[file]).expect("add");
        state.refresh_network();
        if !state.network().has_usable_interface() {
            return;
        }

        let first = state.start_sharing().await.expect("start");
        let first_token = first.session.expect("session").token;

        let second = state.regenerate_session().await.expect("regenerate");
        let second_token = second.session.expect("session").token;

        assert_ne!(first_token, second_token);
        assert_eq!(second.files.len(), 1, "regenerating must not drop files");
        state.stop_sharing().await.expect("stop");
    }

    #[tokio::test]
    async fn adding_and_removing_files_updates_the_totals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, vec![0u8; 1000]).expect("write");
        std::fs::write(&b, vec![0u8; 2000]).expect("write");

        let state = state();
        let outcome = state.add_files(&[a, b]).expect("add");
        assert_eq!(outcome.added.len(), 2);

        let view = state.share_state().await;
        assert_eq!(view.totals.file_count, 2);
        assert_eq!(view.totals.total_bytes, 3000);

        let id = view.files[0].id.clone();
        assert!(state.remove_file(&id).expect("remove"));
        assert_eq!(state.share_state().await.totals.file_count, 1);

        assert_eq!(state.clear_files().expect("clear"), 1);
        assert_eq!(state.share_state().await.totals.file_count, 0);
    }

    #[tokio::test]
    async fn adding_nothing_usable_is_reported_as_an_error() {
        let state = state();
        let missing = PathBuf::from("/definitely/not/here.txt");
        let err = state.add_files(&[missing]).expect_err("should fail");
        assert_eq!(err.code(), "no_files_added");
    }

    #[tokio::test]
    async fn file_changes_are_announced_to_the_ui() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("a.txt");
        std::fs::write(&file, b"x").expect("write");

        let state = state();
        let mut receiver = state.events.subscribe();
        state.add_files(&[file]).expect("add");

        let event = receiver.recv().await.expect("event");
        assert_eq!(event.name, names::SHARED_FILES_CHANGED);
        assert_eq!(event.payload["totals"]["fileCount"], 1);
    }

    #[tokio::test]
    async fn settings_changes_are_persisted_and_visible() {
        let state = state();
        state
            .update_settings(|settings| settings.preferred_port = 9123)
            .expect("update");
        assert_eq!(state.share_state().await.settings.preferred_port, 9123);
    }

    #[tokio::test]
    async fn stopping_when_not_sharing_is_harmless() {
        let state = state();
        state.stop_sharing().await.expect("stop");
        state.stop_sharing().await.expect("stop again");
        assert!(!state.is_sharing().await);
    }

    #[tokio::test]
    async fn shutdown_leaves_nothing_running() {
        let state = state();
        state.start_network_watcher().await;
        state.shutdown().await;
        assert!(!state.is_sharing().await);
    }
}
