//! `BuildDiagnosticOverlay` — renders a collapsible list of build diagnostics
//! below a command block.
//!
//! Gated behind `FeatureFlag::OzBuildParser`.

use warp_core::features::FeatureFlag;

use crate::terminal::build_parser::{BuildDiagnostic, Severity};

/// Returns a human-readable summary badge, e.g. "3 errors, 2 warnings".
///
/// Returns `None` when the feature flag is off or when `diagnostics` is empty.
pub fn diagnostic_badge(diagnostics: &[BuildDiagnostic]) -> Option<String> {
    if !FeatureFlag::OzBuildParser.is_enabled() {
        return None;
    }
    if diagnostics.is_empty() {
        return None;
    }

    let (errors, warnings) = crate::terminal::build_parser::BuildOutputParser::counts(diagnostics);

    let mut parts = Vec::new();
    if errors > 0 {
        parts.push(format!(
            "{} {}",
            errors,
            if errors == 1 { "error" } else { "errors" }
        ));
    }
    if warnings > 0 {
        parts.push(format!(
            "{} {}",
            warnings,
            if warnings == 1 { "warning" } else { "warnings" }
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

/// Returns the ANSI color name for a diagnostic severity, for use in terminal
/// color-coded output.
pub fn severity_color(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "red",
        Severity::Warning => "yellow",
        Severity::Note => "blue",
    }
}

/// Returns a repeat-count badge string (e.g. `"x3"`) when the diagnostic
/// appeared more than once, or `None` when count is 1.
pub fn repeat_badge(d: &BuildDiagnostic) -> Option<String> {
    if d.count > 1 {
        Some(format!("x{}", d.count))
    } else {
        None
    }
}

/// Formats a single `BuildDiagnostic` as a display string.
///
/// When `d.count > 1` a repeat badge is appended, e.g.:
/// `error  src/main.rs:10:5  borrow of moved value: `x`  x3`
pub fn format_diagnostic(d: &BuildDiagnostic) -> String {
    let location = match (&d.file, d.line, d.col) {
        (Some(f), Some(ln), Some(col)) => format!("{}:{}:{}", f.display(), ln, col),
        (Some(f), Some(ln), None) => format!("{}:{}", f.display(), ln),
        (Some(f), None, _) => f.display().to_string(),
        (None, _, _) => String::new(),
    };

    let base = if location.is_empty() {
        format!("{}  {}", d.severity.label(), d.message)
    } else {
        format!("{}  {}  {}", d.severity.label(), location, d.message)
    };

    if let Some(badge) = repeat_badge(d) {
        format!("{base}  {badge}")
    } else {
        base
    }
}

// ---------------------------------------------------------------------------
// Stub: BuildDiagnosticOverlay
//
// Full WarpUI Entity-Component wiring is deferred. This struct holds the data
// needed for rendering and exposes helpers used by the block toolbar.
// ---------------------------------------------------------------------------

/// Holds parsed build diagnostics for a single command block.
///
/// Construct via [`BuildDiagnosticOverlay::new`] when a command finishes and
/// `FeatureFlag::OzBuildParser` is enabled.
pub struct BuildDiagnosticOverlay {
    diagnostics: Vec<BuildDiagnostic>,
    collapsed: bool,
}

impl BuildDiagnosticOverlay {
    /// Create a new overlay from a list of diagnostics.
    pub fn new(diagnostics: Vec<BuildDiagnostic>) -> Self {
        Self {
            diagnostics,
            collapsed: false,
        }
    }

    /// Whether there are any diagnostics to show.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Toggle collapsed/expanded state.
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// Toolbar badge text, e.g. "3 errors, 2 warnings".
    pub fn badge(&self) -> Option<String> {
        diagnostic_badge(&self.diagnostics)
    }

    /// Formatted lines for rendering the expanded list.
    pub fn formatted_lines(&self) -> Vec<String> {
        if self.collapsed {
            return Vec::new();
        }
        self.diagnostics.iter().map(format_diagnostic).collect()
    }

    /// Stub click handler — opens the file in `$EDITOR` at line:col.
    ///
    /// TODO: wire through `AllowOpeningFileLinksUsingEditorEnv` infrastructure.
    pub fn on_click(&self, index: usize) {
        let Some(d) = self.diagnostics.get(index) else {
            return;
        };
        let Some(file) = &d.file else { return };

        // Stub: log the intent; real implementation will invoke the editor open action.
        let _ = (file, d.line, d.col);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::terminal::build_parser::Severity;

    fn make_diag(
        severity: Severity,
        file: &str,
        line: u32,
        col: u32,
        msg: &str,
    ) -> BuildDiagnostic {
        BuildDiagnostic {
            file: Some(PathBuf::from(file)),
            line: Some(line),
            col: Some(col),
            severity,
            message: msg.to_string(),
            count: 1,
        }
    }

    #[test]
    fn badge_none_when_empty() {
        let overlay = BuildDiagnosticOverlay::new(vec![]);
        assert!(overlay.badge().is_none());
    }

    #[test]
    fn format_diagnostic_full() {
        let d = make_diag(Severity::Error, "src/main.rs", 10, 5, "type mismatch");
        let s = format_diagnostic(&d);
        assert!(s.contains("error"));
        assert!(s.contains("src/main.rs:10:5"));
        assert!(s.contains("type mismatch"));
    }

    #[test]
    fn repeat_badge_none_when_count_one() {
        let d = make_diag(Severity::Error, "a.rs", 1, 1, "msg");
        assert!(repeat_badge(&d).is_none());
    }

    #[test]
    fn repeat_badge_shows_count_when_gt_one() {
        let mut d = make_diag(Severity::Error, "a.rs", 1, 1, "msg");
        d.count = 4;
        assert_eq!(repeat_badge(&d), Some("x4".to_string()));
    }

    #[test]
    fn format_diagnostic_includes_badge_when_count_gt_one() {
        let mut d = make_diag(Severity::Error, "src/main.rs", 10, 5, "type mismatch");
        d.count = 3;
        let s = format_diagnostic(&d);
        assert!(s.ends_with("x3"), "expected badge at end, got: {s}");
    }

    #[test]
    fn toggle_collapsed() {
        let mut overlay =
            BuildDiagnosticOverlay::new(vec![make_diag(Severity::Warning, "a.rs", 1, 1, "w")]);
        assert!(!overlay.is_collapsed());
        overlay.toggle_collapsed();
        assert!(overlay.is_collapsed());
        assert!(overlay.formatted_lines().is_empty());
        overlay.toggle_collapsed();
        assert_eq!(overlay.formatted_lines().len(), 1);
    }
}
