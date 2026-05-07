//! File-watcher auto-rerun module.
//!
//! Watches files mentioned in the last command's build diagnostics for
//! changes and emits a [`WatchEvent::FilesChanged`] event after debouncing.
//!
//! Gated behind `FeatureFlag::OzAutoRerun` in `DOGFOOD_FLAGS`.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use crate::terminal::build_parser::BuildDiagnostic;
use instant::Instant;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The current watch-mode state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchMode {
    /// Actively watching files.
    Watching,
    /// User dismissed; no further events will be emitted.
    Dismissed,
    /// Auto-rerun on every change until explicitly stopped.
    Always,
}

/// Events emitted by [`RerunWatcher`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchEvent {
    /// One or more watched files changed (after debounce).
    FilesChanged,
}

/// Debounce window: coalesce file-change signals within this duration.
pub const DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);

/// Watches a set of file paths extracted from build diagnostics.
///
/// Call [`RerunWatcher::notify_change`] whenever an fs event arrives.
/// Poll [`RerunWatcher::drain_event`] to receive debounced [`WatchEvent`]s.
pub struct RerunWatcher {
    watched_paths: HashSet<PathBuf>,
    mode: WatchMode,
    /// Timestamp of the first unprocessed change in the current debounce window.
    pending_since: Option<Instant>,
}

impl RerunWatcher {
    /// Create a watcher from a list of build diagnostics.
    ///
    /// Only diagnostics that carry a `file` path are watched.
    pub fn from_diagnostics(diagnostics: &[BuildDiagnostic]) -> Self {
        let watched_paths = diagnostics.iter().filter_map(|d| d.file.clone()).collect();
        Self {
            watched_paths,
            mode: WatchMode::Watching,
            pending_since: None,
        }
    }

    /// The set of paths currently being watched.
    pub fn watched_paths(&self) -> &HashSet<PathBuf> {
        &self.watched_paths
    }

    /// The current watch mode.
    pub fn mode(&self) -> &WatchMode {
        &self.mode
    }

    /// Signal that one or more watched files changed at `now`.
    ///
    /// Does nothing when mode is [`WatchMode::Dismissed`].
    pub fn notify_change(&mut self, now: Instant) {
        if self.mode == WatchMode::Dismissed {
            return;
        }
        if self.pending_since.is_none() {
            self.pending_since = Some(now);
        }
    }

    /// Dismiss the watcher. Future [`notify_change`] calls are no-ops
    /// and no further events will be emitted.
    pub fn dismiss(&mut self) {
        self.mode = WatchMode::Dismissed;
        self.pending_since = None;
    }

    /// Switch to "Always" mode — auto-rerun on every debounced change.
    pub fn set_always(&mut self) {
        self.mode = WatchMode::Always;
    }

    /// Drain a pending [`WatchEvent`] if the debounce window has elapsed.
    ///
    /// Returns `Some(WatchEvent::FilesChanged)` at most once per debounce
    /// window. Pass the current time via `now` so callers control the clock
    /// (enables deterministic unit tests).
    pub fn drain_event(&mut self, now: Instant) -> Option<WatchEvent> {
        if self.mode == WatchMode::Dismissed {
            return None;
        }
        let pending_since = self.pending_since?;
        if now.duration_since(pending_since) >= DEBOUNCE_WINDOW {
            self.pending_since = None;
            // In Watching mode (one-shot), we don't auto-dismiss here;
            // the caller (UI) will call dismiss() after user responds "No".
            Some(WatchEvent::FilesChanged)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — written BEFORE the implementation (red-green-refactor)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
