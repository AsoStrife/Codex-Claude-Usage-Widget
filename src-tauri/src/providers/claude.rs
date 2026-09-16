use std::path::PathBuf;

use crate::types::{ProviderId, ProviderQuota};

use super::find_executable;

/// Claude Code detection only. The status-line bridge integration described
/// in `plan.md` (Milestone 4) is a separate, much larger piece of work and is
/// intentionally not implemented yet: it requires shipping a bridge mode in
/// this same executable, rewriting the user's Claude settings, and watching
/// a local cache file. Until then we only report whether Claude Code is
/// installed so the UI can show an accurate, non-misleading state.
pub fn detect_path() -> Option<PathBuf> {
    find_executable("claude")
}

pub fn refresh() -> ProviderQuota {
    match detect_path() {
        Some(_) => {
            let mut quota = ProviderQuota::empty(ProviderId::Claude);
            quota.source = "claude-bridge".into();
            quota.error = Some("Claude usage integration is not implemented yet.".into());
            quota
        }
        None => ProviderQuota::not_installed(ProviderId::Claude),
    }
}
