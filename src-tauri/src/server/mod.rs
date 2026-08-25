//! The embedded HTTP server: lifecycle, binding and shared context.
//!
//! The listener binds `0.0.0.0` so a DHCP lease change or a Wi-Fi reconnect
//! does not require a rebind — only the address shown to the user changes.
//! The address the UI displays always comes from the selected interface, never
//! from the bind address.

pub mod files;
pub mod middleware;
pub mod page;
pub mod range;
pub mod routes;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};
use crate::events::EventBus;
use crate::sharing::{ShareRegistry, ShareSession};
use crate::transfer::TransferTracker;

/// How long a graceful shutdown may take before in-flight connections are cut.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);

/// Everything a request handler is allowed to touch. Cheap to clone.
#[derive(Clone)]
pub struct ServerContext {
    pub registry: Arc<ShareRegistry>,
    /// Swapped wholesale when the user regenerates the share link.
    pub session: Arc<RwLock<ShareSession>>,
    pub tracker: Arc<TransferTracker>,
    pub events: EventBus,
    pub device_name: Arc<str>,
    /// Cancelled when sharing stops. Handlers and in-flight streams watch it.
    pub shutdown: CancellationToken,
}

impl ServerContext {
    pub fn publish_files_changed(&self) {
        self.events
            .publish(crate::sharing::files_changed_event(&self.registry));
    }
}

/// A running server. Dropping this does *not* stop the server; call
/// [`ServerHandle::shutdown`] so shutdown stays explicit and awaited.
pub struct ServerHandle {
    pub port: u16,
    pub bind_addr: SocketAddr,
    cancel: CancellationToken,
    join: JoinHandle<()>,
}

impl ServerHandle {
    /// Stop accepting connections and tear down anything still streaming.
    ///
    /// Cancelling the token does double duty: axum stops accepting, and every
    /// active download stream sees it on its next chunk and ends. Without
    /// that second half, "stop sharing" would leave a 10 GB transfer running.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        match tokio::time::timeout(SHUTDOWN_GRACE, self.join).await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => tracing::warn!(target: "droplan", "server task ended badly: {err}"),
            Err(_) => tracing::warn!(
                target: "droplan",
                "server did not shut down within {SHUTDOWN_GRACE:?}; connections were dropped"
            ),
        }
    }
}

/// Bind the first available port, then serve until the context is cancelled.
pub async fn start(
    ctx: ServerContext,
    preferred_port: u16,
    scan_range: u16,
) -> Result<ServerHandle> {
    let listener = bind_listener(preferred_port, scan_range).await?;
    let bind_addr = listener
        .local_addr()
        .map_err(|err| Error::ServerStart(err.to_string()))?;

    let cancel = ctx.shutdown.clone();
    let router = routes::build(ctx);
    let shutdown_signal = cancel.clone();

    let join = tokio::spawn(async move {
        // `into_make_service_with_connect_info` is what makes the peer address
        // available to handlers, which is how the activity list knows who is
        // downloading.
        let service = router.into_make_service_with_connect_info::<SocketAddr>();
        if let Err(err) = axum::serve(listener, service)
            .with_graceful_shutdown(async move { shutdown_signal.cancelled().await })
            .await
        {
            tracing::error!(target: "droplan", "http server stopped: {err}");
        }
    });

    tracing::info!(target: "droplan", "listening on {bind_addr}");
    Ok(ServerHandle {
        port: bind_addr.port(),
        bind_addr,
        cancel,
        join,
    })
}

