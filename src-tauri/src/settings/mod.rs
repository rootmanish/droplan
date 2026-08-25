//! Persisted configuration.
//!
//! Only preferences belong here. The file list and the session token are
//! deliberately *not* persisted: relaunching the app must start a new session
//! and share nothing until the user says so.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const DEFAULT_PORT: u16 = 8080;

/// How many consecutive ports to try before asking the OS for any free one.
pub const DEFAULT_PORT_SCAN_RANGE: u16 = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    /// First port we try to bind. Falls forward if it is taken.
    pub preferred_port: u16,
    /// Interface name the user pinned, if any. `None` means "choose for me".
    pub preferred_interface: Option<String>,
    pub start_sharing_on_launch: bool,
    /// Advertise `_http._tcp` so `droplan-xxxx.local` resolves on the LAN.
    pub enable_mdns: bool,
    /// Put a 6-digit PIN in front of the share page.
    pub require_pin: bool,
    /// Keep serving after the window is closed, with the tray icon as the
    /// visible reminder. Off by default: no hidden servers.
    pub close_to_tray: bool,
    pub port_scan_range: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            preferred_port: DEFAULT_PORT,
            preferred_interface: None,
            start_sharing_on_launch: true,
            enable_mdns: true,
            require_pin: false,
            close_to_tray: false,
            port_scan_range: DEFAULT_PORT_SCAN_RANGE,
        }
    }
}

impl AppSettings {
    /// Clamp anything that arrived from disk or the UI into a usable range.
    pub fn normalized(mut self) -> Self {
        // Ports below 1024 need elevation on Unix and are reserved anyway.
        if self.preferred_port < 1024 {
            self.preferred_port = DEFAULT_PORT;
        }
        self.port_scan_range = self.port_scan_range.clamp(1, 512);
        if let Some(name) = &self.preferred_interface {
            if name.trim().is_empty() {
                self.preferred_interface = None;
            }
        }
        self
    }
}

/// Thread-safe settings holder that writes through to disk.
pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<AppSettings>,
}

impl SettingsStore {
    /// Load from `dir/settings.json`, falling back to defaults for a missing
    /// or corrupt file. A bad file is never fatal, and never silently kept:
    /// the next successful save overwrites it.
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("settings.json");
        let settings = match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<AppSettings>(&raw) {
                Ok(parsed) => parsed.normalized(),
                Err(err) => {
                    tracing::warn!(target: "droplan", "settings.json is not readable ({err}); using defaults");
                    AppSettings::default()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
            Err(err) => {
                tracing::warn!(target: "droplan", "could not read settings.json ({err}); using defaults");
                AppSettings::default()
            }
        };

        SettingsStore {
            path,
            current: RwLock::new(settings),
        }
    }

    pub fn get(&self) -> AppSettings {
        self.current
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Apply a change and persist it.
    pub fn update<F>(&self, mutate: F) -> Result<AppSettings>
    where
        F: FnOnce(&mut AppSettings),
    {
        let updated = {
            let mut guard = self
                .current
                .write()
                .map_err(|_| Error::Settings("the settings lock was poisoned".into()))?;
            mutate(&mut guard);
            *guard = guard.clone().normalized();
            guard.clone()
        };
        self.persist(&updated)?;
        Ok(updated)
    }

    pub fn replace(&self, settings: AppSettings) -> Result<AppSettings> {
        self.update(|current| *current = settings.clone())
    }

    /// Write atomically: a crash mid-write must not leave a truncated file
    /// that fails to parse on next launch.
    fn persist(&self, settings: &AppSettings) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                Error::Settings(format!("could not create {}: {err}", parent.display()))
            })?;
        }
        let serialized = serde_json::to_string_pretty(settings)
            .map_err(|err| Error::Settings(err.to_string()))?;

        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, serialized.as_bytes())
            .map_err(|err| Error::Settings(format!("could not write settings: {err}")))?;
        std::fs::rename(&temp, &self.path)
            .map_err(|err| Error::Settings(format!("could not save settings: {err}")))?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let settings = AppSettings::default();
        assert_eq!(settings.preferred_port, 8080);
        assert!(settings.preferred_interface.is_none());
        assert!(settings.start_sharing_on_launch);
        assert!(!settings.require_pin);
        assert!(!settings.close_to_tray, "no hidden server by default");
    }

    #[test]
    fn normalization_rejects_privileged_and_absurd_values() {
        let settings = AppSettings {
            preferred_port: 80,
            port_scan_range: 9999,
            preferred_interface: Some("   ".into()),
            ..AppSettings::default()
        }
        .normalized();

        assert_eq!(settings.preferred_port, DEFAULT_PORT);
        assert_eq!(settings.port_scan_range, 512);
        assert!(settings.preferred_interface.is_none());
    }

    #[test]
    fn a_missing_file_yields_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SettingsStore::load(dir.path());
        assert_eq!(store.get(), AppSettings::default());
    }

    #[test]
    fn a_corrupt_file_yields_defaults_instead_of_failing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("settings.json"), b"{ not json").expect("write");
        let store = SettingsStore::load(dir.path());
        assert_eq!(store.get(), AppSettings::default());
    }

    #[test]
    fn updates_round_trip_through_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SettingsStore::load(dir.path());
        store
            .update(|settings| {
                settings.preferred_port = 9000;
                settings.preferred_interface = Some("en0".into());
                settings.require_pin = true;
            })
            .expect("update");

        let reloaded = SettingsStore::load(dir.path());
        let settings = reloaded.get();
        assert_eq!(settings.preferred_port, 9000);
        assert_eq!(settings.preferred_interface.as_deref(), Some("en0"));
        assert!(settings.require_pin);
    }

    #[test]
    fn unknown_and_missing_fields_do_not_break_loading() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("settings.json"),
            br#"{"preferredPort": 9100, "somethingFromTheFuture": true}"#,
        )
        .expect("write");

        let settings = SettingsStore::load(dir.path()).get();
        assert_eq!(settings.preferred_port, 9100);
        // Everything absent falls back to the default.
        assert!(settings.enable_mdns);
    }

    #[test]
    fn saving_leaves_no_temporary_file_behind() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SettingsStore::load(dir.path());
        store.update(|s| s.preferred_port = 8123).expect("update");

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(leftovers, ["settings.json"]);
    }
}
