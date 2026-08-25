//! The complete HTTP surface. Nothing else is reachable.
//!
//! ```text
//! GET  /                          neutral landing page, reveals nothing
//! GET  /health                    liveness only
//! GET  /s/{token}                 the share page
//! POST /s/{token}/unlock          PIN form submission
//! GET  /s/{token}/api/files       JSON list, used by the page to self-refresh
//! GET  /s/{token}/files/{file_id} download / range request
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, Form, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::sharing::session::UNLOCK_COOKIE;

use super::files;
use super::middleware::{self, html, not_found};
use super::page;
use super::ServerContext;

/// Deliberate cost per wrong PIN, to make guessing over the LAN impractical
/// without keeping any per-client state.
const PIN_FAILURE_DELAY: Duration = Duration::from_millis(500);

pub fn build(ctx: ServerContext) -> Router {
    // Registered flat rather than nested: `nest` rewrites the request path it
    // hands to layers, and the session guard has to see the real one. Both the
    // bare and trailing-slash forms are registered because axum performs no
    // trailing-slash redirection of its own.
    let session_routes = Router::new()
        .route("/s/{token}", get(share_page))
        .route("/s/{token}/", get(share_page))
        .route("/s/{token}/unlock", post(unlock))
        .route("/s/{token}/api/files", get(api_files))
        .route("/s/{token}/files/{file_id}", get(files::download))
        // `route_layer` runs only for paths this router actually matched, so
        // an unrelated 404 never pays for a session lookup.
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            middleware::session_guard,
        ));

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/favicon.ico", get(favicon))
        .merge(session_routes)
        .fallback(fallback)
        // The share URL contains the session token. Without this, following
        // any outbound link would hand the token to another server.
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(ctx)
}

async fn root() -> Response {
    html(page::render_root_page())
}

/// Intentionally uninformative: no hostname, no version, no file counts.
async fn health() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "ok",
    )
        .into_response()
}

/// Browsers request this unprompted; answering keeps the logs readable.
async fn favicon() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

async fn fallback() -> Response {
    not_found()
}

async fn share_page(
    State(ctx): State<ServerContext>,
    Path(token): Path<String>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    ctx.tracker
        .note_request(&peer.ip().to_string(), user_agent(&headers));

    let items = ctx.registry.list().unwrap_or_default();
    let signature = files_signature(&items);
    html(page::render_share_page(
        &ctx.device_name,
        &format!("/s/{token}"),
        &items,
        &signature,
    ))
}

#[derive(Debug, Deserialize)]
pub struct UnlockForm {
    pub pin: String,
}

async fn unlock(
    State(ctx): State<ServerContext>,
    Path(token): Path<String>,
    Form(form): Form<UnlockForm>,
) -> Response {
    let base = format!("/s/{token}");

    let (accepted, secret) = {
        let Ok(session) = ctx.session.read() else {
            return not_found();
        };
        (
            session.accepts_pin(&form.pin),
            session.unlock_secret().to_string(),
        )
    };

    if !accepted {
        tokio::time::sleep(PIN_FAILURE_DELAY).await;
        return (
            StatusCode::UNAUTHORIZED,
            html(page::render_pin_page(&ctx.device_name, &base, true)),
        )
            .into_response();
    }

    // Scoped to this session's path and unreadable from script; it is only a
    // marker that the PIN was entered, and it dies with the session.
    let cookie =
        format!("{UNLOCK_COOKIE}={secret}; Path={base}; HttpOnly; SameSite=Strict; Max-Age=86400");
    let mut response = Redirect::to(&base).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, files::header_value(&cookie));
    response
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiFile {
    id: String,
    name: String,
    size: u64,
    mime_type: String,
    kind: String,
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiFileList {
    device_name: String,
    /// Changes whenever the shared set changes; the page polls this instead
    /// of re-rendering itself.
    signature: String,
    count: usize,
    total_bytes: u64,
    files: Vec<ApiFile>,
}

async fn api_files(
    State(ctx): State<ServerContext>,
    Path(token): Path<String>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    ctx.tracker
        .note_request(&peer.ip().to_string(), user_agent(&headers));

    let items = ctx.registry.list().unwrap_or_default();
    let signature = files_signature(&items);
    let base = format!("/s/{token}");

    let files: Vec<ApiFile> = items
        .iter()
        .filter(|item| item.available)
        .map(|item| ApiFile {
            id: item.id.clone(),
            name: item.display_name.clone(),
            size: item.size,
            mime_type: item.mime_type.clone(),
            kind: page::type_label(&item.mime_type, &item.display_name),
            url: format!("{base}/files/{}", item.id),
        })
        .collect();

    let payload = ApiFileList {
        device_name: ctx.device_name.to_string(),
        signature,
        count: files.len(),
        total_bytes: files.iter().map(|file| file.size).sum(),
        files,
    };

    ([(header::CACHE_CONTROL, "no-store")], Json(payload)).into_response()
}

fn user_agent(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
}

/// Cheap FNV-1a over the visible file set. Only ever compared for equality,
/// so a non-cryptographic hash is the right tool.
pub fn files_signature(items: &[crate::sharing::ShareItem]) -> String {
    fn mix(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash ^= *byte as u64;
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for item in items.iter().filter(|item| item.available) {
        mix(&mut hash, item.id.as_bytes());
        mix(&mut hash, &item.size.to_le_bytes());
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharing::ShareItem;
    use std::path::PathBuf;

    fn item(id: &str, size: u64, available: bool) -> ShareItem {
        ShareItem {
            id: id.to_string(),
            display_name: format!("{id}.bin"),
            absolute_path: PathBuf::from("/tmp/x"),
            mime_type: "application/octet-stream".into(),
            size,
            added_at: 0,
            available,
        }
    }

    #[test]
    fn the_signature_tracks_the_visible_file_set() {
        let base = vec![item("a", 10, true), item("b", 20, true)];
        assert_eq!(files_signature(&base), files_signature(&base.clone()));

        // Adding, removing, resizing or reordering all change it.
        assert_ne!(
            files_signature(&base),
            files_signature(&[item("a", 10, true)])
        );
        assert_ne!(
            files_signature(&base),
            files_signature(&[item("a", 11, true), item("b", 20, true)])
        );
        assert_ne!(
            files_signature(&base),
            files_signature(&[item("b", 20, true), item("a", 10, true)])
        );
    }

    #[test]
    fn unavailable_files_do_not_affect_the_signature() {
        let with_dead = vec![item("a", 10, true), item("dead", 999, false)];
        assert_eq!(
            files_signature(&with_dead),
            files_signature(&[item("a", 10, true)])
        );
    }

    #[test]
    fn an_empty_registry_has_a_stable_signature() {
        assert_eq!(files_signature(&[]), files_signature(&[]));
        assert_eq!(files_signature(&[]).len(), 16);
    }
}
