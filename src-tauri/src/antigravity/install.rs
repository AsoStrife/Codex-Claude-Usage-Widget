//! Preserve the full previous statusLine block and never edit credentials.
#[cfg(test)]
use super::bridge::FLAG;
use super::command;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn settings_path() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(
        PathBuf::from(home)
            .join(".gemini")
            .join("antigravity-cli")
            .join("settings.json"),
    )
}

pub struct PreparedInstall {
    path: PathBuf,
    settings: Value,
    pub previous: Option<Value>,
}

impl PreparedInstall {
    pub fn write(self) -> Result<(), String> {
        write_settings(&self.path, &self.settings)
    }
}

pub fn prepare(previous: Option<Value>) -> Result<PreparedInstall, String> {
    let path = settings_path().ok_or("Cannot locate the Antigravity configuration directory.")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    prepare_at(&path, &command::bridge_command(&exe), previous)
}

/// Migrate commands written by older widget versions and track a moved widget.
/// A custom command selected by the user is never replaced by this repair.
pub fn repair_owned_bridge() -> Result<(), String> {
    let path = settings_path().ok_or("Cannot locate the Antigravity configuration directory.")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    repair_at(&path, &command::bridge_command(&exe))
}

fn repair_at(path: &Path, command: &str) -> Result<(), String> {
    let mut settings = read_settings(path)?;
    let Some(line) = settings.get("statusLine").filter(|line| is_ours(line)) else {
        return Ok(());
    };
    if line.get("command").and_then(Value::as_str) == Some(command) {
        return Ok(());
    }
    settings["statusLine"]["command"] = json!(command);
    write_settings(path, &settings)
}

fn prepare_at(path: &Path, command: &str, saved: Option<Value>) -> Result<PreparedInstall, String> {
    let mut settings = read_settings(path)?;
    let current = settings.get("statusLine").cloned();
    let previous = if current.as_ref().is_some_and(is_ours) {
        saved
    } else {
        current
    };
    let mut line = previous
        .as_ref()
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}));
    // When the old line was disabled or absent, keep the CLI's built-in line.
    if line.get("command").and_then(Value::as_str).is_none()
        || line.get("enabled").and_then(Value::as_bool) == Some(false)
    {
        line["stack_with_default"] = json!(true);
    }
    line["type"] = json!("command");
    line["command"] = json!(command);
    line["enabled"] = json!(true);
    settings["statusLine"] = line;
    Ok(PreparedInstall {
        path: path.into(),
        settings,
        previous,
    })
}

pub fn uninstall(previous: Option<&Value>) -> Result<(), String> {
    let path = settings_path().ok_or("Cannot locate the Antigravity configuration directory.")?;
    uninstall_at(&path, previous)
}

fn uninstall_at(path: &Path, previous: Option<&Value>) -> Result<(), String> {
    let mut settings = read_settings(path)?;
    if !settings.get("statusLine").is_some_and(is_ours) {
        return Ok(());
    }
    match previous {
        Some(previous) => settings["statusLine"] = previous.clone(),
        None => {
            settings.as_object_mut().unwrap().remove("statusLine");
        }
    }
    write_settings(path, &settings)
}

fn is_ours(line: &Value) -> bool {
    line.get("command")
        .and_then(Value::as_str)
        .is_some_and(command::is_bridge_command)
}

fn read_settings(path: &Path) -> Result<Value, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(e) => return Err(format!("Cannot read Antigravity settings: {e}")),
    };
    let settings: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("Antigravity settings contain invalid JSON: {e}"))?;
    if !settings.is_object() {
        return Err("Antigravity settings must be a JSON object.".into());
    }
    Ok(settings)
}

fn write_settings(path: &Path, settings: &Value) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Invalid settings path")?).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let result = (|| {
        fs::write(
            &tmp,
            serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(value: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "agy-widget-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("settings.json");
            fs::write(&path, value).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_dir(self.0.parent().unwrap());
        }
    }

    #[test]
    fn repeated_install_keeps_original_block_and_uninstall_restores_it() {
        let original = json!({"theme":"dark","statusLine":{"command":"echo original","enabled":false,"padding":3,"stack_with_default":false}});
        let fixture = Fixture::new(&original.to_string());
        let first = prepare_at(&fixture.0, &format!("widget.exe {FLAG}"), None).unwrap();
        let backup = first.previous.clone();
        first.write().unwrap();
        let second = prepare_at(
            &fixture.0,
            &format!("new-widget.exe {FLAG}"),
            backup.clone(),
        )
        .unwrap();
        assert_eq!(second.previous, backup);
        second.write().unwrap();
        assert_eq!(
            read_settings(&fixture.0).unwrap()["statusLine"]["stack_with_default"],
            true
        );
        uninstall_at(&fixture.0, backup.as_ref()).unwrap();
        assert_eq!(read_settings(&fixture.0).unwrap(), original);
    }

    #[test]
    fn removal_preserves_a_status_line_changed_by_the_user() {
        let fixture = Fixture::new(r#"{"statusLine":{"command":"echo new"}}"#);
        uninstall_at(&fixture.0, Some(&json!({"command":"echo old"}))).unwrap();
        assert_eq!(
            read_settings(&fixture.0).unwrap()["statusLine"]["command"],
            "echo new"
        );
    }

    #[test]
    fn repair_updates_an_owned_command_but_preserves_preferences_and_custom_commands() {
        let fixture = Fixture::new(&json!({"theme":"dark", "statusLine":{
            "command":format!("\"C:/Program Files/widget.exe\" {FLAG}"), "enabled":false, "padding":2
        }}).to_string());
        repair_at(&fixture.0, &format!("C:/APP~1/widget.exe {FLAG}")).unwrap();
        let repaired = read_settings(&fixture.0).unwrap();
        assert_eq!(
            repaired["statusLine"]["command"],
            format!("C:/APP~1/widget.exe {FLAG}")
        );
        assert_eq!(repaired["statusLine"]["enabled"], false);
        assert_eq!(repaired["statusLine"]["padding"], 2);
        fs::write(&fixture.0, r#"{"statusLine":{"command":"custom command"}}"#).unwrap();
        repair_at(&fixture.0, "replacement").unwrap();
        assert_eq!(
            read_settings(&fixture.0).unwrap()["statusLine"]["command"],
            "custom command"
        );
    }

    #[test]
    fn no_original_status_line_is_removed_on_uninstall_and_invalid_settings_are_not_overwritten() {
        let fixture = Fixture::new(r#"{"theme":"dark"}"#);
        prepare_at(&fixture.0, &format!("widget.exe {FLAG}"), None)
            .unwrap()
            .write()
            .unwrap();
        uninstall_at(&fixture.0, None).unwrap();
        assert_eq!(read_settings(&fixture.0).unwrap(), json!({"theme":"dark"}));
        fs::write(&fixture.0, "corrupt").unwrap();
        assert!(prepare_at(&fixture.0, "command", None).is_err());
        assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "corrupt");
    }
}
