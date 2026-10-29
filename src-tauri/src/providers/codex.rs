//! Client for OpenAI Codex's `codex app-server` JSON-RPC-over-stdio protocol.
//!
//! Verified live against `codex-cli 0.154.0` (see the repo's implementation
//! notes): messages are newline-delimited JSON objects with no `"jsonrpc"`
//! field, requests carry a numeric `id`, notifications don't. Responses can
//! arrive out of order, so pending requests are correlated by id rather than
//! by send order.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::{oneshot, Mutex as TokioMutex};

use crate::state::AppState;
use crate::types::{ProviderId, ProviderQuota, ProviderStatus, QuotaWindow};

use super::{find_executable, quiet_command};

type PendingMap = Arc<StdMutex<HashMap<i64, oneshot::Sender<Value>>>>;

pub struct Session {
    generation: u64,
    stdin: ChildStdin,
    next_id: i64,
    pending: PendingMap,
    // Keeping the child around (and killing it on drop) ties the app-server's
    // lifetime to this session instead of leaking a background process.
    _child: Child,
}

#[derive(Default)]
pub struct CodexState {
    session: TokioMutex<Option<Session>>,
    next_generation: AtomicU64,
}

enum SessionStartError {
    Disabled,
    MissingExecutable,
    Other(String),
}

pub fn detect_path() -> Option<PathBuf> {
    find_executable("codex").or_else(default_install_path)
}

/// The Codex desktop installer places its CLI in this stable per-user folder,
/// but a tray app can keep running with an older PATH after Codex is updated.
/// Return the candidate even if metadata lookup fails: spawning it gives us
/// the authoritative result and avoids reporting an installed CLI as absent.
fn default_install_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let path = PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
            .join("Programs")
            .join("OpenAI")
            .join("Codex")
            .join("bin")
            .join("codex.exe");
        Some(path)
    }

    #[cfg(not(windows))]
    None
}

/// Fetches a fresh rate-limit snapshot, starting the app-server session if
/// it isn't already running (or restarting it if the previous one died).
pub async fn refresh(app: &AppHandle) -> ProviderQuota {
    let Some(path) = detect_path() else {
        return ProviderQuota::not_installed(ProviderId::Codex);
    };

    match ensure_session(app, &path).await {
        Ok(()) => {}
        Err(SessionStartError::Disabled) => return ProviderQuota::disabled(ProviderId::Codex),
        Err(SessionStartError::MissingExecutable) => {
            return ProviderQuota::not_installed(ProviderId::Codex)
        }
        Err(SessionStartError::Other(message)) => return error_quota(message),
    }

    let state = app.state::<AppState>();
    let (generation, result) = {
        let mut guard = state.codex.session.lock().await;
        if !state.settings.lock().unwrap().monitoring.codex {
            return ProviderQuota::disabled(ProviderId::Codex);
        }
        match guard.as_mut() {
            Some(session) => (
                Some(session.generation),
                request(session, "account/rateLimits/read", json!({})).await,
            ),
            None => (
                None,
                Err("codex app-server session unavailable".to_string()),
            ),
        }
    };

    match result {
        Ok(value) => match serde_json::from_value::<RateLimitsResponse>(value) {
            Ok(response) => quota_from_snapshot(response.rate_limits),
            Err(err) => error_quota(format!("unexpected codex response: {err}")),
        },
        Err(message) => {
            // The session may have died mid-request; drop it so the next
            // refresh respawns the app-server instead of retrying a dead pipe.
            let mut guard = state.codex.session.lock().await;
            if guard.as_ref().map(|session| session.generation) == generation {
                *guard = None;
            }
            error_quota(message)
        }
    }
}

pub async fn app_server_status(app: &AppHandle) -> &'static str {
    let state = app.state::<AppState>();
    if state.codex.session.lock().await.is_some() {
        "running"
    } else {
        "not started"
    }
}

pub async fn stop(app: &AppHandle) {
    *app.state::<AppState>().codex.session.lock().await = None;
}

async fn ensure_session(app: &AppHandle, path: &std::path::Path) -> Result<(), SessionStartError> {
    let state = app.state::<AppState>();
    let mut guard = state.codex.session.lock().await;
    if !state.settings.lock().unwrap().monitoring.codex {
        return Err(SessionStartError::Disabled);
    }
    if guard.is_some() {
        return Ok(());
    }

    let mut child = quiet_command(path)
        .arg("app-server")
        .arg("--stdio")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                SessionStartError::MissingExecutable
            } else {
                SessionStartError::Other(format!("failed to start codex app-server: {err}"))
            }
        })?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| SessionStartError::Other("codex app-server has no stdin".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SessionStartError::Other("codex app-server has no stdout".to_string()))?;

    let pending: PendingMap = Arc::new(StdMutex::new(HashMap::new()));
    let generation = state.codex.next_generation.fetch_add(1, Ordering::Relaxed);
    spawn_reader(app.clone(), stdout, pending.clone(), generation);

    let mut session = Session {
        generation,
        stdin,
        next_id: 0,
        pending,
        _child: child,
    };

    request(
        &mut session,
        "initialize",
        json!({
            "clientInfo": {
                "name": "ai-usage-widget",
                "title": "AI Usage Widget",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }),
    )
    .await
    .map_err(SessionStartError::Other)?;
    notify(&mut session, "initialized", None)
        .await
        .map_err(SessionStartError::Other)?;

    *guard = Some(session);
    Ok(())
}

