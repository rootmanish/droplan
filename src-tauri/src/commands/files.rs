//! Shared-file registry commands.
//!
//! `add_shared_files` takes paths, which is the one place the webview names a
//! filesystem location. Those paths come from a native drag-drop or the OS
//! file picker, both of which are user gestures, and each one is canonicalised
//! and confirmed to be a readable regular file before it is accepted.

use std::path::PathBuf;

use crate::error::Result;
use crate::sharing::{AddOutcome, SharedFilesPayload};

use super::Shared;

#[tauri::command]
pub async fn add_shared_files(state: Shared<'_>, paths: Vec<String>) -> Result<AddOutcome> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    state.add_files(&paths)
}

#[tauri::command]
pub async fn remove_shared_file(state: Shared<'_>, id: String) -> Result<bool> {
    state.remove_file(&id)
}

#[tauri::command]
pub async fn clear_shared_files(state: Shared<'_>) -> Result<usize> {
    state.clear_files()
}

/// Re-check that every shared file is still where it was.
#[tauri::command]
pub async fn refresh_shared_files(state: Shared<'_>) -> Result<SharedFilesPayload> {
    state.refresh_files()?;
    Ok(state.files_payload())
}

#[tauri::command]
pub async fn get_shared_files(state: Shared<'_>) -> Result<SharedFilesPayload> {
    Ok(state.files_payload())
}
