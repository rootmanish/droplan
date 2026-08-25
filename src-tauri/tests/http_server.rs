//! End-to-end tests against a real listener.
//!
//! These drive the actual axum server over a real TCP socket, which is the
//! only way to be confident about the things that matter most here: that a
//! download is byte-exact, that ranges work, and that nothing outside the
//! share registry is reachable.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use droplan_core::events::EventBus;
use droplan_core::server::{self, ServerContext, ServerHandle};
use droplan_core::sharing::{ShareRegistry, ShareSession};
use droplan_core::transfer::TransferTracker;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

struct Harness {
    dir: TempDir,
    registry: Arc<ShareRegistry>,
    session: Arc<RwLock<ShareSession>>,
    tracker: Arc<TransferTracker>,
    handle: Option<ServerHandle>,
    cancel: CancellationToken,
    port: u16,
    token: String,
}

impl Harness {
    async fn start() -> Self {
        Self::start_with_pin(false).await
    }

    async fn start_with_pin(with_pin: bool) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let events = EventBus::default();
        let registry = Arc::new(ShareRegistry::new());
        let tracker = Arc::new(TransferTracker::new(events.clone()));
        let session = Arc::new(RwLock::new(ShareSession::new(with_pin).expect("session")));
        let token = session.read().expect("read").token().to_string();
        let cancel = CancellationToken::new();

        let context = ServerContext {
            registry: Arc::clone(&registry),
            session: Arc::clone(&session),
            tracker: Arc::clone(&tracker),
            events,
            device_name: Arc::from("Test Machine"),
            shutdown: cancel.clone(),
        };

        // Port 0 lets the OS pick, so tests never collide with each other.
        let handle = server::start(context, 0, 1).await.expect("server start");
        let port = handle.port;

        Harness {
            dir,
            registry,
            session,
            tracker,
            handle: Some(handle),
            cancel,
            port,
            token,
        }
    }

    fn write_file(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.dir.path().join(name);
        std::fs::write(&path, bytes).expect("write");
        let outcome = self.registry.add_paths(&[path]).expect("add");
        outcome.added[0].id.clone()
    }

    fn base(&self) -> String {
        format!("http://127.0.0.1:{}/s/{}", self.port, self.token)
    }

    fn url(&self, suffix: &str) -> String {
        format!("{}{suffix}", self.base())
    }

    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn pin(&self) -> String {
        self.session
            .read()
            .expect("read")
            .pin()
            .expect("pin")
            .to_string()
    }

    async fn stop(mut self) {
        self.cancel.cancel();
        if let Some(handle) = self.handle.take() {
            handle.shutdown().await;
        }
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        // Every test asserts on the exact response it gets.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client")
}

