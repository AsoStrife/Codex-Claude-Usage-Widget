//! Claude Code provider, fed by the status-line bridge.
//!
//! Claude Code has no local query interface for subscription limits: the only
//! supported surface is the JSON it hands its status-line command. So this
//! provider never talks to Claude itself; it reads the snapshot that
//! `claude::bridge` writes whenever Claude Code renders its status line.

use std::path::PathBuf;

use crate::claude::cache::{self, CachedWindow, ClaudeUsageCache};
use crate::types::{ProviderId, ProviderQuota, ProviderStatus, QuotaWindow};

use super::find_executable;

const FIVE_HOUR_MINUTES: i64 = 300;
const WEEKLY_MINUTES: i64 = 7 * 24 * 60;

/// The status line only runs while Claude Code is open, so a snapshot older
/// than this means "nobody has used Claude recently", not "something broke".
/// The values stay on screen, flagged as stale.
const STALE_AFTER_SECONDS: i64 = 30 * 60;

pub fn detect_path() -> Option<PathBuf> {
    find_executable("claude")
}

pub fn refresh(integration_enabled: bool) -> ProviderQuota {
    if detect_path().is_none() {
        return ProviderQuota::not_installed(ProviderId::Claude);
    }

    match cache::load() {
        Some(snapshot) => quota_from_cache(snapshot, cache::now_seconds()),
        None => waiting(integration_enabled),
    }
}

fn waiting(integration_enabled: bool) -> ProviderQuota {
    let mut quota = ProviderQuota::empty(ProviderId::Claude);
    quota.source = "claude-bridge".into();
    if !integration_enabled {
        quota.error = Some("Enable Claude usage integration in settings.".into());
    }
    quota
}

fn quota_from_cache(snapshot: ClaudeUsageCache, now: i64) -> ProviderQuota {
    let five_hour = live_window(snapshot.five_hour, FIVE_HOUR_MINUTES, now);
    let weekly = live_window(snapshot.weekly, WEEKLY_MINUTES, now);

    // Every window has rolled over since the snapshot was taken, so the
    // percentages describe a period that no longer exists. Showing them would
    // be worse than showing nothing.
    if five_hour.is_none() && weekly.is_none() {
        let mut quota = waiting(true);
        quota.updated_at = Some(snapshot.updated_at);
        return quota;
    }

    let status = if now - snapshot.updated_at > STALE_AFTER_SECONDS {
        ProviderStatus::Stale
    } else {
        ProviderStatus::Ready
    };

    ProviderQuota {
        provider: ProviderId::Claude,
        status,
        five_hour,
        weekly,
        plan: None,
        updated_at: Some(snapshot.updated_at),
        source: "claude-bridge".into(),
        error: None,
    }
}

/// Drops a window whose reset time has already passed: its usage counter was
/// zeroed by Claude and the cached percentage is simply wrong.
fn live_window(window: Option<CachedWindow>, duration_minutes: i64, now: i64) -> Option<QuotaWindow> {
    let window = window?;
    if window.resets_at.is_some_and(|resets_at| resets_at <= now) {
        return None;
    }
    Some(QuotaWindow {
        used_percentage: window.used_percentage,
        remaining_percentage: (100.0 - window.used_percentage).clamp(0.0, 100.0),
        resets_at: window.resets_at,
        duration_minutes: Some(duration_minutes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_789_550_000;

    fn snapshot(updated_at: i64) -> ClaudeUsageCache {
        ClaudeUsageCache {
            five_hour: Some(CachedWindow {
                used_percentage: 36.0,
                resets_at: Some(NOW + 3_600),
            }),
            weekly: Some(CachedWindow {
                used_percentage: 25.0,
                resets_at: Some(NOW + 300_000),
            }),
            updated_at,
        }
    }

    #[test]
    fn maps_a_fresh_snapshot_to_both_windows() {
        let quota = quota_from_cache(snapshot(NOW - 60), NOW);
        assert_eq!(quota.status, ProviderStatus::Ready);

        let five = quota.five_hour.unwrap();
        assert_eq!(five.used_percentage, 36.0);
        assert_eq!(five.remaining_percentage, 64.0);
        assert_eq!(five.duration_minutes, Some(FIVE_HOUR_MINUTES));

        let weekly = quota.weekly.unwrap();
        assert_eq!(weekly.remaining_percentage, 75.0);
        assert_eq!(weekly.duration_minutes, Some(WEEKLY_MINUTES));
    }

    #[test]
    fn keeps_showing_values_but_flags_an_old_snapshot_as_stale() {
        let quota = quota_from_cache(snapshot(NOW - STALE_AFTER_SECONDS - 1), NOW);
        assert_eq!(quota.status, ProviderStatus::Stale);
        assert!(quota.five_hour.is_some());
        assert_eq!(quota.updated_at, Some(NOW - STALE_AFTER_SECONDS - 1));
    }

    /// The exact state `~/.claude.json`'s cached utilization was found in: the
    /// numbers are real but their windows reset days ago.
    #[test]
    fn discards_windows_whose_reset_time_has_passed() {
        let expired = ClaudeUsageCache {
            five_hour: Some(CachedWindow {
                used_percentage: 22.0,
                resets_at: Some(NOW - 600_000),
            }),
            weekly: Some(CachedWindow {
                used_percentage: 12.0,
                resets_at: Some(NOW - 200_000),
            }),
            updated_at: NOW - 700_000,
        };

        let quota = quota_from_cache(expired, NOW);
        assert_eq!(quota.status, ProviderStatus::WaitingForData);
        assert!(quota.five_hour.is_none());
        assert!(quota.weekly.is_none());
    }

    #[test]
    fn keeps_the_window_that_is_still_open() {
        let mut half = snapshot(NOW - 60);
        half.five_hour = Some(CachedWindow {
            used_percentage: 90.0,
            resets_at: Some(NOW - 10),
        });

        let quota = quota_from_cache(half, NOW);
        assert!(quota.five_hour.is_none());
        assert!(quota.weekly.is_some());
        assert_eq!(quota.status, ProviderStatus::Ready);
    }

    #[test]
    fn a_window_without_a_reset_time_is_kept() {
        let cache = ClaudeUsageCache {
            five_hour: Some(CachedWindow {
                used_percentage: 5.0,
                resets_at: None,
            }),
            weekly: None,
            updated_at: NOW,
        };
        assert!(quota_from_cache(cache, NOW).five_hour.is_some());
    }

    #[test]
    fn asks_the_user_to_enable_the_integration_when_there_is_no_snapshot() {
        assert!(waiting(false).error.is_some());
        assert!(waiting(true).error.is_none());
    }
}
