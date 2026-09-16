//! Installs and removes the widget's bridge as Claude Code's status-line
//! command.
//!
//! The user's `settings.json` is theirs: it is edited as a generic JSON object
//! so unrelated keys survive untouched, the command that was there before is
//! remembered so the bridge can keep feeding it, and an uninstall never clears
//! a status line the widget didn't install.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::bridge::FLAG;

/// Honours `CLAUDE_CONFIG_DIR` the same way Claude Code does, so a user who
/// relocated their config isn't silently written to the wrong file.
pub fn settings_path() -> Option<PathBuf> {
    let dir = match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => home_dir()?.join(".claude"),
    };
    Some(dir.join("settings.json"))
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            return Some(PathBuf::from(profile));
        }
    }
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The command Claude Code is asked to run, e.g.
/// `"C:\Program Files\AI Usage Widget\ai-usage-widget.exe" --claude-statusline-bridge`.
pub fn bridge_command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|err| format!("cannot locate this app: {err}"))?;
    Ok(format!("\"{}\" {FLAG}", exe.display()))
}

fn is_bridge_command(command: &str) -> bool {
    command.contains(FLAG)
}

fn read_settings(path: &Path) -> Result<Value, String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(json!({}));
    };
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|err| format!("{} is not valid JSON: {err}", path.display()))
}

fn write_settings(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let raw = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;

    // Same temp-file dance as the usage cache: Claude Code reads this file on
    // its own schedule and must never catch it half written.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw).map_err(|err| err.to_string())?;
    fs::rename(&tmp, path).map_err(|err| err.to_string())
}

/// Points Claude Code's status line at the widget. Returns the command that was
/// configured before, which the caller stores so the bridge can forward to it
/// and so it can be restored on uninstall.
pub fn install() -> Result<Option<String>, String> {
    let path = settings_path().ok_or_else(|| "cannot locate the Claude config directory".to_string())?;
    install_at(&path, &bridge_command()?)
}

fn install_at(path: &Path, command: &str) -> Result<Option<String>, String> {
    let mut settings = read_settings(path)?;

    let existing = settings
        .get("statusLine")
        .and_then(|line| line.get("command"))
        .and_then(Value::as_str)
        .map(str::to_string);

    // Never record our own command as the "previous" one: that would make the
    // bridge invoke itself on every render.
    let previous = existing.filter(|command| !is_bridge_command(command));

    // Keep whatever cadence the user already chose.
    let refresh_interval = settings
        .get("statusLine")
        .and_then(|line| line.get("refreshInterval"))
        .cloned()
        .unwrap_or_else(|| json!(60));

    settings["statusLine"] = json!({
        "type": "command",
        "command": command,
        "refreshInterval": refresh_interval,
    });

    write_settings(path, &settings)?;
    Ok(previous)
}

/// Restores the status line the widget replaced. Leaves the file alone if the
/// configured command is no longer ours, since the user has changed it since.
pub fn uninstall(previous: Option<&str>) -> Result<(), String> {
    let path = settings_path().ok_or_else(|| "cannot locate the Claude config directory".to_string())?;
    uninstall_at(&path, previous)
}