/// Deterministic filler so a byte-exact comparison actually proves something.
fn payload(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

// ---------------------------------------------------------------- basic pages

#[tokio::test]
async fn the_root_page_is_served_without_revealing_the_session() {
    let harness = Harness::start().await;
    harness.write_file("secret-report.pdf", b"data");

    let response = client().get(harness.origin()).send().await.expect("get");
    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("body");
    assert!(!body.contains(&harness.token));
    assert!(!body.contains("secret-report.pdf"));
    harness.stop().await;
}

#[tokio::test]
async fn health_is_minimal_and_leaks_nothing() {
    let harness = Harness::start().await;

    let response = client()
        .get(format!("{}/health", harness.origin()))
        .send()
        .await
        .expect("get");
    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("body");
    assert_eq!(body, "ok");
    assert!(!body.contains(&harness.token));
    harness.stop().await;
}

#[tokio::test]
async fn the_share_page_lists_the_shared_files() {
    let harness = Harness::start().await;
    harness.write_file("report.pdf", &payload(13_002_342));
    harness.write_file("demo.mp4", &payload(4096));

    let body = client()
        .get(harness.base())
        .send()
        .await
        .expect("get")
        .text()
        .await
        .expect("body");

    assert!(body.contains("Files from Test Machine"));
    assert!(body.contains("report.pdf"));
    assert!(body.contains("demo.mp4"));
    assert!(body.contains("12.4 MB"));
    assert!(body.contains("2 files"));
    harness.stop().await;
}

#[tokio::test]
async fn the_json_api_matches_the_page() {
    let harness = Harness::start().await;
    let id = harness.write_file("a.bin", &payload(1234));

    let body: serde_json::Value = client()
        .get(harness.url("/api/files"))
        .send()
        .await
        .expect("get")
        .json()
        .await
        .expect("json");

    assert_eq!(body["deviceName"], "Test Machine");
    assert_eq!(body["count"], 1);
    assert_eq!(body["totalBytes"], 1234);
    assert_eq!(body["files"][0]["id"], id);
    assert_eq!(body["files"][0]["name"], "a.bin");
    assert!(body["signature"].as_str().is_some_and(|s| !s.is_empty()));
    harness.stop().await;
}

#[tokio::test]
async fn security_headers_are_present_on_every_response() {
    let harness = Harness::start().await;

    for url in [harness.origin(), harness.base()] {
        let response = client().get(&url).send().await.expect("get");
        let headers = response.headers();
        // Without this, following any link out would hand over the token.
        assert_eq!(
            headers.get("referrer-policy").expect("policy"),
            "no-referrer"
        );
        assert_eq!(
            headers.get("x-content-type-options").expect("nosniff"),
            "nosniff"
        );
    }
    harness.stop().await;
}

// ------------------------------------------------------------------ downloads

#[tokio::test]
async fn a_download_is_byte_exact_with_correct_headers() {
    let harness = Harness::start().await;
    let bytes = payload(150_000);
    let id = harness.write_file("build.zip", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("accept-ranges").expect("ranges"),
        "bytes"
    );
    assert_eq!(
        response.headers().get("content-type").expect("type"),
        "application/zip"
    );
    assert_eq!(
        response.headers().get("content-length").expect("len"),
        &bytes.len().to_string()
    );
    let disposition = response
        .headers()
        .get("content-disposition")
        .expect("disposition")
        .to_str()
        .expect("utf8")
        .to_string();
    assert!(disposition.starts_with("attachment;"));
    assert!(disposition.contains("build.zip"));

    let body = response.bytes().await.expect("body");
    assert_eq!(body.len(), bytes.len());
    assert_eq!(body.as_ref(), bytes.as_slice());
    harness.stop().await;
}

#[tokio::test]
async fn media_is_served_inline_unless_download_is_forced() {
    let harness = Harness::start().await;
    let id = harness.write_file("clip.mp4", &payload(2048));

    let inline = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");
    assert!(inline
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|value| value.starts_with("inline;")));

    let forced = client()
        .get(harness.url(&format!("/files/{id}?dl=1")))
        .send()
        .await
        .expect("get");
    assert!(forced
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|value| value.starts_with("attachment;")));
    harness.stop().await;
}

#[tokio::test]
async fn a_non_ascii_filename_survives_the_round_trip() {
    let harness = Harness::start().await;
    let id = harness.write_file("überraschung wichtig.pdf", b"x");

    let response = client()
        .get(harness.url(&format!("/files/{id}?dl=1")))
        .send()
        .await
        .expect("get");
    let disposition = response
        .headers()
        .get("content-disposition")
        .expect("disposition")
        .to_str()
        .expect("ascii header");

    assert!(disposition.contains("filename*=UTF-8''%C3%BCberraschung"));
    assert!(disposition.contains("filename=\""));
    harness.stop().await;
}

#[tokio::test]
async fn an_empty_file_downloads_cleanly() {
    let harness = Harness::start().await;
    let id = harness.write_file("empty.txt", b"");

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-length").expect("len"), "0");
    assert!(response.bytes().await.expect("body").is_empty());
    harness.stop().await;
}

