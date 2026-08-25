//! The gate in front of everything under `/s/{token}`.
//!
//! Two checks, in order: the session token must match the live session, and
//! if a PIN is configured the browser must have already unlocked. A wrong or
//! stale token gets the same generic 404 as a nonexistent path, so probing
//! cannot distinguish "wrong token" from "no session".

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::server::page;
use crate::sharing::session::UNLOCK_COOKIE;

use super::ServerContext;

/// Pull the session token out of `/s/<token>[/...]`.
///
/// Percent-encoded input simply fails to match the token later, which is the
/// desired outcome: the token alphabet contains nothing that needs encoding.
pub fn token_from_path(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/s/")?;
    let token = rest.split('/').next()?;
    (!token.is_empty()).then_some(token)
}

/// Read one cookie value out of a `Cookie` header.
pub fn cookie_value<'a>(header_value: &'a str, name: &str) -> Option<&'a str> {
    header_value.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key.trim() == name {
            Some(value.trim())
        } else {
            None
        }
    })
}

/// True when this request is the PIN form submission itself, which must be
/// allowed through the PIN check so the user can actually unlock.
fn is_unlock_submission(method: &Method, path: &str) -> bool {
    method == Method::POST && path.ends_with("/unlock")
}

pub async fn session_guard(
    State(ctx): State<ServerContext>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    let Some(candidate) = token_from_path(&path) else {
        return not_found();
    };

    let (token_ok, requires_pin, unlocked) = {
        let Ok(session) = ctx.session.read() else {
            return internal_error();
        };
        let token_ok = session.accepts_token(candidate);
        let cookie = request
            .headers()
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| cookie_value(value, UNLOCK_COOKIE));
        (
            token_ok,
            session.requires_pin(),
            session.is_unlocked(cookie),
        )
    };

    if !token_ok {
        tracing::debug!(target: "droplan", "rejected a request with an invalid session token");
        return not_found();
    }
    if requires_pin && !unlocked && !is_unlock_submission(request.method(), &path) {
        let base = format!("/s/{candidate}");
        return (
            StatusCode::UNAUTHORIZED,
            html(page::render_pin_page(&ctx.device_name, &base, false)),
        )
            .into_response();
    }

    next.run(request).await
}

pub fn html(body: String) -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Body::from(body),
    )
        .into_response()
}

pub fn not_found() -> Response {
    (StatusCode::NOT_FOUND, html(page::render_not_found_page())).into_response()
}

pub fn gone() -> Response {
    (StatusCode::GONE, html(page::render_gone_page())).into_response()
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        html(page::render_not_found_page()),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_extracted_from_every_session_path_shape() {
        assert_eq!(token_from_path("/s/abc123"), Some("abc123"));
        assert_eq!(token_from_path("/s/abc123/"), Some("abc123"));
        assert_eq!(token_from_path("/s/abc123/files/xyz"), Some("abc123"));
        assert_eq!(token_from_path("/s/abc123/api/files"), Some("abc123"));
    }

    #[test]
    fn paths_without_a_token_yield_nothing() {
        assert_eq!(token_from_path("/"), None);
        assert_eq!(token_from_path("/s/"), None);
        assert_eq!(token_from_path("/s"), None);
        assert_eq!(token_from_path("/health"), None);
        assert_eq!(token_from_path("/files/xyz"), None);
        assert_eq!(token_from_path("//s/abc"), None);
    }

    #[test]
    fn a_traversal_attempt_is_read_as_a_token_and_will_simply_not_match() {
        // The point is that it is never treated as a path segment on disk.
        assert_eq!(
            token_from_path("/s/..%2f..%2fetc/passwd"),
            Some("..%2f..%2fetc")
        );
        assert_eq!(token_from_path("/s/../../etc/passwd"), Some(".."));
    }

    #[test]
    fn cookies_are_parsed_out_of_a_combined_header() {
        assert_eq!(
            cookie_value("droplan_unlock=abc", "droplan_unlock"),
            Some("abc")
        );
        assert_eq!(
            cookie_value("theme=dark; droplan_unlock=abc; other=1", "droplan_unlock"),
            Some("abc")
        );
        assert_eq!(
            cookie_value("  droplan_unlock = abc  ", "droplan_unlock"),
            Some("abc")
        );
        assert_eq!(cookie_value("theme=dark", "droplan_unlock"), None);
        assert_eq!(cookie_value("", "droplan_unlock"), None);
        assert_eq!(cookie_value("droplan_unlock", "droplan_unlock"), None);
    }

    #[test]
    fn a_prefix_named_cookie_does_not_match() {
        assert_eq!(cookie_value("droplan_unlock_x=abc", "droplan_unlock"), None);
        assert_eq!(cookie_value("xdroplan_unlock=abc", "droplan_unlock"), None);
    }

    #[test]
    fn only_a_post_to_unlock_bypasses_the_pin_gate() {
        assert!(is_unlock_submission(&Method::POST, "/s/tok/unlock"));
        assert!(!is_unlock_submission(&Method::GET, "/s/tok/unlock"));
        assert!(!is_unlock_submission(&Method::POST, "/s/tok/files/abc"));
        assert!(!is_unlock_submission(
            &Method::POST,
            "/s/tok/unlocked-files"
        ));
    }
}
