use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Codex,
    Claude,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderStatus {
    Ready,
    NotInstalled,
    NotAuthenticated,
    WaitingForData,
    Stale,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub used_percentage: f64,
    pub remaining_percentage: f64,
    pub resets_at: Option<i64>,
    pub duration_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuota {
    pub provider: ProviderId,
    pub status: ProviderStatus,
    pub five_hour: Option<QuotaWindow>,
    pub weekly: Option<QuotaWindow>,
    pub plan: Option<String>,
    pub updated_at: Option<i64>,
    pub source: String,
    pub error: Option<String>,
}

impl ProviderQuota {
    pub fn empty(provider: ProviderId) -> Self {
        Self {
            provider,
            status: ProviderStatus::WaitingForData,
            five_hour: None,
            weekly: None,
            plan: None,
            updated_at: None,
            source: "pending".into(),
            error: None,
        }
    }

    pub fn not_installed(provider: ProviderId) -> Self {
        Self {
            status: ProviderStatus::NotInstalled,
            source: "not-installed".into(),
            ..Self::empty(provider)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub codex: ProviderQuota,
    pub claude: ProviderQuota,
}

impl Default for UsageSnapshot {
    fn default() -> Self {
        Self {
            codex: ProviderQuota::empty(ProviderId::Codex),
            claude: ProviderQuota::empty(ProviderId::Claude),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub start_with_windows: bool,
    pub hide_on_blur: bool,
    pub refresh_interval_seconds: u64,
    pub claude_integration_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            hide_on_blur: true,
            refresh_interval_seconds: 60,
            claude_integration_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub app_version: String,
    pub codex_path: Option<String>,
    pub codex_version: Option<String>,
    pub codex_status: ProviderStatus,
    pub codex_app_server_status: String,
    pub claude_path: Option<String>,
    pub claude_version: Option<String>,
    pub claude_integration_enabled: bool,
    pub claude_cache_updated_at: Option<i64>,
    pub claude_settings_path: Option<String>,
}