#[tokio::test]
async fn head_reports_size_and_range_support_without_a_body() {
    let harness = Harness::start().await;
    let bytes = payload(9999);
    let id = harness.write_file("a.bin", &bytes);

    let response = client()
        .head(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("head");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-length").expect("len"),
        "9999"
    );
    assert_eq!(
        response.headers().get("accept-ranges").expect("ranges"),
        "bytes"
    );
    assert!(response.bytes().await.expect("body").is_empty());
    harness.stop().await;
}

#[tokio::test]
async fn a_conditional_request_gets_a_304() {
    let harness = Harness::start().await;
    let id = harness.write_file("a.bin", &payload(500));

    let first = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");
    let etag = first.headers().get("etag").expect("etag").clone();

    let second = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("if-none-match", etag)
        .send()
        .await
        .expect("get");
    assert_eq!(second.status(), 304);
    assert!(second.bytes().await.expect("body").is_empty());
    harness.stop().await;
}

// --------------------------------------------------------------------- ranges

#[tokio::test]
async fn a_range_request_returns_exactly_that_slice() {
    let harness = Harness::start().await;
    let bytes = payload(100_000);
    let id = harness.write_file("video.mp4", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("range", "bytes=1000-1999")
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 206);
    assert_eq!(
        response.headers().get("content-range").expect("range"),
        "bytes 1000-1999/100000"
    );
    assert_eq!(
        response.headers().get("content-length").expect("len"),
        "1000"
    );

    let body = response.bytes().await.expect("body");
    assert_eq!(body.as_ref(), &bytes[1000..2000]);
    harness.stop().await;
}

#[tokio::test]
async fn an_open_ended_range_resumes_to_the_end_of_the_file() {
    let harness = Harness::start().await;
    let bytes = payload(50_000);
    let id = harness.write_file("big.iso", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("range", "bytes=49000-")
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 206);
    assert_eq!(
        response.headers().get("content-range").expect("range"),
        "bytes 49000-49999/50000"
    );
    assert_eq!(
        response.bytes().await.expect("body").as_ref(),
        &bytes[49000..]
    );
    harness.stop().await;
}

#[tokio::test]
async fn a_suffix_range_returns_the_tail() {
    let harness = Harness::start().await;
    let bytes = payload(10_000);
    let id = harness.write_file("a.bin", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("range", "bytes=-256")
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 206);
    assert_eq!(
        response.bytes().await.expect("body").as_ref(),
        &bytes[9744..]
    );
    harness.stop().await;
}

#[tokio::test]
async fn an_unsatisfiable_range_returns_416_with_the_real_size() {
    let harness = Harness::start().await;
    let id = harness.write_file("a.bin", &payload(1000));

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("range", "bytes=5000-6000")
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 416);
    assert_eq!(
        response.headers().get("content-range").expect("range"),
        "bytes */1000"
    );
    harness.stop().await;
}

#[tokio::test]
async fn reassembling_a_file_from_ranges_reproduces_it_exactly() {
    let harness = Harness::start().await;
    let bytes = payload(65_536);
    let id = harness.write_file("resume.bin", &bytes);
    let url = harness.url(&format!("/files/{id}"));

    // Simulate a download interrupted three times and resumed each time.
    let mut assembled: Vec<u8> = Vec::new();
    for (start, end) in [(0usize, 9_999usize), (10_000, 40_959), (40_960, 65_535)] {
        let response = client()
            .get(&url)
            .header("range", format!("bytes={start}-{end}"))
            .send()
            .await
            .expect("get");
        assert_eq!(response.status(), 206);
        assembled.extend_from_slice(&response.bytes().await.expect("body"));
    }
    assert_eq!(assembled, bytes);
    harness.stop().await;
}

