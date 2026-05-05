//! JoeMode session lifecycle hooks — thin app wrapper.
//!
//! Pure logic lives in `warpx::hooks`. This module gates calls behind
//! FeatureFlag::JoeMode / FeatureFlag::OzSessionHooks.

use std::path::Path;

use crate::features::FeatureFlag;
use crate::test_runner::FailureStore;

/// Run on terminal tab/session open. Detects handoff items for the cwd.
pub fn on_session_open(cwd: &Path) -> Option<String> {
    if !FeatureFlag::JoeMode.is_enabled() {
        return None;
    }

    // Surface failure persistence banner before the handoff detect output.
    if FeatureFlag::OzFailurePersistence.is_enabled() {
        if let Some(banner) = failure_banner(cwd) {
            return Some(banner);
        }
    }

    warpx::hooks::on_session_open(cwd)
}

/// Returns a banner string if there are persisted failures for `cwd`.
fn failure_banner(cwd: &Path) -> Option<String> {
    let record = FailureStore::new().load(cwd).ok()??;
    let count = record.test_names.len();
    Some(format!(
        "{count} test{s} failed last session — re-run? ({cmd})",
        s = if count == 1 { "" } else { "s" },
        cmd = record.rerun_command,
    ))
}

/// Run on session close. Triggers godmode handoff validation (best-effort).
pub fn on_session_close(cwd: &Path) {
    if !FeatureFlag::JoeMode.is_enabled() {
        return;
    }
    warpx::hooks::on_session_close(cwd);
}

/// Fire-and-forget: runs `godmode handon` in background on tab open.
pub fn spawn_handon() {
    if !FeatureFlag::OzSessionHooks.is_enabled() {
        return;
    }
    warpx::hooks::spawn_handon();
}

/// Fire-and-forget: runs `godmode handoff` in background on tab close.
pub fn spawn_handoff() {
    if !FeatureFlag::OzSessionHooks.is_enabled() {
        return;
    }
    warpx::hooks::spawn_handoff();
}

/// Run on directory change. Returns doob overdue count for the new directory's repo.
pub fn on_cwd_change(new_cwd: &Path) -> Option<String> {
    if !FeatureFlag::JoeMode.is_enabled() {
        return None;
    }
    warpx::hooks::on_cwd_change(new_cwd)
}
