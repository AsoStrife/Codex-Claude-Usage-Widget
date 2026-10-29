//! Only sanitized quota metadata is persisted; no session content or credentials.
use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::QuotaBucket;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntigravityUsageCache {
    pub buckets: Vec<QuotaBucket>,
    pub plan: Option<String>,
    pub updated_at: i64,
}

pub fn cache_path() -> Option<PathBuf> {
    Some(crate::claude::cache::config_dir()?.join("antigravity-usage.json"))
}

pub fn load() -> Option<AntigravityUsageCache> {
    serde_json::from_str(&fs::read_to_string(cache_path()?).ok()?).ok()
}

pub fn store(snapshot: &AntigravityUsageCache) -> io::Result<()> {
    let path = cache_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory"))?;
    fs::create_dir_all(path.parent().unwrap())?;
    // Multiple CLI sessions may render concurrently. Each bridge owns its
    // temporary file and atomically replaces the shared snapshot when ready.
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let result = (|| {
        fs::write(&tmp, serde_json::to_vec(snapshot)?)?;
        fs::rename(&tmp, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
