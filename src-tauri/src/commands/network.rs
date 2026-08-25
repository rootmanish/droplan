//! Network inspection and interface selection.

use crate::app::state::ShareState;
use crate::error::Result;
use crate::network::{NetworkInterface, NetworkSnapshot};
use crate::platform::{self, PlatformNotice};

use super::Shared;

#[tauri::command]
pub async fn get_network_interfaces(state: Shared<'_>) -> Result<Vec<NetworkInterface>> {
    Ok(state.network().interfaces)
}

#[tauri::command]
pub async fn get_current_network(state: Shared<'_>) -> Result<NetworkSnapshot> {
    Ok(state.network())
}

/// Force a fresh detection pass, e.g. after the user plugged in a cable.
#[tauri::command]
pub async fn refresh_network(state: Shared<'_>) -> Result<NetworkSnapshot> {
    Ok(state.refresh_network())
}

/// Pin an interface, or pass `null` to go back to automatic selection.
#[tauri::command]
pub async fn set_preferred_interface(
    state: Shared<'_>,
    name: Option<String>,
) -> Result<ShareState> {
    state.update_settings(|settings| settings.preferred_interface = name)?;
    let state = state.inner().clone();
    Ok(state.apply_interface_change().await)
}

/// Change the preferred port. Rebinds immediately when sharing is running.
#[tauri::command]
pub async fn set_preferred_port(state: Shared<'_>, port: u16) -> Result<ShareState> {
    state.update_settings(|settings| settings.preferred_port = port)?;
    let state = state.inner().clone();
    state.restart_server().await
}

#[tauri::command]
pub async fn get_platform_notice() -> Result<PlatformNotice> {
    Ok(platform::firewall_notice())
}
