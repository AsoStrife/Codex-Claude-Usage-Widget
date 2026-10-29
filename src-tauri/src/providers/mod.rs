pub mod antigravity;
pub mod claude;
pub mod codex;

use std::path::{Path, PathBuf};

use tokio::process::Command;

/// Resolves an executable name against `PATH`, trying every extension in
/// `PATHEXT` on Windows since `codex`/`claude` are usually installed as
/// `codex.exe`/`codex.cmd` rather than an extension-less binary.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;

    let candidates: Vec<String> = if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
        pathext
            .split(';')
            .filter(|ext| !ext.is_empty())
            .map(|ext| format!("{name}{}", ext.to_lowercase()))
            .collect()
    } else {
        vec![name.to_string()]
    };

    for dir in std::env::split_paths(&path_var) {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

/// Builds a `Command` that never flashes a console window. The release build
/// is a GUI subsystem binary, so spawning `codex`/`claude` (often `.cmd`
/// shims) would otherwise pop up a conhost window on every refresh.
/// Resolve Windows junctions before CreateProcess: Codex's stable install
/// path points through `standalone/current` into a versioned release, and
/// Windows can reject traversal of that launch path with error 448.
pub fn quiet_command(path: &Path) -> Command {
    let mut command = Command::new(executable_path(path));
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn executable_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        // Keep the original path when resolution fails so spawning reports
        // the actual error instead of classifying an inaccessible CLI as absent.
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }
    #[cfg(not(windows))]
    path.to_path_buf()
}

/// Runs `<path> --version` and returns its trimmed stdout, if it succeeds.
pub async fn detect_version(path: &Path) -> Option<String> {
    let output = quiet_command(path).arg("--version").output().await.ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn launches_through_a_directory_junction_using_the_real_binary_path() {
        use std::os::windows::process::CommandExt;
        struct Junction(std::path::PathBuf);
        impl Drop for Junction {
            fn drop(&mut self) {
                // remove_dir removes only the junction, never its target.
                let _ = std::fs::remove_dir(self.0.join("alias"));
                let _ = std::fs::remove_dir(&self.0);
            }
        }
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fixture = Junction(
            std::env::temp_dir().join(format!("widget-launch-{}-{unique}", std::process::id())),
        );
        std::fs::create_dir(&fixture.0).unwrap();
        let exe = std::env::current_exe().unwrap();
        let alias = fixture.0.join("alias");
        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&alias)
            .arg(exe.parent().unwrap())
            .creation_flags(0x0800_0000)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let linked_exe = alias.join(exe.file_name().unwrap());
        let expected = std::fs::canonicalize(&exe).unwrap();
        let mut command = quiet_command(&linked_exe);
        assert_eq!(command.as_std().get_program(), expected.as_os_str());
        let output = command.arg("--list").output().await.unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains("launches_through_a_directory_junction"));
    }

    #[test]
    fn unresolved_executables_keep_their_original_path_for_error_reporting() {
        let path = Path::new("Z:/ai-usage-widget-missing/codex.exe");
        assert_eq!(executable_path(path), path);
    }
}
