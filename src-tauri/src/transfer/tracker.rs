//! Live download tracking and recent-client bookkeeping.
//!
//! Everything here is in memory and capped. Nothing is written to disk: a
//! record of who downloaded what from your laptop is not something to leave
//! lying around after the app closes.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::events::{names, AppEvent, EventBus};
use crate::security::tokens;
use crate::sharing::registry::now_millis;

/// Completed transfers kept for the activity list.
const MAX_RECENT_TRANSFERS: usize = 25;
/// Distinct clients remembered.
const MAX_CLIENTS: usize = 20;
/// Minimum wall-clock gap between two `transfer-progress` events for one
/// transfer, so a fast local copy cannot flood the UI thread.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// …or this many bytes, whichever comes first, so slow links still update.
const PROGRESS_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Active,
    Completed,
    Failed,
}

/// One download, as shown in the desktop UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSnapshot {
    pub id: String,
    pub file_id: String,
    pub file_name: String,
    /// Bytes this response will deliver (the range length, not the file size).
    pub total_bytes: u64,
    /// Size of the whole file, for context when a range was requested.
    pub file_bytes: u64,
    pub transferred_bytes: u64,
    pub is_range_request: bool,
    pub client_ip: String,
    pub user_agent: Option<String>,
    pub status: TransferStatus,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}

impl TransferSnapshot {
    pub fn percent(&self) -> u8 {
        if self.total_bytes == 0 {
            return 100;
        }
        let ratio = (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0;
        ratio.clamp(0.0, 100.0) as u8
    }
}

/// A device that has talked to us during this session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSnapshot {
    pub ip: String,
    pub user_agent: Option<String>,
    /// Best-effort, never authoritative: "iPhone", "Windows", "Android"…
    pub device: String,
    /// "Safari", "Chrome", "Firefox"…
    pub browser: String,
    pub requests: u64,
    pub first_seen: u64,
    pub last_seen: u64,
}

/// Everything the activity panel needs in one call.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySnapshot {
    pub active: Vec<TransferSnapshot>,
    pub recent: Vec<TransferSnapshot>,
    pub clients: Vec<ClientSnapshot>,
    pub total_bytes_served: u64,
}

/// Details needed to open a transfer record.
pub struct TransferStart {
    pub file_id: String,
    pub file_name: String,
    pub total_bytes: u64,
    pub file_bytes: u64,
    pub is_range_request: bool,
    pub client_ip: String,
    pub user_agent: Option<String>,
}

struct ActiveTransfer {
    snapshot: TransferSnapshot,
    last_emit: Instant,
    bytes_since_emit: u64,
}

#[derive(Default)]
struct TrackerInner {
    active: HashMap<String, ActiveTransfer>,
    /// Newest first.
    recent: Vec<TransferSnapshot>,
    clients: HashMap<String, ClientSnapshot>,
    total_bytes_served: u64,
}

pub struct TransferTracker {
    inner: RwLock<TrackerInner>,
    events: EventBus,
}

impl TransferTracker {
    pub fn new(events: EventBus) -> Self {
        TransferTracker {
            inner: RwLock::new(TrackerInner::default()),
            events,
        }
    }

    /// Record a request that is not a download (share page, file list, …).
    pub fn note_request(&self, ip: &str, user_agent: Option<&str>) {
        let changed = {
            let Ok(mut guard) = self.inner.write() else {
                return;
            };
            touch_client(&mut guard.clients, ip, user_agent)
        };
        if changed {
            self.publish_clients();
        }
    }

