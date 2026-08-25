//! System tray icon and menu.
//!
//! Convenience only. Everything the tray offers is also reachable from the
//! main window, so an environment without a working tray (some minimal Linux
//! desktops) loses nothing essential.

use std::sync::Arc;

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::app::state::AppState;
use crate::platform;

pub const MENU_OPEN: &str = "tray_open";
pub const MENU_START: &str = "tray_start";
pub const MENU_STOP: &str = "tray_stop";
pub const MENU_COPY: &str = "tray_copy";
pub const MENU_QUIT: &str = "tray_quit";

/// Menu entries whose enabled state follows the sharing status.
pub struct TrayHandles<R: Runtime> {
    pub start: MenuItem<R>,
    pub stop: MenuItem<R>,
    pub copy: MenuItem<R>,
}

impl<R: Runtime> TrayHandles<R> {
    pub fn set_sharing(&self, sharing: bool) {
        let _ = self.start.set_enabled(!sharing);
        let _ = self.stop.set_enabled(sharing);
        let _ = self.copy.set_enabled(sharing);
    }
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Open DropLAN", true, None::<&str>)?;
    let start = MenuItem::with_id(app, MENU_START, "Start sharing", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, MENU_STOP, "Stop sharing", false, None::<&str>)?;
    let copy = MenuItem::with_id(app, MENU_COPY, "Copy share URL", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit DropLAN", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&open, &separator, &start, &stop, &copy, &separator2, &quit],
    )?;

    app.manage(TrayHandles {
        start: start.clone(),
        stop: stop.clone(),
        copy: copy.clone(),
    });

    let mut builder = TrayIconBuilder::with_id("droplan")
        .tooltip("DropLAN — share files over your LAN")
        .menu(&menu)
        // On Windows and Linux a left click should open the window; the menu
        // stays on right click, which is the platform convention.
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                show_main_window(tray.app_handle());
            }
        });

    let asset = platform::tray_icon();
    if let Ok(icon) = tauri::image::Image::from_bytes(asset.bytes) {
        builder = builder.icon(icon).icon_as_template(asset.is_template);
    } else if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_OPEN => show_main_window(app),
        MENU_START => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    if let Err(err) = state.start_sharing().await {
                        tracing::warn!(target: "droplan", "tray start failed: {err}");
                        crate::app::publish_error(&app, &err);
                    }
                }
            });
        }
        MENU_STOP => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let _ = state.stop_sharing().await;
                }
            });
        }
        MENU_COPY => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = app.try_state::<Arc<AppState>>() else {
                    return;
                };
                if let Some(url) = state.share_state().await.share_url {
                    if let Err(err) = app.clipboard().write_text(url) {
                        tracing::warn!(target: "droplan", "clipboard write failed: {err}");
                    }
                }
            });
        }
        MENU_QUIT => {
            // `exit` triggers RunEvent::Exit, where sharing is torn down.
            app.exit(0);
        }
        other => tracing::debug!(target: "droplan", "unhandled tray item {other}"),
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Reflect the sharing state in the tray menu.
pub fn sync_sharing_state<R: Runtime>(app: &AppHandle<R>, sharing: bool) {
    if let Some(handles) = app.try_state::<TrayHandles<R>>() {
        handles.set_sharing(sharing);
    }
}
