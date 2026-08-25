//! Preference commands.

use crate::app::state::ShareState;
use crate::error::Result;
use crate::settings::AppSettings;

use super::Shared;

#[tauri::command]
pub async fn get_settings(state: Shared<'_>) -> Result<AppSettings> {
    Ok(state.settings.get())
}

/// Replace the whole settings object.
///
/// Changing the port or the PIN requirement has to reach the running server,
/// so this rebinds when it needs to rather than waiting for a restart.
#[tauri::command]
pub async fn update_settings(state: Shared<'_>, settings: AppSettings) -> Result<ShareState> {
    let previous = state.settings.get();
    let applied = state.settings.replace(settings)?;
    let shared = state.inner().clone();

    let port_changed = previous.preferred_port != applied.preferred_port;
    let mdns_changed = previous.enable_mdns != applied.enable_mdns;
    let pin_changed = previous.require_pin != applied.require_pin;

    if pin_changed {
        // The PIN lives in the session, so it takes a new session to apply.
        return shared.regenerate_session().await;
    }
    if port_changed || mdns_changed {
        return shared.restart_server().await;
    }
    Ok(shared.share_state().await)
}
