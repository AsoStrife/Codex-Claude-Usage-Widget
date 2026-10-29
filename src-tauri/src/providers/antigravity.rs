use crate::{
    antigravity::cache::{self, AntigravityUsageCache},
    claude::cache::now_seconds,
    types::{ProviderId, ProviderQuota, ProviderStatus},
};
use std::path::PathBuf;

pub fn detect_path() -> Option<PathBuf> {
    super::find_executable("agy").or_else(|| {
        #[cfg(windows)]
        let path = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("agy/bin/agy.exe");
        #[cfg(not(windows))]
        let path = PathBuf::from(std::env::var_os("HOME")?).join(".local/bin/agy");
        path.is_file().then_some(path)
    })
}

pub fn refresh(enabled: bool) -> ProviderQuota {
    if detect_path().is_none() {
        return ProviderQuota::not_installed(ProviderId::Antigravity);
    }
    if !enabled {
        return waiting("Enable Antigravity usage integration in settings.");
    }
    if let Err(error) = crate::antigravity::install::repair_owned_bridge() {
        let mut quota = waiting(&format!(
            "Could not repair Antigravity integration: {error}"
        ));
        quota.status = ProviderStatus::Error;
        return quota;
    }
    match cache::load() {
        Some(snapshot) => quota_from_cache(snapshot, now_seconds()),
        None => waiting("Open Antigravity CLI and run /usage to refresh quota data."),
    }
}

fn waiting(message: &str) -> ProviderQuota {
    ProviderQuota {
        source: "antigravity-bridge".into(),
        error: Some(message.into()),
        ..ProviderQuota::empty(ProviderId::Antigravity)
    }
}

fn quota_from_cache(snapshot: AntigravityUsageCache, now: i64) -> ProviderQuota {
    let mut quota = waiting("Open Antigravity CLI and run /usage to refresh quota data.");
    quota.buckets = snapshot
        .buckets
        .into_iter()
        .filter(|bucket| bucket.disabled || bucket.resets_at.map_or(true, |reset| reset > now))
        .collect();
    quota.plan = snapshot.plan;
    quota.updated_at = Some(snapshot.updated_at);
    if !quota.buckets.is_empty() {
        quota.status = if now.saturating_sub(snapshot.updated_at) > 1800 {
            ProviderStatus::Stale
        } else {
            ProviderStatus::Ready
        };
        quota.error = None;
    }
    quota
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::QuotaBucket;
    #[test]
    fn expires_each_bucket_without_inventing_refilled_quotas_and_marks_old_data_stale() {
        let bucket = |id: &str, reset| QuotaBucket {
            id: id.into(),
            label: id.into(),
            remaining_percentage: Some(25.0),
            remaining_amount: None,
            resets_at: Some(reset),
            disabled: false,
        };
        let snapshot = AntigravityUsageCache {
            buckets: vec![bucket("5h", 99), bucket("weekly", 5000)],
            plan: None,
            updated_at: 50,
        };
        let quota = quota_from_cache(snapshot.clone(), 2000);
        assert_eq!(quota.status, ProviderStatus::Stale);
        assert_eq!(quota.buckets.len(), 1);
        assert_eq!(quota.buckets[0].id, "weekly");
        let expired = quota_from_cache(snapshot, 5000);
        assert_eq!(expired.status, ProviderStatus::WaitingForData);
        assert!(expired.buckets.is_empty());
    }
}
