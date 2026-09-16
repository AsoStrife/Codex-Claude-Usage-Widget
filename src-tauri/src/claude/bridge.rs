//! Status-line bridge mode: `ai-usage-widget.exe --claude-statusline-bridge`.
//!
//! Claude Code runs the configured status-line command on every render and
//! hands it a JSON payload on stdin. That payload is the only supported local
//! source of subscription rate-limit state, so the widget installs itself as
//! that command, keeps the numbers, and forwards everything to whatever
//! status-line command the user already had.

use std::io::Read;
use std::path::PathBuf;

use serde::Deserialize;

use super::cache::{self, CachedWindow, ClaudeUsageCache};

pub const FLAG: &str = "--claude-statusline-bridge";

const BOM: char = '\u{feff}';

#[derive(Deserialize)]
struct StatusLinePayload {
    #[serde(default)]
    model: Option<Model>,
    #[serde(default)]
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize)]
struct Model {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct RateLimits {
    #[serde(default)]
    five_hour: Option<Window>,
    #[serde(default)]
    seven_day: Option<Window>,
}

#[derive(Deserialize)]
struct Window {
    #[serde(default)]
    used_percentage: Option<f64>,
    #[serde(default)]
    resets_at: Option<i64>,
}

/// Runs the bridge and reports whether `--claude-statusline-bridge` was passed.
/// Returns `false` for a normal launch so `main` starts the GUI instead.
///
/// Nothing here may panic or hang: it runs inside the user's status line on
/// every render.
pub fn run_if_requested() -> bool {
    if !std::env::args().any(|arg| arg == FLAG) {
        return false;
    }

    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    let payload = parse(&input);
    if let Some(snapshot) = payload.as_ref().and_then(snapshot_from) {
        let _ = cache::store(&snapshot);
    }

    // The user's previous status line takes priority: the widget is a passive
    // observer here and must not take their status bar away from them.
    match forward_to_previous(&input) {
        Some(output) if !output.trim().is_empty() => print!("{output}"),
        _ => println!("{}", fallback_line(payload.as_ref())),
    }
    true
}

fn parse(input: &str) -> Option<StatusLinePayload> {
    serde_json::from_str(input.trim_start_matches(BOM)).ok()
}

fn snapshot_from(payload: &StatusLinePayload) -> Option<ClaudeUsageCache> {
    let limits = payload.rate_limits.as_ref()?;
    let five_hour = cached_window(limits.five_hour.as_ref());
    let weekly = cached_window(limits.seven_day.as_ref());
    if five_hour.is_none() && weekly.is_none() {
        return None;
    }
    Some(ClaudeUsageCache {
        five_hour,
        weekly,
        updated_at: cache::now_seconds(),
    })
}

fn cached_window(window: Option<&Window>) -> Option<CachedWindow> {
    let window = window?;
    Some(CachedWindow {
        used_percentage: window.used_percentage?,
        resets_at: window.resets_at,
    })
}

/// Reads the command the widget replaced out of its own settings file. The
/// bridge has no Tauri state, so it reads the same JSON the app writes.
fn previous_command() -> Option<String> {
    let path: PathBuf = cache::config_dir()?.join("settings.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let command = value.get("claudePreviousStatusLine")?.as_str()?.trim();
    if command.is_empty() {
        None
    } else {
        Some(command.to_string())
    }
}

#[cfg(windows)]
fn forward_to_previous(input: &str) -> Option<String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let command = previous_command()?;
    let shell = std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string());

    // `raw_arg`, not `arg`: cmd.exe does its own command-line parsing, so Rust
    // must not re-quote the user's command string.
    let mut child = Command::new(shell)
        .raw_arg(format!("/d /s /c {command}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let output = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(not(windows))]
fn forward_to_previous(input: &str) -> Option<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let command = previous_command()?;
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let output = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// What the status line shows when the user had no command of their own.
fn fallback_line(payload: Option<&StatusLinePayload>) -> String {
    let Some(payload) = payload else {
        return "[Claude]".to_string();
    };

    let model = payload
        .model
        .as_ref()
        .and_then(|m| m.display_name.clone().or_else(|| m.id.clone()))
        .unwrap_or_else(|| "Claude".to_string());

    let mut parts: Vec<String> = Vec::new();
    if let Some(limits) = payload.rate_limits.as_ref() {
        if let Some(used) = limits.five_hour.as_ref().and_then(|w| w.used_percentage) {
            parts.push(format!("5h: {used:.0}%"));
        }
        if let Some(used) = limits.seven_day.as_ref().and_then(|w| w.used_percentage) {
            parts.push(format!("7d: {used:.0}%"));
        }
    }

    if parts.is_empty() {
        format!("[{model}]")
    } else {
        format!("[{model}] {}", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a real Claude Code 2.1.273 status-line invocation.
    const REAL_PAYLOAD: &str = r#"{
      "session_id": "688a67cb",
      "model": { "id": "claude-haiku-4-5-20251001", "display_name": "Haiku 4.5" },
      "version": "2.1.273",
      "context_window": { "used_percentage": null },
      "rate_limits": {
        "five_hour": { "used_percentage": 36, "resets_at": 1789561800 },
        "seven_day": { "used_percentage": 25, "resets_at": 1789862400 }
      }
    }"#;

    #[test]
    fn extracts_both_windows_from_a_real_payload() {
        let snapshot = snapshot_from(&parse(REAL_PAYLOAD).unwrap()).unwrap();
        let five = snapshot.five_hour.unwrap();
        let weekly = snapshot.weekly.unwrap();
        assert_eq!(five.used_percentage, 36.0);
        assert_eq!(five.resets_at, Some(1789561800));
        assert_eq!(weekly.used_percentage, 25.0);
        assert_eq!(weekly.resets_at, Some(1789862400));
    }

    #[test]
    fn renders_a_status_line_for_a_real_payload() {
        let line = fallback_line(parse(REAL_PAYLOAD).as_ref());
        assert_eq!(line, "[Haiku 4.5] 5h: 36% 7d: 25%");
    }

    #[test]
    fn tolerates_a_utf8_bom() {
        assert!(parse(&format!("{BOM}{REAL_PAYLOAD}")).is_some());
    }

    /// Claude only reports rate limits once a session has had an API response,
    /// so a payload without them must not overwrite a good cached snapshot.
    #[test]
    fn payload_without_rate_limits_produces_no_snapshot() {
        let payload = parse(r#"{"model":{"display_name":"Opus 5"}}"#).unwrap();
        assert!(snapshot_from(&payload).is_none());
        assert_eq!(fallback_line(Some(&payload)), "[Opus 5]");
    }

    #[test]
    fn garbage_input_is_not_fatal() {
        assert!(parse("not json").is_none());
        assert_eq!(fallback_line(None), "[Claude]");
    }
}
