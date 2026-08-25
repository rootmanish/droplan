//! The Tauri shell.
//!
//! This is the only module that knows Tauri exists. It owns the window, the
//! tray, the command surface and the bridge that turns core events into
//! webview events; all behaviour lives in [`state::AppState`].

pub mod state;
pub mod tray;

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime, WindowEvent};

use crate::commands;
use crate::error::Error;
use crate::events::names;
use crate::platform;

use state::AppState;

/// Build and run the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup)
        .on_window_event(on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::sharing::get_share_state,
            commands::sharing::start_sharing,
            commands::sharing::stop_sharing,
            commands::sharing::regenerate_share_session,
            commands::sharing::get_qr_svg,
            commands::sharing::get_transfer_activity,
            commands::network::get_network_interfaces,
            commands::network::get_current_network,
            commands::network::refresh_network,
            commands::network::set_preferred_interface,
            commands::network::set_preferred_port,
            commands::network::get_platform_notice,
            commands::files::add_shared_files,
            commands::files::remove_shared_file,
            commands::files::clear_shared_files,
            commands::files::refresh_shared_files,
            commands::files::get_shared_files,
            commands::settings::get_settings,
            commands::settings::update_settings,
        ])
        .build(tauri::generate_context!())
        .expect("DropLAN could not start")
        .run(on_run_event);
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_env("DROPLAN_LOG")
        .unwrap_or_else(|_| EnvFilter::new("droplan=info,tower_http=warn,warn"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false))
        .try_init();
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    let config_dir = handle
        .path()
        .app_config_dir()
        .map_err(|err| format!("no writable config directory: {err}"))?;
    std::fs::create_dir_all(&config_dir)
        .map_err(|err| format!("could not create {}: {err}", config_dir.display()))?;

    let state = AppState::new(&config_dir)?;
    app.manage(Arc::clone(&state));

    if let Err(err) = tray::build(&handle) {
        // A missing tray is a degraded experience, never a failure to launch.
        tracing::warn!(target: "droplan", "system tray unavailable: {err}");
    }

    spawn_event_bridge(handle.clone(), Arc::clone(&state));
    spawn_startup(handle, state);
    Ok(())
}

/// Forward core events to the webview, and keep the tray in step.
fn spawn_event_bridge(app: AppHandle, state: Arc<AppState>) {
    let mut receiver = state.events.subscribe();

    tauri::async_runtime::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    // Read the tray state out of the payload rather than
                    // inferring it from the event name: both `sharing-started`
                    // and `network-changed` carry a full ShareState, and its
                    // `sharing` flag is the only thing that is always right.
                    match event.name {
                        names::SHARING_STARTED | names::NETWORK_CHANGED => {
                            if let Some(sharing) = event
                                .payload
                                .get("sharing")
                                .and_then(|value| value.as_bool())
                            {
                                tray::sync_sharing_state(&app, sharing);
                            }
                        }
                        names::SHARING_STOPPED => tray::sync_sharing_state(&app, false),
                        _ => {}
                    }
                    if let Err(err) = app.emit(event.name, event.payload) {
                        tracing::debug!(target: "droplan", "could not emit {}: {err}", event.name);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    // The UI fell behind during a burst. It re-reads full state
                    // on the next event, so dropping a few is safe.
                    tracing::debug!(target: "droplan", "ui event bridge lagged by {skipped}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// First-run work: warm platform caches, detect the network, optionally start
/// sharing. Kept off the main thread so the window paints immediately.
fn spawn_startup(app: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        // `networksetup` on macOS is a subprocess; never block the UI on it.
        let _ = tauri::async_runtime::spawn_blocking(platform::prime_caches).await;

        state.refresh_network();
        state.start_network_watcher().await;

        let settings = state.settings.get();
        if settings.start_sharing_on_launch {
            match state.start_sharing().await {
                Ok(_) => tracing::info!(target: "droplan", "sharing started on launch"),
                Err(err) => {
                    tracing::warn!(target: "droplan", "could not start sharing on launch: {err}");
                    publish_error(&app, &err);
                }
            }
        }

        // Push the initial picture even when sharing did not start.
        let snapshot = state.share_state().await;
        let _ = app.emit(names::NETWORK_CHANGED, snapshot);
    });
}

/// Surface a recoverable failure in the UI without turning it into a panic.
pub fn publish_error<R: Runtime>(app: &AppHandle<R>, error: &Error) {
    let _ = app.emit(
        names::NOTICE,
        serde_json::json!({ "code": error.code(), "message": error.to_string() }),
    );
}

fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let app = window.app_handle().clone();
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };

    if state.settings.get().close_to_tray {
        // Keep serving, but only because the user asked for it: the tray icon
        // stays visible so the running server is never invisible.
        api.prevent_close();
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.hide();
        }
        return;
    }

    // Default: closing the window ends the session. Hold the close until the
    // listener is actually down, so no socket outlives the window.
    api.prevent_close();
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn(async move {
        state.shutdown().await;
        app.exit(0);
    });
}

fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { code, api, .. } => {
            // `code: None` means the last window went away. With "keep sharing
            // in the tray" on, that must not end the process.
            //
            // An explicit exit — tray Quit, or closing the window with the
            // setting off — always carries a code and is always honoured;
            // preventing it here would make Quit do nothing.
            if code.is_none() {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    if state.settings.get().close_to_tray {
                        api.prevent_exit();
                    }
                }
            }
        }
        RunEvent::Exit => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let state = Arc::clone(&state);
                // Last chance to release the port and the mDNS name.
                tauri::async_runtime::block_on(async move { state.shutdown().await });
            }
        }
        _ => {}
    }
}