#[tokio::test]
async fn a_stale_if_range_falls_back_to_the_whole_file() {
    let harness = Harness::start().await;
    let bytes = payload(4000);
    let id = harness.write_file("a.bin", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("range", "bytes=0-99")
        .header("if-range", "\"a-validator-from-a-different-file\"")
        .send()
        .await
        .expect("get");

    assert_eq!(
        response.status(),
        200,
        "a changed file must restart the download"
    );
    assert_eq!(response.bytes().await.expect("body").len(), bytes.len());
    harness.stop().await;
}

#[tokio::test]
async fn a_multi_range_request_is_declined_and_the_whole_file_is_sent() {
    let harness = Harness::start().await;
    let bytes = payload(2000);
    let id = harness.write_file("a.bin", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header("range", "bytes=0-99,500-599")
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 200);
    assert_eq!(response.bytes().await.expect("body").len(), bytes.len());
    harness.stop().await;
}

// -------------------------------------------------------------------- refusal

#[tokio::test]
async fn a_wrong_session_token_is_indistinguishable_from_a_missing_page() {
    let harness = Harness::start().await;
    let id = harness.write_file("a.bin", b"secret");

    for token in [
        "wrong".to_string(),
        harness.token.to_uppercase(),
        harness.token[..harness.token.len() - 1].to_string(),
        format!("{}x", harness.token),
        String::new(),
    ] {
        let url = format!("{}/s/{token}/files/{id}", harness.origin());
        let response = client().get(&url).send().await.expect("get");
        assert_eq!(
            response.status(),
            404,
            "token {token:?} should not be accepted"
        );
        assert!(!response.text().await.expect("body").contains("secret"));
    }
    harness.stop().await;
}

#[tokio::test]
async fn path_traversal_never_escapes_the_share_registry() {
    let harness = Harness::start().await;
    harness.write_file("shared.txt", b"shared");

    // A file that exists on disk but was never shared.
    let private = harness.dir.path().join("private.txt");
    std::fs::write(&private, b"PRIVATE").expect("write");

    let attempts = [
        format!("{}/files/../../../etc/passwd", harness.base()),
        format!("{}/files/..%2f..%2f..%2fetc%2fpasswd", harness.base()),
        format!("{}/files/%2e%2e%2f%2e%2e%2fetc%2fpasswd", harness.base()),
        format!("{}/files/{}", harness.base(), private.display()),
        format!("{}/files/..\\..\\windows\\win.ini", harness.base()),
        format!("{}/files/....//....//etc/passwd", harness.base()),
        format!("{}/files/private.txt", harness.base()),
        format!("{}/files//etc/passwd", harness.base()),
        format!("{}/etc/passwd", harness.base()),
        format!("{}/../../../etc/passwd", harness.origin()),
        format!("{}/files/%00", harness.base()),
    ];

    for url in attempts {
        let response = client().get(&url).send().await.expect("request");
        assert!(
            response.status().is_client_error(),
            "{url} returned {}",
            response.status()
        );
        let body = response.text().await.unwrap_or_default();
        assert!(!body.contains("PRIVATE"), "{url} leaked an unshared file");
        assert!(!body.contains("root:"), "{url} leaked /etc/passwd");
    }
    harness.stop().await;
}

#[tokio::test]
async fn an_unknown_file_id_is_a_404() {
    let harness = Harness::start().await;
    harness.write_file("a.bin", b"x");

    for id in ["", "aaaaaaaaaaaaaaaa", "0", "%20", "null", "' OR 1=1--"] {
        let response = client()
            .get(harness.url(&format!("/files/{id}")))
            .send()
            .await
            .expect("get");
        assert!(
            response.status().is_client_error(),
            "id {id:?} was accepted"
        );
    }
    harness.stop().await;
}

#[tokio::test]
async fn removing_a_file_invalidates_its_url_immediately() {
    let harness = Harness::start().await;
    let id = harness.write_file("demo.mp4", &payload(4096));
    let url = harness.url(&format!("/files/{id}"));

    assert_eq!(client().get(&url).send().await.expect("get").status(), 200);

    harness.registry.remove(&id).expect("remove");

    assert_eq!(
        client().get(&url).send().await.expect("get").status(),
        404,
        "a removed file must stop being downloadable at once"
    );
    harness.stop().await;
}

#[tokio::test]
async fn clearing_the_registry_invalidates_every_url() {
    let harness = Harness::start().await;
    let a = harness.write_file("a.bin", b"a");
    let b = harness.write_file("b.bin", b"b");

    harness.registry.clear().expect("clear");

    for id in [a, b] {
        let response = client()
            .get(harness.url(&format!("/files/{id}")))
            .send()
            .await
            .expect("get");
        assert_eq!(response.status(), 404);
    }
    harness.stop().await;
}

#[tokio::test]
async fn a_file_deleted_from_disk_reports_gone_and_is_marked_unavailable() {
    let harness = Harness::start().await;
    let id = harness.write_file("vanishing.txt", b"here for now");
    std::fs::remove_file(harness.dir.path().join("vanishing.txt")).expect("remove");

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");

    assert_eq!(response.status(), 410);
    assert!(response
        .text()
        .await
        .expect("body")
        .contains("File unavailable"));

    let item = harness
        .registry
        .get(&id)
        .expect("get")
        .expect("still listed");
    assert!(
        !item.available,
        "the desktop list should show it as unavailable"
    );
    harness.stop().await;
}

#[tokio::test]
async fn unknown_paths_return_the_generic_not_found_page() {
    let harness = Harness::start().await;

    for path in ["/nope", "/s", "/api/files", "/.env", "/admin"] {
        let response = client()
            .get(format!("{}{path}", harness.origin()))
            .send()
            .await
            .expect("get");
        assert_eq!(response.status(), 404, "{path}");
    }
    harness.stop().await;
}

// ------------------------------------------------------------------------ PIN

#[tokio::test]
async fn a_pin_protected_session_requires_the_pin_before_anything_else() {
    let harness = Harness::start_with_pin(true).await;
    let id = harness.write_file("secret.txt", b"CLASSIFIED");

    let page = client().get(harness.base()).send().await.expect("get");
    assert_eq!(page.status(), 401);
    let body = page.text().await.expect("body");
    assert!(body.contains("Enter the PIN"));
    assert!(!body.contains("secret.txt"));

    // The gate also covers direct file access, not just the listing.
    let direct = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");
    assert_eq!(direct.status(), 401);
    assert!(!direct.text().await.expect("body").contains("CLASSIFIED"));
    harness.stop().await;
}

#[tokio::test]
async fn the_right_pin_unlocks_the_session_and_the_wrong_one_does_not() {
    let harness = Harness::start_with_pin(true).await;
    let id = harness.write_file("secret.txt", b"CLASSIFIED");
    let jar_client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(3))
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client");

    let wrong = jar_client
        .post(harness.url("/unlock"))
        .form(&[("pin", "000000000")])
        .send()
        .await
        .expect("post");
    assert_eq!(wrong.status(), 401);
    assert!(wrong.text().await.expect("body").contains("did not match"));

    let right = jar_client
        .post(harness.url("/unlock"))
        .form(&[("pin", harness.pin().as_str())])
        .send()
        .await
        .expect("post");
    assert_eq!(
        right.status(),
        200,
        "unlock should redirect to the share page"
    );
    assert!(right.text().await.expect("body").contains("secret.txt"));

    let download = jar_client
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");
    assert_eq!(download.status(), 200);
    assert_eq!(
        download.bytes().await.expect("body").as_ref(),
        b"CLASSIFIED"
    );
    harness.stop().await;
}