    /// Open a transfer. The returned guard reports progress and closes the
    /// record even if the client disconnects mid-stream.
    pub fn begin(self: &Arc<Self>, start: TransferStart) -> TransferGuard {
        let id = tokens::file_id().unwrap_or_else(|_| format!("t{}", now_millis()));
        let snapshot = TransferSnapshot {
            id: id.clone(),
            file_id: start.file_id,
            file_name: start.file_name,
            total_bytes: start.total_bytes,
            file_bytes: start.file_bytes,
            transferred_bytes: 0,
            is_range_request: start.is_range_request,
            client_ip: start.client_ip.clone(),
            user_agent: start.user_agent.clone(),
            status: TransferStatus::Active,
            started_at: now_millis(),
            finished_at: None,
        };

        if let Ok(mut guard) = self.inner.write() {
            touch_client(
                &mut guard.clients,
                &start.client_ip,
                start.user_agent.as_deref(),
            );
            guard.active.insert(
                id.clone(),
                ActiveTransfer {
                    snapshot: snapshot.clone(),
                    last_emit: Instant::now(),
                    bytes_since_emit: 0,
                },
            );
        }

        self.events
            .publish(AppEvent::new(names::TRANSFER_STARTED, &snapshot));
        self.publish_clients();

        TransferGuard {
            tracker: Arc::clone(self),
            id,
            finished: false,
        }
    }

    fn advance(&self, id: &str, delta: u64) {
        if delta == 0 {
            return;
        }
        let due = {
            let Ok(mut guard) = self.inner.write() else {
                return;
            };
            let Some(entry) = guard.active.get_mut(id) else {
                return;
            };
            entry.snapshot.transferred_bytes =
                entry.snapshot.transferred_bytes.saturating_add(delta);
            entry.bytes_since_emit = entry.bytes_since_emit.saturating_add(delta);

            let elapsed = entry.last_emit.elapsed();
            if elapsed >= PROGRESS_INTERVAL || entry.bytes_since_emit >= PROGRESS_BYTES {
                entry.last_emit = Instant::now();
                entry.bytes_since_emit = 0;
                Some(entry.snapshot.clone())
            } else {
                None
            }
        };

        if let Some(snapshot) = due {
            self.events
                .publish(AppEvent::new(names::TRANSFER_PROGRESS, &snapshot));
        }
    }

    fn finish(&self, id: &str, success: bool) {
        let finished = {
            let Ok(mut guard) = self.inner.write() else {
                return;
            };
            let Some(mut entry) = guard.active.remove(id) else {
                return;
            };
            entry.snapshot.status = if success {
                TransferStatus::Completed
            } else {
                TransferStatus::Failed
            };
            entry.snapshot.finished_at = Some(now_millis());

            guard.total_bytes_served = guard
                .total_bytes_served
                .saturating_add(entry.snapshot.transferred_bytes);
            guard.recent.insert(0, entry.snapshot.clone());
            guard.recent.truncate(MAX_RECENT_TRANSFERS);
            entry.snapshot
        };

        let name = if success {
            names::TRANSFER_COMPLETED
        } else {
            names::TRANSFER_FAILED
        };
        self.events.publish(AppEvent::new(name, &finished));
    }

    pub fn snapshot(&self) -> ActivitySnapshot {
        let Ok(guard) = self.inner.read() else {
            return ActivitySnapshot::default();
        };
        let mut active: Vec<TransferSnapshot> = guard
            .active
            .values()
            .map(|entry| entry.snapshot.clone())
            .collect();
        active.sort_by_key(|transfer| std::cmp::Reverse(transfer.started_at));

        ActivitySnapshot {
            active,
            recent: guard.recent.clone(),
            clients: sorted_clients(&guard.clients),
            total_bytes_served: guard.total_bytes_served,
        }
    }

    pub fn active_count(&self) -> usize {
        self.inner.read().map(|g| g.active.len()).unwrap_or(0)
    }

    /// Forget everything. Called when a sharing session ends, so activity
    /// from a previous session is not attributed to the new one.
    pub fn reset(&self) {
        if let Ok(mut guard) = self.inner.write() {
            guard.active.clear();
            guard.recent.clear();
            guard.clients.clear();
            guard.total_bytes_served = 0;
        }
        self.publish_clients();
    }

    fn publish_clients(&self) {
        let clients = self
            .inner
            .read()
            .map(|guard| sorted_clients(&guard.clients))
            .unwrap_or_default();
        self.events
            .publish(AppEvent::new(names::CLIENTS_CHANGED, &clients));
    }
}

/// Reports bytes as they leave and guarantees the record is closed.
///
/// If the response body is dropped because the phone walked out of Wi-Fi
/// range, `Drop` marks the transfer failed rather than leaving it "active"
/// forever.
pub struct TransferGuard {
    tracker: Arc<TransferTracker>,
    id: String,
    finished: bool,
}