/// Reads newline-delimited JSON messages from the app-server for as long as
/// it stays alive, resolving pending requests by id and forwarding
/// `account/rateLimits/updated` push notifications straight into app state.
fn spawn_reader(
    app: AppHandle,
    stdout: tokio::process::ChildStdout,
    pending: PendingMap,
    generation: u64,
) {
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            handle_message(&app, &pending, value, generation).await;
        }

        // The app-server process exited; clear the session so the next
        // refresh spawns a new one instead of writing into a dead pipe.
        pending.lock().unwrap().clear();
        let state = app.state::<AppState>();
        let mut guard = state.codex.session.lock().await;
        // An older reader can finish after monitoring has already been
        // re-enabled. It must not clear the new app-server session.
        if guard
            .as_ref()
            .is_some_and(|session| session.generation == generation)
        {
            *guard = None;
        }
    });
}

async fn handle_message(app: &AppHandle, pending: &PendingMap, value: Value, generation: u64) {
    if let Some(id) = value.get("id").and_then(Value::as_i64) {
        if let Some(sender) = pending.lock().unwrap().remove(&id) {
            let _ = sender.send(value);
        }
        return;
    }

    if value.get("method").and_then(Value::as_str) == Some("account/rateLimits/updated") {
        if let Some(params) = value.get("params") {
            if let Ok(notification) =
                serde_json::from_value::<RateLimitsUpdatedParams>(params.clone())
            {
                let quota = quota_from_snapshot(notification.rate_limits);
                apply_and_emit(app, quota, generation);
            }
        }
    }
}

fn apply_and_emit(app: &AppHandle, quota: ProviderQuota, generation: u64) {
    let state = app.state::<AppState>();
    let snapshot = {
        let preferences = state.settings.lock().unwrap();
        if !preferences.monitoring.codex
            || state.codex.next_generation.load(Ordering::Relaxed) != generation.wrapping_add(1)
        {
            return;
        }
        let mut usage = state.usage.lock().unwrap();
        usage.codex = quota;
        usage.clone()
    };
    let _ = app.emit("usage://updated", snapshot);
}

async fn request(session: &mut Session, method: &str, params: Value) -> Result<Value, String> {
    let id = session.next_id;
    session.next_id += 1;

    let (tx, rx) = oneshot::channel();
    session.pending.lock().unwrap().insert(id, tx);

    let payload = json!({ "id": id, "method": method, "params": params });
    if let Err(err) = write_line(&mut session.stdin, &payload).await {
        session.pending.lock().unwrap().remove(&id);
        return Err(err);
    }

    let response = tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .map_err(|_| format!("codex app-server timed out waiting for '{method}'"))?
        .map_err(|_| "codex app-server closed the connection".to_string())?;

    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("codex app-server returned an error");
        return Err(message.to_string());
    }

    Ok(response.get("result").cloned().unwrap_or(Value::Null))
}

async fn notify(session: &mut Session, method: &str, params: Option<Value>) -> Result<(), String> {
    let payload = match params {
        Some(params) => json!({ "method": method, "params": params }),
        None => json!({ "method": method }),
    };
    write_line(&mut session.stdin, &payload).await
}

async fn write_line(stdin: &mut ChildStdin, payload: &Value) -> Result<(), String> {
    let mut line = serde_json::to_string(payload).map_err(|err| err.to_string())?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|err| format!("failed to write to codex app-server: {err}"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsResponse {
    rate_limits: RateLimitSnapshotJson,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitsUpdatedParams {
    rate_limits: RateLimitSnapshotJson,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitSnapshotJson {
    primary: Option<RateLimitWindowJson>,
    secondary: Option<RateLimitWindowJson>,
    plan_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitWindowJson {
    used_percent: f64,
    window_duration_mins: Option<i64>,
    resets_at: Option<i64>,
}

/// Codex reports arbitrary windows identified only by their duration, not by
/// a fixed "primary is always 5h" contract (per `plan.md`), so windows are
/// bucketed by how long they are rather than by their `primary`/`secondary`
/// position in the response.
fn quota_from_snapshot(snapshot: RateLimitSnapshotJson) -> ProviderQuota {
    let mut five_hour = None;
    let mut weekly = None;

    for window in [snapshot.primary, snapshot.secondary].into_iter().flatten() {
        let mapped = QuotaWindow {
            used_percentage: window.used_percent,
            remaining_percentage: (100.0 - window.used_percent).clamp(0.0, 100.0),
            resets_at: window.resets_at,
            duration_minutes: window.window_duration_mins,
        };
        match window.window_duration_mins {
            Some(minutes) if minutes <= 360 => five_hour = Some(mapped),
            Some(_) => weekly = Some(mapped),
            None => {}
        }
    }

    ProviderQuota {
        provider: ProviderId::Codex,
        status: ProviderStatus::Ready,
        five_hour,
        weekly,
        plan: snapshot.plan_type,
        buckets: Vec::new(),
        updated_at: Some(now()),
        source: "codex-app-server".into(),
        error: None,
    }
}

fn error_quota(message: String) -> ProviderQuota {
    let lower = message.to_lowercase();
    let status = if lower.contains("auth") || lower.contains("login") || lower.contains("sign in") {
        ProviderStatus::NotAuthenticated
    } else {
        ProviderStatus::Error
    };
    ProviderQuota {
        provider: ProviderId::Codex,
        status,
        five_hour: None,
        weekly: None,
        plan: None,
        updated_at: None,
        source: "codex-app-server".into(),
        error: Some(message),
        buckets: Vec::new(),
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
