use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::*;
use crate::terminal::build_parser::{BuildDiagnostic, Severity};

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

fn diag_with_file(path: &str) -> BuildDiagnostic {
    BuildDiagnostic {
        file: Some(PathBuf::from(path)),
        line: None,
        col: None,
        severity: Severity::Error,
        message: "test error".to_string(),
    }
}

fn diag_without_file() -> BuildDiagnostic {
    BuildDiagnostic {
        file: None,
        line: None,
        col: None,
        severity: Severity::Warning,
        message: "linker warning".to_string(),
    }
}

// ---------------------------------------------------------------------------
// File-path extraction from BuildDiagnostic list
// ---------------------------------------------------------------------------

#[test]
fn extract_file_paths_from_diagnostics() {
    let diagnostics = vec![
        diag_with_file("src/main.rs"),
        diag_without_file(),
        diag_with_file("src/lib.rs"),
    ];
    let watcher = RerunWatcher::from_diagnostics(&diagnostics);
    let paths = watcher.watched_paths();
    assert_eq!(
        paths.len(),
        2,
        "expected 2 watched paths, got {}",
        paths.len()
    );
    assert!(paths.contains(&PathBuf::from("src/main.rs")));
    assert!(paths.contains(&PathBuf::from("src/lib.rs")));
}

#[test]
fn no_paths_when_all_diagnostics_lack_file() {
    let diagnostics = vec![diag_without_file(), diag_without_file()];
    let watcher = RerunWatcher::from_diagnostics(&diagnostics);
    assert!(watcher.watched_paths().is_empty());
}

#[test]
fn deduplicates_repeated_file_paths() {
    let diagnostics = vec![diag_with_file("src/main.rs"), diag_with_file("src/main.rs")];
    let watcher = RerunWatcher::from_diagnostics(&diagnostics);
    assert_eq!(watcher.watched_paths().len(), 1);
}

// ---------------------------------------------------------------------------
// Debounce: rapid events within 500 ms → single FilesChanged
// ---------------------------------------------------------------------------

#[test]
fn debounce_coalesces_rapid_changes_into_single_event() {
    let diagnostics = vec![diag_with_file("src/main.rs")];
    let mut watcher = RerunWatcher::from_diagnostics(&diagnostics);

    let t0 = Instant::now();
    // Fire 3 change notifications within a 100 ms window.
    watcher.notify_change(t0);
    watcher.notify_change(t0 + Duration::from_millis(50));
    watcher.notify_change(t0 + Duration::from_millis(100));

    // At 300 ms the debounce window (500 ms) has not yet elapsed — no event.
    let early = t0 + Duration::from_millis(300);
    assert_eq!(
        watcher.drain_event(early),
        None,
        "event should not fire before debounce window"
    );

    // At 600 ms the debounce window has elapsed — exactly one event.
    let after = t0 + Duration::from_millis(600);
    assert_eq!(
        watcher.drain_event(after),
        Some(WatchEvent::FilesChanged),
        "expected FilesChanged after debounce window"
    );

    // A second drain immediately after returns None (event consumed).
    assert_eq!(watcher.drain_event(after), None);
}

// ---------------------------------------------------------------------------
// WatchMode::Dismissed stops future events
// ---------------------------------------------------------------------------

#[test]
fn dismissed_mode_stops_events() {
    let diagnostics = vec![diag_with_file("src/lib.rs")];
    let mut watcher = RerunWatcher::from_diagnostics(&diagnostics);

    let t0 = Instant::now();
    watcher.notify_change(t0);
    watcher.dismiss();

    // Even after the debounce window no event should fire.
    let after = t0 + Duration::from_millis(600);
    assert_eq!(
        watcher.drain_event(after),
        None,
        "dismissed watcher must not emit events"
    );
    assert_eq!(watcher.mode(), &WatchMode::Dismissed);
}

#[test]
fn notify_after_dismiss_is_noop() {
    let diagnostics = vec![diag_with_file("src/lib.rs")];
    let mut watcher = RerunWatcher::from_diagnostics(&diagnostics);

    watcher.dismiss();
    let t0 = Instant::now();
    watcher.notify_change(t0);

    let after = t0 + Duration::from_millis(600);
    assert_eq!(watcher.drain_event(after), None);
}

// ---------------------------------------------------------------------------
// WatchMode::Always — event fires, watcher stays active
// ---------------------------------------------------------------------------

#[test]
fn always_mode_allows_repeated_events() {
    let diagnostics = vec![diag_with_file("src/main.rs")];
    let mut watcher = RerunWatcher::from_diagnostics(&diagnostics);
    watcher.set_always();

    let t0 = Instant::now();
    watcher.notify_change(t0);

    let after = t0 + Duration::from_millis(600);
    assert_eq!(watcher.drain_event(after), Some(WatchEvent::FilesChanged));
    assert_eq!(watcher.mode(), &WatchMode::Always);

    // A second change should produce another event.
    let t1 = after + Duration::from_millis(10);
    watcher.notify_change(t1);
    let after2 = t1 + Duration::from_millis(600);
    assert_eq!(watcher.drain_event(after2), Some(WatchEvent::FilesChanged));
}
