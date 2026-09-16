use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::claude;
use crate::providers;
use crate::settings;
use crate::state::AppState;
use crate::types::{AppSettings, Diagnostics, ProviderId, ProviderQuota, UsageSnapshot};
use crate::window;

#[tauri::command]
pub fn get_usage_snapshot(app: AppHandle) -> UsageSnapshot {
    app.state::<AppState>().usage.lock().unwrap().clone()
}

#[tauri::command]
pub async fn refresh_provider(app: AppHandle, provider: ProviderId) -> ProviderQuota {
    let quota = match provider {
        ProviderId::Codex => providers::codex::refresh(&app).await,
        ProviderId::Claude => providers::claude::refresh(claude_integration_enabled(&app)),
    };

    let state = app.state::<AppState>();
    let mut usage = state.usage.lock().unwrap();
    match provider {
        ProviderId::Codex => usage.codex = quota.clone(),
        ProviderId::Claude => usage.claude = quota.clone(),
    }
    quota
}

#[tauri::command]
pub async fn refresh_all(app: AppHandle) -> UsageSnapshot {
    refresh_all_and_emit(&app).await
}

/// Shared by the `refresh_all` command and the tray's "Refresh" menu item.
pub async fn refresh_all_and_emit(app: &AppHandle) -> UsageSnapshot {
    let codex = providers::codex::refresh(app).await;
    let claude = providers::claude::refresh(claude_integration_enabled(app));

    let snapshot = {
        let state = app.state::<AppState>();
        let mut usage = state.usage.lock().unwrap();
        usage.codex = codex;
        usage.claude = claude;
        usage.clone()
    };
    let _ = app.emit("usage://updated", snapshot.clone());
    snapshot
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppSettings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

#[tauri::command]
pub async fn set_start_with_windows(app: AppHandle, enabled: bool) -> Result<AppSettings, String> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|err| err.to_string())?;

    let settings = {
        let state = app.state::<AppState>();
        let mut settings = state.settings.lock().unwrap();
        settings.start_with_windows = enabled;
        settings.clone()
    };
    settings::save(&app, &settings)?;
    Ok(settings)
}

/// Shared with the tray's "Start with Windows" menu item, which has no
/// boolean argument of its own and just flips the current value.
pub async fn toggle_start_with_windows(app: &AppHandle) -> Result<AppSettings, String> {
    let current = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .start_with_windows;
    set_start_with_windows(app.clone(), !current).await
}

#[tauri::command]
pub async fn enable_claude_integration(app: AppHandle) -> Result<AppSettings, String> {
    set_claude_integration(app, true).await
}

#[tauri::command]
pub async fn disable_claude_integration(app: AppHandle) -> Result<AppSettings, String> {
    set_claude_integration(app, false).await
}

/// Installs (or removes) the widget as Claude Code's status-line command, which
/// is the only supported way to read subscription limits locally.
async fn set_claude_integration(app: AppHandle, enabled: bool) -> Result<AppSettings, String> {
    let previous = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .claude_previous_status_line
        .clone();

    let previous = if enabled {
        claude::install::install()?
    } else {
        claude::install::uninstall(previous.as_deref())?;
        None
    };

    let settings = {
        let state = app.state::<AppState>();
        let mut settings = state.settings.lock().unwrap();
        settings.claude_integration_enabled = enabled;
        // Written before the bridge can run: it reads this file to find the
        // command it has to forward to.
        settings.claude_previous_status_line = previous;
        settings.clone()
    };
    settings::save(&app, &settings)?;

    // Re-read the provider so the card reflects the new state immediately
    // rather than after the next poll.
    refresh_all_and_emit(&app).await;
    Ok(settings)
}

fn claude_integration_enabled(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .claude_integration_enabled
}

#[tauri::command]
pub async fn get_diagnostics(app: AppHandle) -> Diagnostics {
    let codex_path = providers::codex::detect_path();
    let codex_version = match &codex_path {
        Some(path) => providers::detect_version(path).await,
        None => None,
    };
    let codex_status = app.state::<AppState>().usage.lock().unwrap().codex.status;
    let codex_app_server_status = providers::codex::app_server_status(&app).await.to_string();

    let claude_path = providers::claude::detect_path();
    let claude_version = match &claude_path {
        Some(path) => providers::detect_version(path).await,
        None => None,
    };
    let claude_integration_enabled = claude_integration_enabled(&app);
    let claude_cache_updated_at = claude::cache::load().map(|cache| cache.updated_at);
    let claude_settings_path = claude::install::settings_path().map(|p| p.display().to_string());

    Diagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        codex_path: codex_path.map(|p| p.display().to_string()),
        codex_version,
        codex_status,
        codex_app_server_status,
        claude_path: claude_path.map(|p| p.display().to_string()),
        claude_version,
        claude_integration_enabled,
        claude_cache_updated_at,
        claude_settings_path,
    }
}

#[tauri::command]
pub fn hide_popup(app: AppHandle) {
    window::hide_popup(&app);
}

#[tauri::command]
pub fn begin_popup_drag(app: AppHandle) -> Result<(), String> {
    let window = window::popup(&app).ok_or_else(|| "main window not found".to_string())?;
    // Must be armed *before* the native drag starts: Windows deactivates the
    // window as soon as the move loop begins, which would otherwise trip the
    // "hide on blur" rule and close the popup the user is trying to move.
    window::suppress_blur(&app);
    window.start_dragging().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn resize_popup(app: AppHandle, height: f64) -> Result<(), String> {
    window::resize_to_content(&app, height).map(|_| ())
}
