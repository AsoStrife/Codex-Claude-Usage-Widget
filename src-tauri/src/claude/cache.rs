//! The bridge's local snapshot of Claude's rate-limit state.
//!
//! Written by the status-line bridge process (which has no Tauri `AppHandle`)
//! and read by the widget, so the directory is derived from the environment on
//! both sides rather than from the Tauri path API.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Must match `identifier` in `tauri.conf.json`: Tauri derives
/// `app_config_dir()` from it, and the bridge has to land in the same place.
pub const APP_IDENTIFIER: &str = "com.asostrife.ai-usage-widget";

const CACHE_FILE: &str = "claude-usage.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedWindow {
    pub used_percentage: f64,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageCache {
    pub five_hour: Option<CachedWindow>,
    pub weekly: Option<CachedWindow>,
    pub updated_at: i64,
}

/// `%APPDATA%\<identifier>` on Windows, which is exactly what Tauri's
/// `app_config_dir()` resolves to, so the widget and the bridge agree.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    Some(base?.join(APP_IDENTIFIER))
}

pub fn cache_path() -> Option<PathBuf> {
    Some(config_dir()?.join(CACHE_FILE))
}

pub fn load() -> Option<ClaudeUsageCache> {
    let raw = fs::read_to_string(cache_path()?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Writes via a temporary file and a rename, so the widget can never observe a
/// half-written snapshot while the bridge is running.
pub fn store(cache: &ClaudeUsageCache) -> io::Result<()> {
    let path = cache_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let raw = serde_json::to_string_pretty(cache)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw)?;
    fs::rename(&tmp, &path)
}

pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bridge builds its path from `APP_IDENTIFIER` while the rest of the
    /// app goes through Tauri, so a drift between the two would silently split
    /// them into different directories.
    #[test]
    fn identifier_matches_tauri_config() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"].as_str(), Some(APP_IDENTIFIER));
    }
}
