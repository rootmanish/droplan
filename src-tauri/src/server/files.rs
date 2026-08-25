//! Streaming file responses.
//!
//! Bytes go straight from the filesystem to the socket in 64 KB chunks; a
//! 50 GB file costs the same memory as a 50 KB one. Every response is opened
//! against a path that was resolved when the user added the file, never
//! against anything derived from the request.

use std::io::SeekFrom;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::Stream;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::events::AppEvent;
use crate::security::paths;
use crate::transfer::{TransferGuard, TransferStart};

use super::middleware::{gone, not_found};
use super::range::{self, RangeRequest};
use super::ServerContext;

/// Read size. Large enough to keep the syscall count low on gigabit LAN,
/// small enough that many concurrent downloads stay cheap.
const CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Deserialize, Default)]
pub struct DownloadQuery {
    /// `?dl=1` forces a save dialog even for media the browser could play.
    #[serde(default)]
    pub dl: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn download(
    State(ctx): State<ServerContext>,
    Path((_token, file_id)): Path<(String, String)>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(query): Query<DownloadQuery>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    // The id is looked up, never interpreted. An unknown id is simply absent.
    let item = match ctx.registry.get(&file_id) {
        Ok(Some(item)) => item,
        Ok(None) => return not_found(),
        Err(err) => {
            tracing::error!(target: "droplan", "registry lookup failed: {err}");
            return not_found();
        }
    };

    let mut file = match tokio::fs::File::open(&item.absolute_path).await {
        Ok(file) => file,
        Err(err) => {
            // The file moved or was deleted after it was shared. Tell the
            // desktop side so it can grey the row out.
            tracing::info!(target: "droplan", "shared file is unreadable: {err}");
            if ctx.registry.mark_unavailable(&item.id).unwrap_or(false) {
                ctx.events.publish(AppEvent::notice(
                    "file_unavailable",
                    format!("{} is no longer available on disk.", item.display_name),
                ));
                ctx.publish_files_changed();
            }
            return gone();
        }
    };

    let (file_size, modified) = match file.metadata().await {
        Ok(metadata) => (metadata.len(), modified_seconds(&metadata)),
        Err(_) => return gone(),
    };

    let etag = etag_for(&item.id, file_size, modified);
    if let Some(client_etag) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    {
        if client_etag.split(',').any(|value| value.trim() == etag) {
            return (
                StatusCode::NOT_MODIFIED,
                common_headers(&etag),
                Body::empty(),
            )
                .into_response();
        }
    }

    let wants_range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        // A resumed download sends `If-Range`. If the file changed underneath
        // us the validator will not match and we must send the whole file.
        .filter(|_| if_range_matches(&headers, &etag))
        .map(|value| range::parse_range(value, file_size))
        .unwrap_or(RangeRequest::None);

    let (status, start, length) = match wants_range {
        RangeRequest::Unsatisfiable => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_RANGE,
                header_value(&range::unsatisfied_content_range(file_size)),
            );
            headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            return (StatusCode::RANGE_NOT_SATISFIABLE, headers, Body::empty()).into_response();
        }
        RangeRequest::Satisfiable(byte_range) => (
            StatusCode::PARTIAL_CONTENT,
            byte_range.start,
            byte_range.len(),
        ),
        RangeRequest::None => (StatusCode::OK, 0, file_size),
    };

    let force_download = query.dl.is_some();
    let inline = !force_download && paths::is_inline_previewable(&item.mime_type);

    let mut response_headers = common_headers(&etag);
    response_headers.insert(header::CONTENT_TYPE, header_value(&item.mime_type));
    response_headers.insert(header::CONTENT_LENGTH, header_value(&length.to_string()));
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        header_value(&paths::content_disposition(&item.display_name, inline)),
    );
    if let RangeRequest::Satisfiable(byte_range) = wants_range {
        response_headers.insert(
            header::CONTENT_RANGE,
            header_value(&byte_range.content_range(file_size)),
        );
    }

    // HEAD is how a download manager asks "how big, and can you do ranges?".
    // Answer with headers only, and do not open a transfer record for it.
    if method == Method::HEAD {
        return (status, response_headers, Body::empty()).into_response();
    }

    if start > 0 {
        if let Err(err) = file.seek(SeekFrom::Start(start)).await {
            tracing::warn!(target: "droplan", "seek failed: {err}");
            return gone();
        }
    }

    let guard = ctx.tracker.begin(TransferStart {
        file_id: item.id.clone(),
        file_name: item.display_name.clone(),
        total_bytes: length,
        file_bytes: file_size,
        is_range_request: matches!(wants_range, RangeRequest::Satisfiable(_)),
        client_ip: peer.ip().to_string(),
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
    });

    let reader = ReaderStream::with_capacity(file.take(length), CHUNK_SIZE);
    let body = Body::from_stream(TrackedStream::new(reader, guard, ctx.clone()));

    (status, response_headers, body).into_response()
}

