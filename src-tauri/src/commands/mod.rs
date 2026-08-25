//! The typed boundary between React and Rust.
//!
//! Every command is narrow and named for an intent. There is deliberately no
//! generic "read this path" or "run this" command: the webview can add files
//! the user chose, and nothing else.

pub mod files;
pub mod network;
pub mod settings;
pub mod sharing;

use std::sync::Arc;

use tauri::State;

use crate::app::state::AppState;

/// Shorthand for the managed state every command receives.
pub type Shared<'a> = State<'a, Arc<AppState>>;
