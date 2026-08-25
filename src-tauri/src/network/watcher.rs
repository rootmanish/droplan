//! Watches for the network moving under our feet.
//!
//! Wi-Fi drops, the user walks to another router, a cable goes in, DHCP hands
//! out a new lease, the laptop wakes from sleep. All of these change the
//! address other devices must use, so the UI and the QR code have to follow.
//!
//! The listener itself binds `0.0.0.0`, so none of this requires a rebind.
//! What changes is the address we *show*.

use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::interfaces::{self, NetworkSnapshot};

/// Poll interval. Low enough to feel immediate, high enough to be invisible
/// in a CPU graph: one `getifaddrs` plus one routing-table lookup.
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(4);

/// If a tick arrives this much later than scheduled, the machine was asleep.
pub const RESUME_SLACK: Duration = Duration::from_secs(20);

#[derive(Debug)]
pub enum NetworkEvent {
    /// The set of interfaces or the default route changed.
    Changed(Box<NetworkSnapshot>),
    /// The process was suspended and has just resumed.
    Resumed(Box<NetworkSnapshot>),
}

/// Remembers the last shape of the network so only real changes are reported.
#[derive(Default)]
pub struct ChangeDetector {
    last_fingerprint: Option<String>,
}

impl ChangeDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when this snapshot differs from the previous one. The very first
    /// observation counts as a change so the UI gets an initial value.
    pub fn observe(&mut self, snapshot: &NetworkSnapshot) -> bool {
        let fingerprint = snapshot.fingerprint();
        let changed = self.last_fingerprint.as_deref() != Some(fingerprint.as_str());
        self.last_fingerprint = Some(fingerprint);
        changed
    }

    /// Seed without reporting a change, for a snapshot taken at startup.
    pub fn prime(&mut self, snapshot: &NetworkSnapshot) {
        self.last_fingerprint = Some(snapshot.fingerprint());
    }
}

pub struct NetworkWatcher {
    cancel: CancellationToken,
    handle: JoinHandle<()>,
}

impl NetworkWatcher {
    pub async fn stop(self) {
        self.cancel.cancel();
        if let Err(err) = self.handle.await {
            tracing::debug!(target: "droplan", "network watcher ended: {err}");
        }
    }
}