/// Strong validator built from the opaque id, the size and the modification
/// time.
///
/// Size alone is not enough: a file edited in place to the same byte length
/// would keep its validator and a revalidating client would be handed a stale
/// 304. Including mtime also keeps `If-Range` honest, so a resumed download of
/// a file that changed underneath restarts instead of stitching two versions
/// together.
fn etag_for(file_id: &str, size: u64, modified: u64) -> String {
    format!("\"{file_id}-{size:x}-{modified:x}\"")
}

/// Modification time as whole seconds since the epoch, or 0 where the
/// filesystem does not report one.
fn modified_seconds(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn if_range_matches(headers: &HeaderMap, etag: &str) -> bool {
    match headers.get(header::IF_RANGE).and_then(|v| v.to_str().ok()) {
        None => true,
        Some(value) => value.trim() == etag,
    }
}

fn common_headers(etag: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(header::ETAG, header_value(etag));
    // Revalidate every time: a shared file can be swapped or revoked.
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=0, must-revalidate"),
    );
    headers
}

/// Wraps the file stream so that every chunk is counted, and so that stopping
/// sharing tears down downloads that are already in flight.
struct TrackedStream {
    inner: Pin<Box<dyn Stream<Item = std::io::Result<Bytes>> + Send>>,
    guard: Option<TransferGuard>,
    ctx: ServerContext,
}

impl TrackedStream {
    fn new<S>(inner: S, guard: TransferGuard, ctx: ServerContext) -> Self
    where
        S: Stream<Item = std::io::Result<Bytes>> + Send + 'static,
    {
        TrackedStream {
            inner: Box::pin(inner),
            guard: Some(guard),
            ctx,
        }
    }
}

impl Stream for TrackedStream {
    type Item = std::io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Checked on every chunk: "Stop sharing" must cut off a download that
        // is halfway through a 10 GB file, not wait for it to finish.
        if self.ctx.shutdown.is_cancelled() {
            if let Some(guard) = self.guard.take() {
                guard.fail();
            }
            return Poll::Ready(Some(Err(std::io::Error::other("sharing stopped"))));
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if let Some(guard) = self.guard.as_ref() {
                    guard.advance(chunk.len() as u64);
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                if let Some(guard) = self.guard.take() {
                    guard.fail();
                }
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                if let Some(guard) = self.guard.take() {
                    guard.complete();
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// `Drop` covers the case the stream is never polled to completion, which is
/// what happens when the client disconnects.
impl Drop for TrackedStream {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            guard.fail();
        }
    }
}

pub(crate) fn header_value(value: &str) -> HeaderValue {
    HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etags_change_with_size_identity_and_mtime() {
        assert_eq!(etag_for("abc", 1024, 16), "\"abc-400-10\"");
        assert_ne!(etag_for("abc", 1024, 16), etag_for("abc", 1025, 16));
        assert_ne!(etag_for("abc", 1024, 16), etag_for("abd", 1024, 16));
        // The case that size alone would miss: same length, edited in place.
        assert_ne!(etag_for("abc", 1024, 16), etag_for("abc", 1024, 17));
        // Must be a valid quoted-string.
        assert!(etag_for("abc", 1, 1).starts_with('"'));
        assert!(etag_for("abc", 1, 1).ends_with('"'));
    }

    #[test]
    fn a_filesystem_without_mtime_still_produces_a_validator() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"x").expect("write");
        let metadata = std::fs::metadata(&path).expect("metadata");
        // Whatever the platform reports, this must not panic.
        let _ = modified_seconds(&metadata);
    }

    #[test]
    fn if_range_is_honoured_only_on_an_exact_validator_match() {
        let etag = etag_for("abc", 10, 1);
        let mut headers = HeaderMap::new();
        assert!(
            if_range_matches(&headers, &etag),
            "absent If-Range means proceed"
        );

        headers.insert(header::IF_RANGE, header_value(&etag));
        assert!(if_range_matches(&headers, &etag));

        headers.insert(header::IF_RANGE, header_value("\"something-else\""));
        assert!(!if_range_matches(&headers, &etag));
    }

    #[test]
    fn common_headers_always_advertise_range_support() {
        let headers = common_headers("\"x\"");
        assert_eq!(
            headers
                .get(header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok()),
            Some("bytes")
        );
        assert_eq!(
            headers.get(header::ETAG).and_then(|v| v.to_str().ok()),
            Some("\"x\"")
        );
        assert!(headers
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|value| value.contains("must-revalidate")));
    }

    #[test]
    fn an_invalid_header_value_degrades_instead_of_panicking() {
        assert_eq!(header_value("ok").to_str().ok(), Some("ok"));
        assert_eq!(header_value("bad\nvalue").to_str().ok(), Some(""));
    }
}