impl TransferGuard {
    pub fn advance(&self, delta: u64) {
        self.tracker.advance(&self.id, delta);
    }

    pub fn complete(mut self) {
        self.finished = true;
        self.tracker.finish(&self.id, true);
    }

    pub fn fail(mut self) {
        self.finished = true;
        self.tracker.finish(&self.id, false);
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Drop for TransferGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.tracker.finish(&self.id, false);
        }
    }
}

fn sorted_clients(clients: &HashMap<String, ClientSnapshot>) -> Vec<ClientSnapshot> {
    let mut list: Vec<ClientSnapshot> = clients.values().cloned().collect();
    list.sort_by_key(|client| std::cmp::Reverse(client.last_seen));
    list
}

/// Insert or refresh a client record. Returns true when the list changed in a
/// way worth telling the UI about.
fn touch_client(
    clients: &mut HashMap<String, ClientSnapshot>,
    ip: &str,
    user_agent: Option<&str>,
) -> bool {
    let now = now_millis();
    if let Some(existing) = clients.get_mut(ip) {
        existing.requests += 1;
        existing.last_seen = now;
        if let Some(agent) = user_agent {
            if existing.user_agent.as_deref() != Some(agent) {
                existing.device = device_hint(agent).to_string();
                existing.browser = browser_hint(agent).to_string();
                existing.user_agent = Some(agent.to_string());
            }
        }
        return false;
    }

    if clients.len() >= MAX_CLIENTS {
        // Evict the least recently seen so a busy LAN cannot grow this list.
        if let Some(oldest) = clients
            .values()
            .min_by_key(|client| client.last_seen)
            .map(|client| client.ip.clone())
        {
            clients.remove(&oldest);
        }
    }

    clients.insert(
        ip.to_string(),
        ClientSnapshot {
            ip: ip.to_string(),
            user_agent: user_agent.map(str::to_string),
            device: device_hint(user_agent.unwrap_or_default()).to_string(),
            browser: browser_hint(user_agent.unwrap_or_default()).to_string(),
            requests: 1,
            first_seen: now,
            last_seen: now,
        },
    );
    true
}

/// Deliberately coarse. The point is "which of my devices is that?", not
/// fingerprinting, and a wrong guess costs nothing.
pub fn device_hint(user_agent: &str) -> &'static str {
    let ua = user_agent.to_ascii_lowercase();
    if ua.contains("iphone") {
        "iPhone"
    } else if ua.contains("ipad") {
        "iPad"
    } else if ua.contains("android") {
        "Android"
    } else if ua.contains("windows") {
        "Windows"
    } else if ua.contains("mac os x") || ua.contains("macintosh") {
        "Mac"
    } else if ua.contains("cros") {
        "ChromeOS"
    } else if ua.contains("linux") {
        "Linux"
    } else if ua.is_empty() {
        "Unknown device"
    } else {
        "Other"
    }
}

