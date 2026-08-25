//! A sharing session: the unguessable token that gates every LAN request.
//!
//! A session is created when sharing starts and thrown away when it stops. A
//! fresh application launch is therefore always a fresh session, so a link
//! someone kept from yesterday is dead.

use serde::Serialize;

use crate::error::Result;
use crate::security::tokens;

use super::registry::now_millis;

/// Digits in a generated PIN.
pub const PIN_DIGITS: usize = 6;

/// Cookie that records a successful PIN unlock for one session.
pub const UNLOCK_COOKIE: &str = "droplan_unlock";

#[derive(Debug, Clone)]
pub struct ShareSession {
    /// High-entropy path segment: `/s/<token>`.
    token: String,
    /// Set only when the user turned on PIN protection.
    pin: Option<String>,
    /// Value handed to a browser that entered the right PIN. Rotates with the
    /// session, so regenerating the link also logs every browser out.
    unlock_secret: String,
    started_at: u64,
}

impl ShareSession {
    pub fn new(with_pin: bool) -> Result<Self> {
        Ok(ShareSession {
            token: tokens::session_token()?,
            pin: if with_pin {
                Some(tokens::random_pin(PIN_DIGITS)?)
            } else {
                None
            },
            unlock_secret: tokens::unlock_secret()?,
            started_at: now_millis(),
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn pin(&self) -> Option<&str> {
        self.pin.as_deref()
    }

    pub fn requires_pin(&self) -> bool {
        self.pin.is_some()
    }

    pub fn started_at(&self) -> u64 {
        self.started_at
    }

    pub fn unlock_secret(&self) -> &str {
        &self.unlock_secret
    }

    /// URL path prefix for everything this session exposes.
    pub fn base_path(&self) -> String {
        format!("/s/{}", self.token)
    }

    /// Full URL to show the user and encode into the QR code.
    pub fn share_url(&self, host: &str, port: u16) -> String {
        format!("http://{host}:{port}{}", self.base_path())
    }

    /// Token check for an incoming request. Constant time, so a client on the
    /// LAN cannot narrow the token down by timing repeated guesses.
    pub fn accepts_token(&self, candidate: &str) -> bool {
        tokens::constant_time_eq(&self.token, candidate)
    }

    /// PIN check for the unlock form. Also constant time.
    pub fn accepts_pin(&self, candidate: &str) -> bool {
        match &self.pin {
            None => true,
            Some(pin) => tokens::constant_time_eq(pin, candidate.trim()),
        }
    }

    /// Cookie check for an already-unlocked browser.
    pub fn accepts_unlock_secret(&self, candidate: &str) -> bool {
        tokens::constant_time_eq(&self.unlock_secret, candidate)
    }

    /// Whether this request may proceed past the PIN gate.
    pub fn is_unlocked(&self, cookie_value: Option<&str>) -> bool {
        if !self.requires_pin() {
            return true;
        }
        cookie_value
            .map(|value| self.accepts_unlock_secret(value))
            .unwrap_or(false)
    }
}

/// What the desktop UI is told about the current session. The PIN is included
/// because the user has to read it out; the unlock secret never is.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub token: String,
    pub base_path: String,
    pub pin: Option<String>,
    pub started_at: u64,
}

impl From<&ShareSession> for SessionInfo {
    fn from(session: &ShareSession) -> Self {
        SessionInfo {
            token: session.token.clone(),
            base_path: session.base_path(),
            pin: session.pin.clone(),
            started_at: session.started_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_token_is_long_and_url_safe() {
        let session = ShareSession::new(false).expect("session");
        assert_eq!(session.token().len(), tokens::SESSION_TOKEN_LEN);
        assert!(session.token().chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(session.base_path(), format!("/s/{}", session.token()));
    }

    #[test]
    fn share_urls_use_the_selected_lan_address() {
        let session = ShareSession::new(false).expect("session");
        let url = session.share_url("192.168.1.42", 8080);
        assert!(url.starts_with("http://192.168.1.42:8080/s/"));
        assert!(!url.contains("127.0.0.1"));
        assert!(!url.contains("0.0.0.0"));
    }

    #[test]
    fn only_the_exact_token_is_accepted() {
        let session = ShareSession::new(false).expect("session");
        let token = session.token().to_string();

        assert!(session.accepts_token(&token));
        assert!(!session.accepts_token(""));
        assert!(!session.accepts_token(&token[..token.len() - 1]));
        assert!(!session.accepts_token(&format!("{token}x")));
        assert!(!session.accepts_token(&token.to_uppercase()));
        assert!(!session.accepts_token("../../etc/passwd"));
    }

    #[test]
    fn each_session_gets_a_different_token() {
        let a = ShareSession::new(false).expect("a");
        let b = ShareSession::new(false).expect("b");
        assert_ne!(a.token(), b.token());
        assert_ne!(a.unlock_secret(), b.unlock_secret());
        assert!(!a.accepts_token(b.token()));
    }

    #[test]
    fn sessions_without_a_pin_are_always_unlocked() {
        let session = ShareSession::new(false).expect("session");
        assert!(!session.requires_pin());
        assert!(session.pin().is_none());
        assert!(session.is_unlocked(None));
        assert!(session.is_unlocked(Some("anything")));
    }

    #[test]
    fn sessions_with_a_pin_need_the_right_pin_then_the_right_cookie() {
        let session = ShareSession::new(true).expect("session");
        let pin = session.pin().expect("pin").to_string();
        assert_eq!(pin.len(), PIN_DIGITS);

        assert!(session.requires_pin());
        assert!(!session.is_unlocked(None));
        assert!(!session.is_unlocked(Some("guess")));

        assert!(session.accepts_pin(&pin));
        assert!(
            session.accepts_pin(&format!("  {pin} ")),
            "surrounding space is tolerated"
        );
        assert!(!session.accepts_pin("000000000"));
        assert!(!session.accepts_pin(""));

        assert!(session.is_unlocked(Some(session.unlock_secret())));
    }

    #[test]
    fn session_info_never_carries_the_unlock_secret() {
        let session = ShareSession::new(true).expect("session");
        let info = SessionInfo::from(&session);
        let json = serde_json::to_string(&info).expect("json");
        assert!(json.contains(session.token()));
        assert!(!json.contains(session.unlock_secret()));
    }
}