// ---------------------------------------------------------------- concurrency

#[tokio::test]
async fn many_simultaneous_downloads_all_complete_intact() {
    let harness = Harness::start().await;
    let bytes = payload(512 * 1024);
    let id = harness.write_file("shared.bin", &bytes);
    let url = harness.url(&format!("/files/{id}"));

    let mut tasks = Vec::new();
    for _ in 0..12 {
        let url = url.clone();
        tasks.push(tokio::spawn(async move {
            let response = client().get(&url).send().await.expect("get");
            assert_eq!(response.status(), 200);
            response.bytes().await.expect("body").to_vec()
        }));
    }

    for task in tasks {
        assert_eq!(task.await.expect("join"), bytes);
    }
    harness.stop().await;
}

#[tokio::test]
async fn concurrent_range_requests_for_different_files_do_not_interfere() {
    let harness = Harness::start().await;
    let a_bytes = payload(40_000);
    let b_bytes: Vec<u8> = payload(40_000).into_iter().rev().collect();
    let a = harness.write_file("a.bin", &a_bytes);
    let b = harness.write_file("b.bin", &b_bytes);

    let (left, right) = tokio::join!(
        client()
            .get(harness.url(&format!("/files/{a}")))
            .header("range", "bytes=100-199")
            .send(),
        client()
            .get(harness.url(&format!("/files/{b}")))
            .header("range", "bytes=100-199")
            .send(),
    );

    let left = left.expect("a").bytes().await.expect("body");
    let right = right.expect("b").bytes().await.expect("body");
    assert_eq!(left.as_ref(), &a_bytes[100..200]);
    assert_eq!(right.as_ref(), &b_bytes[100..200]);
    assert_ne!(left, right);
    harness.stop().await;
}

