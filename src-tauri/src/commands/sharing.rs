//! Sharing lifecycle and status commands.

use crate::app::state::ShareState;
use crate::error::Result;
use crate::sharing::qr;
use crate::transfer::ActivitySnapshot;

use super::Shared;

#[tauri::command]
pub async fn get_share_state(state: Shared<'_>) -> Result<ShareState> {
    Ok(state.share_state().await)
}

#[tauri::command]
pub async fn start_sharing(state: Shared<'_>) -> Result<ShareState> {
    let state = state.inner().clone();
    state.start_sharing().await
}

#[tauri::command]
pub async fn stop_sharing(state: Shared<'_>) -> Result<ShareState> {
    state.stop_sharing().await?;
    Ok(state.share_state().await)
}

#[tauri::command]
pub async fn regenerate_share_session(state: Shared<'_>) -> Result<ShareState> {
    let state = state.inner().clone();
    state.regenerate_session().await
}

/// SVG for the given URL. The frontend inlines it, so it inherits the theme.
#[tauri::command]
pub async fn get_qr_svg(url: String) -> Result<String> {
    qr::render_svg(&url)
}

#[tauri::command]
pub async fn get_transfer_activity(state: Shared<'_>) -> Result<ActivitySnapshot> {
    Ok(state.activity())
}