fn uninstall_at(path: &Path, previous: Option<&str>) -> Result<(), String> {
    let mut settings = read_settings(path)?;

    let current = settings
        .get("statusLine")
        .and_then(|line| line.get("command"))
        .and_then(Value::as_str);
    let Some(current) = current else {
        return Ok(());
    };
    if !is_bridge_command(current) {
        return Ok(());
    }

    match previous.map(str::trim).filter(|command| !command.is_empty()) {
        Some(command) => settings["statusLine"]["command"] = json!(command),
        None => {
            if let Some(object) = settings.as_object_mut() {
                object.remove("statusLine");
            }
        }
    }

    write_settings(path, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_its_own_command() {
        assert!(is_bridge_command("\"C:\\app.exe\" --claude-statusline-bridge"));
        assert!(!is_bridge_command("powershell -File status.ps1"));
    }

    #[test]
    fn bridge_command_is_quoted_so_paths_with_spaces_survive() {
        let command = bridge_command().unwrap();
        assert!(command.starts_with('"'));
        assert!(command.ends_with(FLAG));
        // The quoted executable must be closed before the flag begins.
        assert!(command.contains("\" --claude-statusline-bridge"));
    }

    const OURS: &str = "\"C:\\Apps\\AI Usage Widget.exe\" --claude-statusline-bridge";
    const THEIRS: &str = "powershell -NoProfile -File \"C:/Tools/statusline.ps1\"";

    /// Shaped like a real `~/.claude/settings.json`: a status line plus a pile
    /// of unrelated keys that must survive the round trip untouched.
    fn existing_settings() -> String {
        serde_json::to_string_pretty(&json!({
            "model": "haiku",
            "statusLine": { "type": "command", "command": THEIRS, "refreshInterval": 30 },
            "theme": "dark",
            "extraKnownMarketplaces": { "official": { "source": { "repo": "anthropics/x" } } },
        }))
        .unwrap()
    }

    struct TempSettings(PathBuf);

    impl TempSettings {
        fn new(name: &str, contents: Option<&str>) -> Self {
            let path = std::env::temp_dir()
                .join(format!("ai-usage-widget-test-{name}"))
                .join("settings.json");
            let _ = fs::remove_dir_all(path.parent().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if let Some(contents) = contents {
                fs::write(&path, contents).unwrap();
            }
            Self(path)
        }

        fn read(&self) -> Value {
            serde_json::from_str(&fs::read_to_string(&self.0).unwrap()).unwrap()
        }

        fn command(&self) -> Option<String> {
            self.read()["statusLine"]["command"]
                .as_str()
                .map(str::to_string)
        }
    }

    impl Drop for TempSettings {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    #[test]
    fn install_replaces_the_command_and_reports_the_previous_one() {
        let settings = TempSettings::new("install", Some(&existing_settings()));

        let previous = install_at(&settings.0, OURS).unwrap();

        assert_eq!(previous.as_deref(), Some(THEIRS));
        assert_eq!(settings.command().as_deref(), Some(OURS));
        // Unrelated keys, and the user's own refresh cadence, are preserved.
        let written = settings.read();
        assert_eq!(written["model"], "haiku");
        assert_eq!(written["theme"], "dark");
        assert_eq!(written["extraKnownMarketplaces"]["official"]["source"]["repo"], "anthropics/x");
        assert_eq!(written["statusLine"]["refreshInterval"], 30);
    }

    #[test]
    fn uninstall_restores_exactly_what_was_there_before() {
        let before = existing_settings();
        let settings = TempSettings::new("roundtrip", Some(&before));

        let previous = install_at(&settings.0, OURS).unwrap();
        uninstall_at(&settings.0, previous.as_deref()).unwrap();

        let restored: Value = settings.read();
        let original: Value = serde_json::from_str(&before).unwrap();
        assert_eq!(restored, original);
    }

    /// Enabling twice must not record the bridge as its own predecessor, which
    /// would make the status line invoke the widget recursively forever.
    #[test]
    fn installing_twice_does_not_capture_its_own_command() {
        let settings = TempSettings::new("twice", Some(&existing_settings()));

        install_at(&settings.0, OURS).unwrap();
        let second = install_at(&settings.0, OURS).unwrap();

        assert_eq!(second, None);
        assert_eq!(settings.command().as_deref(), Some(OURS));
    }

    #[test]
    fn uninstall_removes_the_status_line_when_the_user_had_none() {
        let settings = TempSettings::new("none", Some(r#"{"model":"haiku"}"#));

        let previous = install_at(&settings.0, OURS).unwrap();
        assert_eq!(previous, None);
        uninstall_at(&settings.0, None).unwrap();

        let written = settings.read();
        assert!(written.get("statusLine").is_none());
        assert_eq!(written["model"], "haiku");
    }

    /// If the user has since pointed the status line somewhere else, disabling
    /// the integration must leave their choice alone.
    #[test]
    fn uninstall_leaves_a_status_line_the_widget_does_not_own() {
        let settings = TempSettings::new("foreign", Some(&existing_settings()));

        uninstall_at(&settings.0, Some("something-else")).unwrap();

        assert_eq!(settings.command().as_deref(), Some(THEIRS));
    }

    #[test]
    fn install_creates_the_file_when_there_is_no_settings_json_yet() {
        let settings = TempSettings::new("fresh", None);

        assert_eq!(install_at(&settings.0, OURS).unwrap(), None);

        assert_eq!(settings.command().as_deref(), Some(OURS));
        assert_eq!(settings.read()["statusLine"]["refreshInterval"], 60);
    }

    #[test]
    fn a_corrupt_settings_file_is_reported_rather_than_overwritten() {
        let settings = TempSettings::new("corrupt", Some("{ not json"));

        let error = install_at(&settings.0, OURS).unwrap_err();

        assert!(error.contains("not valid JSON"), "{error}");
        assert_eq!(fs::read_to_string(&settings.0).unwrap(), "{ not json");
    }
}
