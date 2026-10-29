use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Codex,
    Claude,
    Antigravity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderStatus {
    Disabled,
    Ready,
    NotInstalled,
    NotAuthenticated,
    WaitingForData,
    Stale,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub used_percentage: f64,
    pub remaining_percentage: f64,
    pub resets_at: Option<i64>,
    pub duration_minutes: Option<i64>,
}

/// Named quota buckets can report a fraction or an absolute balance. An
/// absolute balance alone does not tell us the account's percentage remaining.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaBucket {
    pub id: String,
    pub label: String,
    pub remaining_percentage: Option<f64>,
    pub remaining_amount: Option<f64>,
    pub resets_at: Option<i64>,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuota {
    pub provider: ProviderId,
    pub status: ProviderStatus,
    pub five_hour: Option<QuotaWindow>,
    pub weekly: Option<QuotaWindow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buckets: Vec<QuotaBucket>,
    pub plan: Option<String>,
    pub updated_at: Option<i64>,
    pub source: String,
    pub error: Option<String>,
}

impl ProviderQuota {
    pub fn disabled(provider: ProviderId) -> Self {
        Self {
            status: ProviderStatus::Disabled,
            source: "disabled".into(),
            ..Self::empty(provider)
        }
    }

    pub fn empty(provider: ProviderId) -> Self {
        Self {
            provider,
            status: ProviderStatus::WaitingForData,
            five_hour: None,
            weekly: None,
            buckets: Vec::new(),
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
    pub antigravity: ProviderQuota,
}

impl Default for UsageSnapshot {
    fn default() -> Self {
        Self {
            codex: ProviderQuota::empty(ProviderId::Codex),
            claude: ProviderQuota::empty(ProviderId::Claude),
            antigravity: ProviderQuota::empty(ProviderId::Antigravity),
        }
    }
}

impl UsageSnapshot {
    pub fn provider_mut(&mut self, provider: ProviderId) -> &mut ProviderQuota {
        match provider {
            ProviderId::Codex => &mut self.codex,
            ProviderId::Claude => &mut self.claude,
            ProviderId::Antigravity => &mut self.antigravity,
        }
    }

    pub fn apply_monitoring(&mut self, monitoring: &ProviderMonitoring) {
        for provider in [
            ProviderId::Codex,
            ProviderId::Claude,
            ProviderId::Antigravity,
        ] {
            if !monitoring.enabled(provider) {
                *self.provider_mut(provider) = ProviderQuota::disabled(provider);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderMonitoring {
    pub codex: bool,
    pub claude: bool,
    pub antigravity: bool,
}

impl Default for ProviderMonitoring {
    fn default() -> Self {
        Self {
            codex: true,
            claude: true,
            antigravity: true,
        }
    }
}

impl ProviderMonitoring {
    pub fn enabled(&self, provider: ProviderId) -> bool {
        match provider {
            ProviderId::Codex => self.codex,
            ProviderId::Claude => self.claude,
            ProviderId::Antigravity => self.antigravity,
        }
    }

    pub fn set_enabled(&mut self, provider: ProviderId, enabled: bool) {
        match provider {
            ProviderId::Codex => self.codex = enabled,
            ProviderId::Claude => self.claude = enabled,
            ProviderId::Antigravity => self.antigravity = enabled,
        }
    }
}

// `default` on the container, not just the new field: settings files written
// by an older build are missing keys, and without this the whole file fails to
// deserialize and every setting silently resets.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub monitoring: ProviderMonitoring,
    pub start_with_windows: bool,
    pub hide_on_blur: bool,
    pub refresh_interval_seconds: u64,
    pub claude_integration_enabled: bool,
    /// Claude's status-line command from before the bridge was installed. The
    /// bridge forwards to it, and enabling/disabling the integration restores
    /// it, so the user never loses their own status line.
    pub claude_previous_status_line: Option<String>,
    pub antigravity_integration_enabled: bool,
    /// Keep the entire statusLine block, including enabled, padding and
    /// stack_with_default, so disabling can restore the original configuration.
    pub antigravity_previous_status_line: Option<serde_json::Value>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            monitoring: ProviderMonitoring::default(),
            start_with_windows: false,
            hide_on_blur: true,
            refresh_interval_seconds: 60,
            claude_integration_enabled: false,
            claude_previous_status_line: None,
            antigravity_integration_enabled: false,
            antigravity_previous_status_line: None,
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
    pub antigravity_path: Option<String>,
    pub antigravity_version: Option<String>,
    pub antigravity_integration_enabled: bool,
    pub antigravity_cache_updated_at: Option<i64>,
    pub antigravity_settings_path: Option<String>,
}

#[cfg(test)]
mod monitoring_tests {
    use super::*;

    #[test]
    fn older_settings_keep_existing_connections_and_enable_all_monitors() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
            "startWithWindows":true, "claudeIntegrationEnabled":true,
            "antigravityIntegrationEnabled":true, "claudePreviousStatusLine":"custom"
        }"#,
        )
        .unwrap();
        assert!(
            settings.monitoring.codex
                && settings.monitoring.claude
                && settings.monitoring.antigravity
        );
        assert!(
            settings.start_with_windows
                && settings.claude_integration_enabled
                && settings.antigravity_integration_enabled
        );
        assert_eq!(
            settings.claude_previous_status_line.as_deref(),
            Some("custom")
        );
    }

    #[test]
    fn saved_monitor_choices_survive_reload_and_partial_settings() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"monitoring":{"codex":false}}"#).unwrap();
        assert!(!settings.monitoring.codex);
        assert!(settings.monitoring.claude && settings.monitoring.antigravity);
        let reloaded: AppSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(!reloaded.monitoring.codex);
        let mut snapshot = UsageSnapshot::default();
        snapshot.apply_monitoring(&reloaded.monitoring);
        assert_eq!(snapshot.codex.status, ProviderStatus::Disabled);
        assert_eq!(snapshot.claude.status, ProviderStatus::WaitingForData);
    }
}
