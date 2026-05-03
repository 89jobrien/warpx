//! JoeMode session lifecycle hooks.
//!
//! Fires `hj detect` on session open and `godmode handoff` on session close.
//! All operations are best-effort — failures are logged to stderr, never fatal.
//! Gated behind FeatureFlag::JoeMode.

use std::path::Path;
use std::process::Command;

use crate::features::FeatureFlag;

/// Run on terminal tab/session open. Detects handoff items for the cwd.
/// Output is returned as a string to be displayed as a banner (or ignored).
pub fn on_session_open(cwd: &Path) -> Option<String> {
    if !FeatureFlag::JoeMode.is_enabled() {
        return None;
    }

    let output = Command::new("hj").arg("detect").arg(cwd).output().ok()?;

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

/// Run on session close. Triggers godmode handoff validation (best-effort).
pub fn on_session_close(cwd: &Path) {
    if !FeatureFlag::JoeMode.is_enabled() {
        return;
    }

    let _ = Command::new("godmode")
        .arg("handoff")
        .current_dir(cwd)
        .output();
}

/// Fire-and-forget: runs `godmode handon` in background on tab open.
/// Gated by FeatureFlag::OzSessionHooks. Never blocks the UI thread.
pub fn spawn_handon() {
    if !FeatureFlag::OzSessionHooks.is_enabled() {
        return;
    }
    let _ = Command::new("godmode")
        .arg("handon")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Fire-and-forget: runs `godmode handoff` in background on tab close.
/// Gated by FeatureFlag::OzSessionHooks. Never blocks the UI thread.
pub fn spawn_handoff() {
    if !FeatureFlag::OzSessionHooks.is_enabled() {
        return;
    }
    let _ = Command::new("godmode")
        .arg("handoff")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Run on directory change. Returns doob overdue count for the new directory's repo.
pub fn on_cwd_change(new_cwd: &Path) -> Option<String> {
    if !FeatureFlag::JoeMode.is_enabled() {
        return None;
    }

    let cache = crate::joe::read_doob_cache()?;
    let repo = new_cwd.file_name()?.to_string_lossy().to_string();
    let repo_count = cache.overdue_by_repo.get(&repo).copied().unwrap_or(0);

    if repo_count > 0 {
        Some(format!("doob: {repo_count} overdue in {repo}"))
    } else {
        None
    }
}