#[tokio::test]
async fn a_large_file_streams_without_being_read_into_memory() {
    let harness = Harness::start().await;
    // 24 MB is enough to span many chunks while keeping the test quick.
    let bytes = payload(24 * 1024 * 1024);
    let id = harness.write_file("large.bin", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .send()
        .await
        .expect("get");
    assert_eq!(response.status(), 200);

    // Consume it as a stream, the way a browser does.
    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    let mut received = 0usize;
    let mut first_chunk = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("chunk");
        if first_chunk.is_empty() {
            first_chunk = chunk.to_vec();
        }
        received += chunk.len();
    }

    assert_eq!(received, bytes.len());
    assert_eq!(&first_chunk[..16], &bytes[..16]);
    harness.stop().await;
}

// ------------------------------------------------------------------ lifecycle

#[tokio::test]
async fn transfers_are_recorded_in_the_activity_list() {
    let harness = Harness::start().await;
    let bytes = payload(20_000);
    let id = harness.write_file("tracked.bin", &bytes);

    let response = client()
        .get(harness.url(&format!("/files/{id}")))
        .header(
            "user-agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) Safari/604.1",
        )
        .send()
        .await
        .expect("get");
    let _ = response.bytes().await.expect("body");

    // The stream completes slightly after the body is delivered.
    for _ in 0..50 {
        if !harness.tracker.snapshot().recent.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let activity = harness.tracker.snapshot();
    let record = activity.recent.first().expect("a completed transfer");
    assert_eq!(record.file_name, "tracked.bin");
    assert_eq!(record.transferred_bytes, bytes.len() as u64);
    assert_eq!(record.client_ip, "127.0.0.1");

    let client_record = activity.clients.first().expect("a client");
    assert_eq!(client_record.device, "iPhone");
    assert_eq!(client_record.browser, "Safari");
    harness.stop().await;
}

#[tokio::test]
async fn stopping_the_server_closes_the_port() {
    let harness = Harness::start().await;
    let id = harness.write_file("a.bin", b"x");
    let url = harness.url(&format!("/files/{id}"));
    let origin = harness.origin();

    assert_eq!(client().get(&url).send().await.expect("get").status(), 200);
    harness.stop().await;

    let health = reqwest::Client::new()
        .get(format!("{origin}/health"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;
    assert!(health.is_err(), "the listener must be gone after shutdown");
}
