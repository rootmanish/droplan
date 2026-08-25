//! What is being shared, and with whom.
//!
//! [`registry`] owns the set of files the user picked. [`session`] owns the
//! secret that makes those files reachable from the LAN. The two are separate
//! on purpose: regenerating the link must not disturb the file list, and
//! editing the file list must not invalidate the link.

pub mod qr;
pub mod registry;
pub mod session;

pub use registry::{AddOutcome, RegistryTotals, ShareItem, ShareRegistry};
pub use session::{SessionInfo, ShareSession};

use serde::Serialize;

use crate::events::{names, AppEvent};

/// The payload every `shared-files-changed` event carries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedFilesPayload {
    pub files: Vec<ShareItem>,
    pub totals: RegistryTotals,
}

impl SharedFilesPayload {
    pub fn of(registry: &ShareRegistry) -> Self {
        SharedFilesPayload {
            files: registry.list().unwrap_or_default(),
            totals: registry.totals().unwrap_or_default(),
        }
    }
}

/// Built in one place so the desktop UI sees the same shape no matter whether
/// the change came from a Tauri command or from the HTTP side noticing that a
/// file vanished.
pub fn files_changed_event(registry: &ShareRegistry) -> AppEvent {
    AppEvent::new(
        names::SHARED_FILES_CHANGED,
        &SharedFilesPayload::of(registry),
    )
}
