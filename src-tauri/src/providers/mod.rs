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
pub fn quiet_command(path: &Path) -> Command {
    let mut command = Command::new(path);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
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
