//! Documented CLI contract: https://antigravity.google/docs/cli/statusline/
//! Runs before Tauri initialization, even when the widget is already running.
use std::{collections::BTreeMap, io::Read};

use serde::Deserialize;
use serde_json::Value;

use super::cache::{self, AntigravityUsageCache};
use crate::{claude::cache::now_seconds, types::QuotaBucket};

pub const FLAG: &str = "--antigravity-statusline-bridge";
const MAX_INPUT_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct Payload {
    quota: Option<BTreeMap<String, Value>>,
    plan_tier: Option<String>,
}

#[derive(Deserialize)]
struct Bucket {
    remaining_fraction: Option<f64>,
    remaining_amount: Option<f64>,
    reset_time: Option<Value>,
    reset_in_seconds: Option<f64>,
    #[serde(default)]
    disabled: bool,
}

pub fn run_if_requested() -> bool {
    if !std::env::args().any(|arg| arg == FLAG) {
        return false;
    }
    let mut input = String::new();
    let read = std::io::stdin()
        .take(MAX_INPUT_BYTES + 1)
        .read_to_string(&mut input);
    if read.is_ok() && input.len() as u64 <= MAX_INPUT_BYTES {
        if let Some(snapshot) = snapshot_from(&input, now_seconds()) {
            let _ = cache::store(&snapshot);
        }
        // If there was no custom command, installation enables
        // stack_with_default and this bridge can stay silent.
        if let Some(output) = forward_to_previous(&input) {
            print!("{output}");
        }
    }
    true
}

fn snapshot_from(input: &str, now: i64) -> Option<AntigravityUsageCache> {
    let payload: Payload = serde_json::from_str(input.trim_start_matches('\u{feff}')).ok()?;
    let quota = payload.quota?;
    let was_empty = quota.is_empty();
    let buckets: Vec<_> = quota
        .into_iter()
        .filter_map(|(id, raw)| {
            let raw: Bucket = serde_json::from_value(raw).ok()?;
            let remaining_percentage = raw
                .remaining_fraction
                .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
                .map(|v| v * 100.0);
            let remaining_amount = raw.remaining_amount.filter(|v| v.is_finite() && *v >= 0.0);
            if !raw.disabled && remaining_percentage.is_none() && remaining_amount.is_none() {
                return None;
            }
            let resets_at = raw.reset_time.as_ref().and_then(parse_reset).or_else(|| {
                let seconds = raw.reset_in_seconds?.floor();
                if !seconds.is_finite() || seconds < 0.0 || seconds >= i64::MAX as f64 {
                    return None;
                }
                now.checked_add(seconds as i64)
            });
            let label = id
                .split(['-', '_'])
                .filter(|s| !s.is_empty())
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            Some(QuotaBucket {
                id,
                label,
                remaining_percentage,
                remaining_amount,
                resets_at,
                disabled: raw.disabled,
            })
        })
        .collect();
    // An absent/malformed quota payload must not erase a valid reading.
    // An explicitly empty map, however, is an authoritative empty snapshot.
    if buckets.is_empty() && !was_empty {
        return None;
    }
    Some(AntigravityUsageCache {
        buckets,
        plan: payload.plan_tier,
        updated_at: now,
    })
}

fn parse_reset(value: &Value) -> Option<i64> {
    if let Some(seconds) = value.as_i64() {
        return Some(seconds);
    }
    let text = value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|date| date.timestamp())
}

fn previous_command() -> Option<String> {
    let path = crate::claude::cache::config_dir()?.join("settings.json");
    let settings: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let previous = settings.get("antigravityPreviousStatusLine")?;
    if previous.get("enabled").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let command = previous.get("command")?.as_str()?.trim();
    if command.is_empty() || super::command::is_bridge_command(command) {
        return None;
    }
    Some(command.into())
}

fn forward_to_previous(input: &str) -> Option<String> {
    use std::{
        io::Write,
        process::{Command, Stdio},
        sync::mpsc,
        time::{Duration, Instant},
    };

    let command = previous_command()?;
    #[cfg(windows)]
    let mut shell = {
        use std::os::windows::process::CommandExt;
        let mut shell = Command::new(std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into()));
        shell.raw_arg(format!("/d /s /c \"{command}\""));
        shell.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        shell
    };
    #[cfg(not(windows))]
    let mut shell = {
        let mut shell = Command::new("sh");
        shell.args(["-c", &command]);
        shell
    };
    let mut child = shell
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let input = input.as_bytes().to_vec();
    std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout.take(64 * 1024).read_to_end(&mut output);
        let _ = sender.send(output);
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let output = receiver.recv_timeout(Duration::from_millis(100)).ok()?;
    Some(String::from_utf8_lossy(&output).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: i64 = 1_789_550_000;

    #[test]
    fn reads_documented_buckets_without_retaining_private_session_data() {
        let snapshot = snapshot_from(
            r#"{
          "email":"private@example.com", "transcript_path":"private", "plan_tier":"Pro",
          "quota":{
            "gemini-weekly":{"remaining_fraction":0.9378,"reset_time":"2026-07-06T07:50:32Z"},
            "gemini-5h":{"remaining_fraction":0,"reset_in_seconds":30},
            "credits":{"remaining_amount":42},
            "disabled-model":{"disabled":true}
          }
        }"#,
            NOW,
        )
        .unwrap();
        assert_eq!(snapshot.buckets.len(), 4);
        let weekly = snapshot
            .buckets
            .iter()
            .find(|b| b.id == "gemini-weekly")
            .unwrap();
        assert!((weekly.remaining_percentage.unwrap() - 93.78).abs() < 0.001);
        assert_eq!(weekly.resets_at, Some(1_783_324_232));
        let five = snapshot
            .buckets
            .iter()
            .find(|b| b.id == "gemini-5h")
            .unwrap();
        assert_eq!(five.remaining_percentage, Some(0.0));
        assert_eq!(five.resets_at, Some(NOW + 30));
        let credits = snapshot.buckets.iter().find(|b| b.id == "credits").unwrap();
        assert_eq!(credits.remaining_percentage, None);
        assert_eq!(credits.remaining_amount, Some(42.0));
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("private"));
    }

    #[test]
    fn ignores_missing_and_malformed_data_but_can_clear_an_empty_quota() {
        for raw in [
            "not json",
            r#"{}"#,
            r#"{"quota":null}"#,
            r#"{"quota":{"bad":{"remaining_fraction":2}}}"#,
            r#"{"quota":{"bad":{"remaining_amount":-1}}}"#,
        ] {
            assert!(snapshot_from(raw, NOW).is_none(), "{raw}");
        }
        assert!(snapshot_from("\u{feff}{\"quota\":{}}", NOW)
            .unwrap()
            .buckets
            .is_empty());
    }
}
