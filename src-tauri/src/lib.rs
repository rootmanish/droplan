//! DropLAN — drop files, and share them with any device on your local network.
//!
//! ```text
//!   Desktop UI (React)                Application core (this crate)
//!   ──────────────────                ─────────────────────────────
//!   drag & drop        ── command ──▶ sharing::registry   opaque id ─┐
//!   file list          ◀── event ───  events::EventBus              │
//!   QR / share URL                    network::interfaces           │
//!                                     server (axum)  ◀── LAN ───────┘
//! ```
//!
//! Module map:
//!
//! - [`network`]  — which address other devices should use, and noticing when
//!   that answer changes.
//! - [`server`]   — the embedded axum listener, its routes and byte streaming.
//! - [`sharing`]  — what is shared (the registry) and the secret that makes it
//!   reachable (the session).
//! - [`security`] — token generation and path/filename hygiene.
//! - [`transfer`] — live download and client bookkeeping.
//! - [`settings`] — the small set of preferences that survive a restart.
//! - [`platform`] — every `cfg(target_os)` in the project.
//! - [`app`]      — the Tauri shell; the only module that knows about Tauri.
//!
//! Everything except [`app`] and [`commands`] is free of desktop-framework
//! dependencies, which is what would make a headless or mobile front end
//! possible later without redesigning the core.

pub mod app;
pub mod commands;
pub mod error;
pub mod events;
pub mod network;
pub mod platform;
pub mod security;
pub mod server;
pub mod settings;
pub mod sharing;
pub mod transfer;

pub use app::run;
pub use error::{Error, Result};