pub fn browser_hint(user_agent: &str) -> &'static str {
    let ua = user_agent.to_ascii_lowercase();
    // Order matters: Edge and Chrome both claim Safari, Chrome claims Safari.
    if ua.contains("edg/") || ua.contains("edga/") || ua.contains("edgios/") {
        "Edge"
    } else if ua.contains("opr/") || ua.contains("opera") {
        "Opera"
    } else if ua.contains("firefox") || ua.contains("fxios") {
        "Firefox"
    } else if ua.contains("crios") || ua.contains("chrome") || ua.contains("chromium") {
        "Chrome"
    } else if ua.contains("safari") {
        "Safari"
    } else if ua.contains("curl") {
        "curl"
    } else if ua.contains("wget") {
        "wget"
    } else if ua.is_empty() {
        "Unknown"
    } else {
        "Other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1";
    const WIN_CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
    const WIN_EDGE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.0.0";
    const ANDROID_FIREFOX: &str =
        "Mozilla/5.0 (Android 14; Mobile; rv:127.0) Gecko/127.0 Firefox/127.0";

    fn tracker() -> (Arc<TransferTracker>, EventBus) {
        let bus = EventBus::default();
        (Arc::new(TransferTracker::new(bus.clone())), bus)
    }

    fn start(file_name: &str, total: u64, ip: &str, ua: Option<&str>) -> TransferStart {
        TransferStart {
            file_id: format!("id-{file_name}"),
            file_name: file_name.to_string(),
            total_bytes: total,
            file_bytes: total,
            is_range_request: false,
            client_ip: ip.to_string(),
            user_agent: ua.map(str::to_string),
        }
    }

    #[test]
    fn a_transfer_moves_from_active_to_recent() {
        let (tracker, _bus) = tracker();
        let guard = tracker.begin(start("demo.mp4", 1000, "192.168.1.51", Some(IPHONE)));

        assert_eq!(tracker.active_count(), 1);
        guard.advance(400);
        assert_eq!(tracker.snapshot().active[0].transferred_bytes, 400);
        assert_eq!(tracker.snapshot().active[0].percent(), 40);

        guard.advance(600);
        guard.complete();

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.active.len(), 0);
        assert_eq!(snapshot.recent.len(), 1);
        assert_eq!(snapshot.recent[0].status, TransferStatus::Completed);
        assert_eq!(snapshot.recent[0].transferred_bytes, 1000);
        assert_eq!(snapshot.total_bytes_served, 1000);
    }

    #[test]
    fn dropping_a_guard_marks_the_transfer_failed() {
        let (tracker, _bus) = tracker();
        {
            let guard = tracker.begin(start("big.iso", 5_000_000_000, "192.168.1.74", None));
            guard.advance(1_000_000);
            // The client disconnects: the guard is dropped without completing.
        }
        let snapshot = tracker.snapshot();
        assert_eq!(
            snapshot.active.len(),
            0,
            "a dropped transfer must not stay active"
        );
        assert_eq!(snapshot.recent[0].status, TransferStatus::Failed);
        assert_eq!(snapshot.recent[0].transferred_bytes, 1_000_000);
    }

    #[test]
    fn explicit_failure_is_recorded() {
        let (tracker, _bus) = tracker();
        let guard = tracker.begin(start("a.bin", 10, "10.0.0.2", None));
        guard.fail();
        assert_eq!(tracker.snapshot().recent[0].status, TransferStatus::Failed);
    }

    #[tokio::test]
    async fn started_and_completed_events_are_published() {
        let (tracker, bus) = tracker();
        let mut receiver = bus.subscribe();

        let guard = tracker.begin(start("report.pdf", 100, "192.168.1.9", Some(WIN_CHROME)));
        guard.complete();

        let mut seen = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            seen.push(event.name);
        }
        assert!(seen.contains(&names::TRANSFER_STARTED));
        assert!(seen.contains(&names::TRANSFER_COMPLETED));
        assert!(seen.contains(&names::CLIENTS_CHANGED));
    }

    #[tokio::test]
    async fn progress_events_are_throttled() {
        let (tracker, bus) = tracker();
        let mut receiver = bus.subscribe();
        let guard = tracker.begin(start("stream.bin", 1_000_000, "10.0.0.3", None));

        // A thousand small writes in a tight loop must not become a thousand
        // events; only the byte threshold or the timer may release one.
        for _ in 0..1000 {
            guard.advance(1024);
        }
        drop(guard);

        let progress = std::iter::from_fn(|| receiver.try_recv().ok())
            .filter(|event| event.name == names::TRANSFER_PROGRESS)
            .count();
        assert!(
            progress <= 2,
            "expected throttled progress, saw {progress} events"
        );
        assert_eq!(tracker.snapshot().recent[0].transferred_bytes, 1_024_000);
    }

    #[test]
    fn zero_byte_advances_are_ignored() {
        let (tracker, _bus) = tracker();
        let guard = tracker.begin(start("empty.txt", 0, "10.0.0.4", None));
        guard.advance(0);
        assert_eq!(tracker.snapshot().active[0].transferred_bytes, 0);
        assert_eq!(
            tracker.snapshot().active[0].percent(),
            100,
            "an empty file is complete"
        );
    }

    #[test]
    fn concurrent_transfers_are_tracked_independently() {
        let (tracker, _bus) = tracker();
        let a = tracker.begin(start("a.bin", 100, "192.168.1.10", None));
        let b = tracker.begin(start("b.bin", 200, "192.168.1.11", None));

        a.advance(50);
        b.advance(200);
        assert_eq!(tracker.active_count(), 2);

        b.complete();
        assert_eq!(tracker.active_count(), 1);
        a.complete();

        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.recent.len(), 2);
        assert_eq!(snapshot.total_bytes_served, 250);
        assert_eq!(snapshot.clients.len(), 2);
    }

    #[test]
    fn recent_transfers_are_capped() {
        let (tracker, _bus) = tracker();
        for index in 0..(MAX_RECENT_TRANSFERS + 10) {
            tracker
                .begin(start(&format!("f{index}.bin"), 1, "10.0.0.5", None))
                .complete();
        }
        let snapshot = tracker.snapshot();
        assert_eq!(snapshot.recent.len(), MAX_RECENT_TRANSFERS);
        // Newest first.
        assert_eq!(
            snapshot.recent[0].file_name,
            format!("f{}.bin", MAX_RECENT_TRANSFERS + 9)
        );
    }

    #[test]
    fn clients_are_deduplicated_by_ip_and_counted() {
        let (tracker, _bus) = tracker();
        tracker.note_request("192.168.1.51", Some(IPHONE));
        tracker.note_request("192.168.1.51", Some(IPHONE));
        tracker.note_request("192.168.1.74", Some(WIN_EDGE));

        let clients = tracker.snapshot().clients;
        assert_eq!(clients.len(), 2);
        let iphone = clients
            .iter()
            .find(|c| c.ip == "192.168.1.51")
            .expect("iphone");
        assert_eq!(iphone.requests, 2);
        assert_eq!(iphone.device, "iPhone");
        assert_eq!(iphone.browser, "Safari");
    }

    #[test]
    fn the_client_list_is_capped() {
        let (tracker, _bus) = tracker();
        for index in 0..(MAX_CLIENTS + 5) {
            tracker.note_request(&format!("192.168.1.{index}"), None);
        }
        assert!(tracker.snapshot().clients.len() <= MAX_CLIENTS);
    }

    #[test]
    fn reset_clears_everything() {
        let (tracker, _bus) = tracker();
        tracker
            .begin(start("a.bin", 5, "10.0.0.6", None))
            .complete();
        tracker.note_request("10.0.0.7", None);
        tracker.reset();

        let snapshot = tracker.snapshot();
        assert!(snapshot.active.is_empty());
        assert!(snapshot.recent.is_empty());
        assert!(snapshot.clients.is_empty());
        assert_eq!(snapshot.total_bytes_served, 0);
    }

    #[test]
    fn device_and_browser_hints_cover_the_common_cases() {
        assert_eq!(device_hint(IPHONE), "iPhone");
        assert_eq!(browser_hint(IPHONE), "Safari");
        assert_eq!(device_hint(WIN_CHROME), "Windows");
        assert_eq!(browser_hint(WIN_CHROME), "Chrome");
        assert_eq!(
            browser_hint(WIN_EDGE),
            "Edge",
            "Edge also claims Chrome and Safari"
        );
        assert_eq!(device_hint(ANDROID_FIREFOX), "Android");
        assert_eq!(browser_hint(ANDROID_FIREFOX), "Firefox");
        assert_eq!(device_hint(""), "Unknown device");
        assert_eq!(browser_hint("curl/8.4.0"), "curl");
    }

    #[test]
    fn percent_is_clamped_and_safe_for_ranges() {
        let mut snapshot = TransferSnapshot {
            id: "x".into(),
            file_id: "y".into(),
            file_name: "clip.mp4".into(),
            total_bytes: 100,
            file_bytes: 1000,
            transferred_bytes: 250,
            is_range_request: true,
            client_ip: "10.0.0.1".into(),
            user_agent: None,
            status: TransferStatus::Active,
            started_at: 0,
            finished_at: None,
        };
        assert_eq!(snapshot.percent(), 100, "never above 100");
        snapshot.transferred_bytes = 25;
        assert_eq!(
            snapshot.percent(),
            25,
            "progress is against the range, not the file"
        );
    }
}