/// Start polling. `preferred` is read fresh each tick so a change of the
/// pinned interface takes effect without restarting the watcher.
pub fn spawn<P>(
    interval: Duration,
    preferred: P,
    sender: UnboundedSender<NetworkEvent>,
    initial: Option<&NetworkSnapshot>,
) -> NetworkWatcher
where
    P: Fn() -> Option<String> + Send + Sync + 'static,
{
    let cancel = CancellationToken::new();
    let task_cancel = cancel.clone();

    let mut detector = ChangeDetector::new();
    if let Some(snapshot) = initial {
        detector.prime(snapshot);
    }

    let handle = tokio::spawn(async move {
        let mut last_tick = Instant::now();

        loop {
            tokio::select! {
                // Cancellation wins, so shutdown never waits out an interval.
                biased;
                _ = task_cancel.cancelled() => break,
                _ = tokio::time::sleep(interval) => {}
            }

            let elapsed = last_tick.elapsed();
            last_tick = Instant::now();
            // A monotonic clock does not advance across suspend on every OS,
            // but where it does, a tick far later than scheduled is the
            // clearest available signal that we were asleep.
            let resumed = elapsed > interval + RESUME_SLACK;

            // `detect` does blocking syscalls; keep them off the async worker.
            let preferred_name = preferred();
            let detected =
                tokio::task::spawn_blocking(move || interfaces::detect(preferred_name.as_deref()))
                    .await;

            let snapshot = match detected {
                Ok(Ok(snapshot)) => snapshot,
                Ok(Err(err)) => {
                    tracing::warn!(target: "droplan", "network detection failed: {err}");
                    continue;
                }
                Err(err) => {
                    tracing::warn!(target: "droplan", "network detection task failed: {err}");
                    continue;
                }
            };

            let changed = detector.observe(&snapshot);
            let event = if resumed {
                tracing::info!(target: "droplan", "resumed after {elapsed:?}; re-checking the network");
                Some(NetworkEvent::Resumed(Box::new(snapshot)))
            } else if changed {
                tracing::info!(target: "droplan", "network changed: {}", snapshot.fingerprint());
                Some(NetworkEvent::Changed(Box::new(snapshot)))
            } else {
                None
            };

            if let Some(event) = event {
                // A closed receiver means the app is shutting down.
                if sender.send(event).is_err() {
                    break;
                }
            }
        }
    });

    NetworkWatcher { cancel, handle }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::interfaces::{build_interfaces, RawInterface};
    use std::net::Ipv4Addr;

    fn snapshot(name: &str, ip: &str, route: Option<&str>) -> NetworkSnapshot {
        let raw = vec![RawInterface {
            name: name.to_string(),
            ip: ip.parse().expect("ip"),
            netmask: Some(Ipv4Addr::new(255, 255, 255, 0)),
            is_loopback: false,
        }];
        NetworkSnapshot {
            interfaces: build_interfaces(raw, route.map(|r| r.parse().expect("ip"))),
            selected: None,
            default_route: route.map(str::to_string),
            detected_at: 0,
        }
    }

    #[test]
    fn the_first_observation_always_counts_as_a_change() {
        let mut detector = ChangeDetector::new();
        assert!(detector.observe(&snapshot("en0", "192.168.1.42", None)));
    }

    #[test]
    fn an_unchanged_network_is_silent() {
        let mut detector = ChangeDetector::new();
        let current = snapshot("en0", "192.168.1.42", Some("192.168.1.42"));
        assert!(detector.observe(&current));
        assert!(!detector.observe(&current));
        assert!(!detector.observe(&current));
    }

    #[test]
    fn a_dhcp_address_change_is_reported() {
        let mut detector = ChangeDetector::new();
        detector.prime(&snapshot("en0", "192.168.1.42", Some("192.168.1.42")));
        assert!(detector.observe(&snapshot("en0", "192.168.0.18", Some("192.168.0.18"))));
    }

    #[test]
    fn losing_and_regaining_the_network_are_both_reported() {
        let mut detector = ChangeDetector::new();
        detector.prime(&snapshot("en0", "192.168.1.42", Some("192.168.1.42")));

        let disconnected = NetworkSnapshot::default();
        assert!(detector.observe(&disconnected), "losing Wi-Fi is a change");
        assert!(
            !detector.observe(&disconnected),
            "still disconnected is not"
        );
        assert!(detector.observe(&snapshot("en0", "192.168.1.42", Some("192.168.1.42"))));
    }

    #[test]
    fn a_new_interface_appearing_is_reported() {
        let mut detector = ChangeDetector::new();
        detector.prime(&snapshot("en0", "192.168.1.42", Some("192.168.1.42")));

        let mut with_ethernet = snapshot("en0", "192.168.1.42", Some("192.168.1.42"));
        with_ethernet
            .interfaces
            .extend(snapshot("en5", "10.0.0.15", None).interfaces);
        assert!(detector.observe(&with_ethernet));
    }

    #[test]
    fn priming_suppresses_the_initial_change() {
        let mut detector = ChangeDetector::new();
        let current = snapshot("en0", "192.168.1.42", None);
        detector.prime(&current);
        assert!(!detector.observe(&current));
    }

    #[tokio::test]
    async fn the_watcher_reports_the_real_network_and_stops_promptly() {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let watcher = spawn(Duration::from_millis(40), || None, sender, None);

        let event = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .expect("a first observation should arrive")
            .expect("channel open");
        match event {
            NetworkEvent::Changed(snapshot) | NetworkEvent::Resumed(snapshot) => {
                assert!(!snapshot.fingerprint().is_empty());
            }
        }

        let stopped = tokio::time::timeout(Duration::from_secs(2), watcher.stop()).await;
        assert!(stopped.is_ok(), "stop must not wait out the poll interval");
    }

    #[tokio::test]
    async fn a_closed_receiver_ends_the_watcher() {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let watcher = spawn(Duration::from_millis(20), || None, sender, None);
        drop(receiver);
        assert!(tokio::time::timeout(Duration::from_secs(3), watcher.stop())
            .await
            .is_ok());
    }
}
