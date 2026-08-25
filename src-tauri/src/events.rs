//! The one-way channel from the Rust core to the desktop UI.
//!
//! Core modules publish onto a broadcast bus and never touch Tauri. A single
//! bridge task in [`crate::app`] forwards everything to the webview. That
//! keeps the network, server and transfer code free of UI dependencies and
//! testable without a running Tauri app.

use serde::Serialize;
use tokio::sync::broadcast;

/// Event names shared with the frontend. Keep in sync with `src/types/index.ts`.
pub mod names {
    pub const NETWORK_CHANGED: &str = "network-changed";
    pub const SHARING_STARTED: &str = "sharing-started";
    pub const SHARING_STOPPED: &str = "sharing-stopped";
    pub const SHARED_FILES_CHANGED: &str = "shared-files-changed";
    pub const TRANSFER_STARTED: &str = "transfer-started";
    pub const TRANSFER_PROGRESS: &str = "transfer-progress";
    pub const TRANSFER_COMPLETED: &str = "transfer-completed";
    pub const TRANSFER_FAILED: &str = "transfer-failed";
    pub const CLIENTS_CHANGED: &str = "clients-changed";
    pub const SYSTEM_RESUMED: &str = "system-resumed";
    pub const NOTICE: &str = "notice";
}

/// A named message plus its already-serialised payload.
#[derive(Debug, Clone)]
pub struct AppEvent {
    pub name: &'static str,
    pub payload: serde_json::Value,
}

impl AppEvent {
    pub fn new<T: Serialize>(name: &'static str, payload: &T) -> Self {
        let payload = serde_json::to_value(payload).unwrap_or_else(|err| {
            tracing::error!(target: "droplan", "event {name} payload could not be serialised: {err}");
            serde_json::Value::Null
        });
        AppEvent { name, payload }
    }

    pub fn bare(name: &'static str) -> Self {
        AppEvent {
            name,
            payload: serde_json::Value::Null,
        }
    }

    /// A user-visible, non-fatal message (a file vanished, mDNS failed, …).
    pub fn notice(code: &str, message: impl Into<String>) -> Self {
        AppEvent::new(
            names::NOTICE,
            &serde_json::json!({ "code": code, "message": message.into() }),
        )
    }
}

/// Fan-out channel. Cloning is cheap; every clone publishes to the same bus.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        EventBus { sender }
    }

    /// Publish. With no subscribers this is a no-op rather than an error:
    /// the core keeps running whether or not a window is listening.
    pub fn publish(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        // Generous enough that a burst of transfer-progress events during a
        // UI stall does not drop the terminal completed/failed event.
        EventBus::new(256)
    }
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("subscribers", &self.subscriber_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscribers_receive_published_events() {
        let bus = EventBus::default();
        let mut receiver = bus.subscribe();

        bus.publish(AppEvent::new(
            names::SHARING_STARTED,
            &serde_json::json!({ "port": 8080 }),
        ));

        let event = receiver.recv().await.expect("event");
        assert_eq!(event.name, "sharing-started");
        assert_eq!(event.payload["port"], 8080);
    }

    #[tokio::test]
    async fn publishing_without_subscribers_is_harmless() {
        let bus = EventBus::default();
        assert_eq!(bus.subscriber_count(), 0);
        bus.publish(AppEvent::bare(names::SHARING_STOPPED));
    }

    #[tokio::test]
    async fn every_clone_shares_one_bus() {
        let bus = EventBus::default();
        let mut receiver = bus.subscribe();
        let clone = bus.clone();
        clone.publish(AppEvent::bare(names::SYSTEM_RESUMED));
        assert_eq!(receiver.recv().await.expect("event").name, "system-resumed");
    }

    #[test]
    fn notices_carry_a_code_and_a_message() {
        let event = AppEvent::notice("file_unavailable", "demo.mp4 is no longer on disk");
        assert_eq!(event.name, names::NOTICE);
        assert_eq!(event.payload["code"], "file_unavailable");
        assert_eq!(event.payload["message"], "demo.mp4 is no longer on disk");
    }
}
