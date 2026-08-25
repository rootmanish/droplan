//! Security primitives: unguessable identifiers and safe path/name handling.
//!
//! The threat model is a private LAN that is not fully trusted: other people
//! may be on the same Wi-Fi. Everything reachable over HTTP therefore sits
//! behind a high-entropy session token, and no request may ever influence
//! which path on disk is opened.

pub mod paths;
pub mod tokens;
