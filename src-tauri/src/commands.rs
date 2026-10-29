use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::providers;
use crate::settings;
use crate::state::AppState;
use crate::types::{AppSettings, Diagnostics, ProviderId, ProviderQuota, UsageSnapshot};
use crate::window;
use crate::{antigravity, claude};

#[tauri::command]
pub fn get_usage_snapshot(app: AppHandle) -> UsageSnapshot {
    app.state::<AppState>().usage.lock().unwrap().clone()
}

#[tauri::command]
pub async fn refresh_provider(app: AppHandle, provider: ProviderId) -> ProviderQuota {
    let preferences = app.state::<AppState>().settings.lock().unwrap().clone();
    let quota = if !preferences.monitoring.enabled(provider) {
        ProviderQuota::disabled(provider)
    } else {
        match provider {
            ProviderId::Codex => providers::codex::refresh(&app).await,
            ProviderId::Claude => {
                providers::claude::refresh(preferences.claude_integration_enabled)
            }
            ProviderId::Antigravity => {
                providers::antigravity::refresh(preferences.antigravity_integration_enabled)
            }
        }
    };

    let state = app.state::<AppState>();
    // A request may finish after the user has disabled its provider.
    let preferences = state.settings.lock().unwrap();
    let quota = if preferences.monitoring.enabled(provider) {
        quota
    } else {
        ProviderQuota::disabled(provider)
    };
    let mut usage = state.usage.lock().unwrap();
    *usage.provider_mut(provider) = quota.clone();
    quota
}

#[tauri::command]
pub async fn refresh_all(app: AppHandle) -> UsageSnapshot {
    refresh_all_and_emit(&app).await
}

/// Shared by the `refresh_all` command and the tray's "Refresh" menu item.
pub async fn refresh_all_and_emit(app: &AppHandle) -> UsageSnapshot {
    for provider in [
        ProviderId::Codex,
        ProviderId::Claude,
        ProviderId::Antigravity,
    ] {
        refresh_provider(app.clone(), provider).await;
    }
    let snapshot = app.state::<AppState>().usage.lock().unwrap().clone();
    let _ = app.emit("usage://updated", snapshot.clone());
    snapshot
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppSettings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

#[tauri::command]
pub async fn set_provider_monitoring(
    app: AppHandle,
    provider: ProviderId,
    enabled: bool,
) -> Result<AppSettings, String> {
    let (preferences, snapshot) = {
        let state = app.state::<AppState>();
        let mut current = state.settings.lock().unwrap();
        let mut next = current.clone();
        next.monitoring.set_enabled(provider, enabled);
        settings::save(&app, &next)?;
        *current = next.clone();
        let mut usage = state.usage.lock().unwrap();
        *usage.provider_mut(provider) = if enabled {
            ProviderQuota::empty(provider)
        } else {
            ProviderQuota::disabled(provider)
        };
        (next, usage.clone())
    };
    let _ = app.emit("usage://updated", snapshot);
    if !enabled && provider == ProviderId::Codex {
        providers::codex::stop(&app).await;
    }
    if enabled {
        refresh_provider(app.clone(), provider).await;
        let snapshot = app.state::<AppState>().usage.lock().unwrap().clone();
        let _ = app.emit("usage://updated", snapshot);
    }
    Ok(preferences)
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

fn antigravity_integration_enabled(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .antigravity_integration_enabled
}

/// Called by the lightweight cache poll; never spawns a CLI or queries a model.
pub fn refresh_antigravity_and_emit(app: &AppHandle) {
    let preferences = app.state::<AppState>().settings.lock().unwrap().clone();
    let quota = if preferences.monitoring.antigravity {
        providers::antigravity::refresh(preferences.antigravity_integration_enabled)
    } else {
        ProviderQuota::disabled(ProviderId::Antigravity)
    };
    let changed = {
        let state = app.state::<AppState>();
        let preferences = state.settings.lock().unwrap();
        let quota = if preferences.monitoring.antigravity {
            quota
        } else {
            ProviderQuota::disabled(ProviderId::Antigravity)
        };
        let mut usage = state.usage.lock().unwrap();
        if usage.antigravity == quota {
            return;
        }
        usage.antigravity = quota;
        usage.clone()
    };
    let _ = app.emit("usage://updated", changed);
}

#[tauri::command]
pub fn set_antigravity_integration(app: AppHandle, enabled: bool) -> Result<AppSettings, String> {
    let state = app.state::<AppState>();
    let mut current = state.settings.lock().unwrap();
    let mut next = current.clone();
    if enabled {
        if providers::antigravity::detect_path().is_none() {
            return Err("Antigravity CLI (agy) was not found.".into());
        }
        let prepared =
            antigravity::install::prepare(current.antigravity_previous_status_line.clone())?;
        next.antigravity_previous_status_line = prepared.previous.clone();
        next.antigravity_integration_enabled = true;
        // Save the backup before the CLI can invoke the bridge. If the CLI
        // settings write fails, restore the app's previous settings as well.
        settings::save(&app, &next)?;
        if let Err(error) = prepared.write() {
            return match settings::save(&app, &current) {
                Ok(()) => Err(error),
                Err(rollback) => Err(format!(
                    "{error}. Could not restore widget settings: {rollback}"
                )),
            };
        }
    } else {
        antigravity::install::uninstall(current.antigravity_previous_status_line.as_ref())?;
        next.antigravity_previous_status_line = None;
        next.antigravity_integration_enabled = false;
        settings::save(&app, &next)?;
    }
    *current = next.clone();
    drop(current);
    refresh_antigravity_and_emit(&app);
    Ok(next)
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
    let claude_integration_enabled = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .claude_integration_enabled;
    let claude_cache_updated_at = claude::cache::load().map(|cache| cache.updated_at);
    let claude_settings_path = claude::install::settings_path().map(|p| p.display().to_string());
    let antigravity_path = providers::antigravity::detect_path();
    let antigravity_version = match &antigravity_path {
        Some(path) => providers::detect_version(path).await,
        None => None,
    };

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
        antigravity_path: antigravity_path.map(|p| p.display().to_string()),
        antigravity_version,
        antigravity_integration_enabled: antigravity_integration_enabled(&app),
        antigravity_cache_updated_at: antigravity::cache::load().map(|cache| cache.updated_at),
        antigravity_settings_path: antigravity::install::settings_path()
            .map(|p| p.display().to_string()),
    }
}

#[tauri::command]
pub fn get_popup_pinned(app: AppHandle) -> bool {
    app.state::<AppState>()
        .popup_pinned
        .load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
pub fn set_popup_pinned(app: AppHandle, pinned: bool) -> Result<bool, String> {
    window::set_popup_pinned(&app, pinned)
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
