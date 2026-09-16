#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod claude;
mod commands;
mod providers;
mod settings;
mod state;
mod tray;
mod types;
mod window;

use std::time::Duration;

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use state::AppState;

/// Floor for the configured refresh interval, so a bad settings file can't turn
/// the widget into a busy loop spawning `codex app-server` requests.
const MIN_REFRESH_SECONDS: u64 = 15;
const MAX_REFRESH_SECONDS: u64 = 3600;

fn main() {
    // Must come first: in bridge mode the process is a short-lived filter for
    // Claude Code's status line and must never build a window or a tray icon.
    if claude::bridge::run_if_requested() {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show_popup_default(app);
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            let handle = app.handle().clone();

            let loaded_settings = settings::load(&handle);
            app.manage(AppState::new(loaded_settings));

            tray::setup(&handle)?;

            if let Some(window) = app.get_webview_window("main") {
                let event_handle = handle.clone();
                window.on_window_event(move |event| match event {
                    // Alt+F4 / a taskbar "close" request should hide the
                    // widget like clicking the tray icon again, not quit
                    // the whole app (there's no titlebar close button since
                    // the window is undecorated).
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        window::hide_popup(&event_handle);
                    }
                    WindowEvent::Focused(false) => window::on_blur(&event_handle),
                    _ => {}
                });
            }

            spawn_refresh_loop(handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_usage_snapshot,
            commands::refresh_provider,
            commands::refresh_all,
            commands::get_settings,
            commands::set_start_with_windows,
            commands::enable_claude_integration,
            commands::disable_claude_integration,
            commands::get_diagnostics,
            commands::hide_popup,
            commands::begin_popup_drag,
            commands::resize_popup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Keeps provider data fresh on its own. Without this the widget only ever
/// showed whatever the single startup refresh produced, so the popup looked
/// empty until the user pressed "Refresh" by hand.
fn spawn_refresh_loop(handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            commands::refresh_all_and_emit(&handle).await;

            let interval = {
                let state = handle.state::<AppState>();
                let seconds = state.settings.lock().unwrap().refresh_interval_seconds;
                seconds.clamp(MIN_REFRESH_SECONDS, MAX_REFRESH_SECONDS)
            };
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });
}