/// Try the preferred port, then the next `scan_range` ports, then let the OS
/// choose. Only a total failure is an error the user has to read about.
async fn bind_listener(preferred_port: u16, scan_range: u16) -> Result<TcpListener> {
    let mut last_tried = preferred_port;

    for offset in 0..scan_range {
        let Some(port) = preferred_port.checked_add(offset) else {
            break;
        };
        last_tried = port;
        match TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).await {
            Ok(listener) => return Ok(listener),
            Err(err) if is_address_in_use(&err) => continue,
            Err(err) => {
                // Permission denied and friends are worth surfacing verbatim.
                return Err(Error::ServerStart(format!(
                    "Could not bind port {port}: {err}"
                )));
            }
        }
    }

    // Port 0 asks the OS for any free port. Better an unexpected port number
    // than refusing to share at all.
    match TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await {
        Ok(listener) => {
            tracing::warn!(
                target: "droplan",
                "ports {preferred_port}-{last_tried} were busy; using an OS-assigned port"
            );
            Ok(listener)
        }
        Err(_) => Err(Error::NoAvailablePort {
            preferred: preferred_port,
            last: last_tried,
        }),
    }
}

fn is_address_in_use(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(shutdown: CancellationToken) -> ServerContext {
        let events = EventBus::default();
        ServerContext {
            registry: Arc::new(ShareRegistry::new()),
            session: Arc::new(RwLock::new(ShareSession::new(false).expect("session"))),
            tracker: Arc::new(TransferTracker::new(events.clone())),
            events,
            device_name: Arc::from("Test Machine"),
            shutdown,
        }
    }

    #[tokio::test]
    async fn the_preferred_port_is_used_when_it_is_free() {
        // Learn a free port from the OS, release it, then ask for it by number.
        //
        // There is an unavoidable gap between releasing the probe and binding
        // again in which something else on the machine can take the port, so a
        // single attempt would be flaky. Retrying with a fresh candidate makes
        // the assertion meaningful without depending on that race.
        for attempt in 0..8 {
            let probe = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
                .await
                .expect("probe");
            let candidate = probe.local_addr().expect("addr").port();
            drop(probe);

            let listener = bind_listener(candidate, 1).await.expect("bind");
            if listener.local_addr().expect("addr").port() == candidate {
                return;
            }
            assert!(attempt < 7, "the preferred port was never honoured");
        }
    }

    #[tokio::test]
    async fn a_busy_port_falls_forward_to_the_next_one() {
        let occupied = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .await
            .expect("occupy");
        let busy_port = occupied.local_addr().expect("addr").port();

        let listener = bind_listener(busy_port, 16).await.expect("bind");
        let chosen = listener.local_addr().expect("addr").port();
        assert_ne!(chosen, busy_port);
        assert!(u32::from(chosen) > u32::from(busy_port));
        assert!(u32::from(chosen) <= u32::from(busy_port) + 16);
    }

    #[tokio::test]
    async fn binding_always_succeeds_via_an_os_assigned_port() {
        let occupied = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            .await
            .expect("occupy");
        let busy_port = occupied.local_addr().expect("addr").port();

        // A scan range of one leaves no alternative but the OS fallback.
        let listener = bind_listener(busy_port, 1).await.expect("bind");
        assert_ne!(listener.local_addr().expect("addr").port(), busy_port);
    }

    #[tokio::test]
    async fn a_scan_that_would_overflow_the_port_space_is_safe() {
        let listener = bind_listener(u16::MAX, 32).await.expect("bind");
        assert!(listener.local_addr().is_ok());
    }

    #[tokio::test]
    async fn the_server_starts_and_stops_cleanly() {
        let cancel = CancellationToken::new();
        let handle = start(context(cancel.clone()), 0, 1).await.expect("start");
        let port = handle.port;
        assert!(port > 0);

        let url = format!("http://127.0.0.1:{port}/health");
        let response = reqwest::get(&url).await.expect("health request");
        assert_eq!(response.status(), 200);

        handle.shutdown().await;

        // The listener is gone, so a fresh connection must fail.
        let result = reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await;
        assert!(result.is_err(), "the port must be closed after shutdown");
    }

    #[tokio::test]
    async fn the_server_can_be_restarted_on_the_same_port() {
        let first = start(context(CancellationToken::new()), 0, 1)
            .await
            .expect("start");
        let port = first.port;
        first.shutdown().await;

        let second = start(context(CancellationToken::new()), port, 8)
            .await
            .expect("restart");
        assert_eq!(second.port, port, "the freed port should be reusable");
        second.shutdown().await;
    }
}
