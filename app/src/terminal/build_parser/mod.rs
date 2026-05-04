//! Build output parser — detects build tool from command prefix and parses
//! compiler/linter output into structured diagnostics.
//!
//! Gated behind `FeatureFlag::OzBuildParser`.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

/// Severity level of a build diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    /// Returns a short label suitable for display.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

/// A single parsed build diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildDiagnostic {
    /// Source file path, if present in the output.
    pub file: Option<PathBuf>,
    /// 1-based line number, if present.
    pub line: Option<u32>,
    /// 1-based column number, if present.
    pub col: Option<u32>,
    pub severity: Severity,
    pub message: String,
}

/// Which build tool was detected from the command prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildTool {
    Cargo,
    GoBuild,
    Tsc,
    Ruff,
}

/// Detects the build tool from a command line prefix and parses its output into
/// a list of [`BuildDiagnostic`]s.
///
/// Only activate when `FeatureFlag::OzBuildParser.is_enabled()` returns `true`.
pub struct BuildOutputParser {
    tool: BuildTool,
}

impl BuildOutputParser {
    /// Returns `Some(parser)` when `command_prefix` starts with a known build
    /// tool invocation, or `None` if the command is not recognised.
    pub fn detect(command_prefix: &str) -> Option<Self> {
        let tool = detect_tool(command_prefix)?;
        Some(Self { tool })
    }

    /// Which tool was detected.
    pub fn tool(&self) -> BuildTool {
        self.tool
    }

    /// Parse the output lines of a build command into structured diagnostics.
    pub fn parse<'a>(&self, lines: impl Iterator<Item = &'a str>) -> Vec<BuildDiagnostic> {
        match self.tool {
            BuildTool::Cargo => parse_cargo(lines),
            BuildTool::GoBuild => parse_go(lines),
            BuildTool::Tsc => parse_tsc(lines),
            BuildTool::Ruff => parse_ruff(lines),
        }
    }

    /// Convenience summary: (error_count, warning_count).
    pub fn counts(diagnostics: &[BuildDiagnostic]) -> (usize, usize) {
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        (errors, warnings)
    }
}

// ---------------------------------------------------------------------------
// Tool detection
// ---------------------------------------------------------------------------

fn detect_tool(cmd: &str) -> Option<BuildTool> {
    let cmd = cmd.trim();
    if is_cargo_command(cmd) {
        return Some(BuildTool::Cargo);
    }
    if cmd.starts_with("go build")
        || cmd.starts_with("go test")
        || cmd.starts_with("go run")
        || cmd.starts_with("go vet")
    {
        return Some(BuildTool::GoBuild);
    }
    if cmd.starts_with("tsc") || cmd.starts_with("npx tsc") {
        return Some(BuildTool::Tsc);
    }
    if cmd.starts_with("ruff check") || cmd.starts_with("ruff") {
        return Some(BuildTool::Ruff);
    }
    None
}

fn is_cargo_command(cmd: &str) -> bool {
    // Match: cargo build, cargo check, cargo test, cargo clippy, cargo nextest
    let prefixes = [
        "cargo build",
        "cargo check",
        "cargo test",
        "cargo clippy",
        "cargo nextest",
        "cargo run",
    ];
    prefixes.iter().any(|p| cmd.starts_with(p))
}

// ---------------------------------------------------------------------------
// cargo / rustc parser
//
// rustc emits lines like:
//   error[E0382]: borrow of moved value: `x`
//    --> src/main.rs:10:5
//   warning: unused variable `y`
//    --> src/lib.rs:3:9
//   note: ...
// ---------------------------------------------------------------------------

fn parse_cargo<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<BuildDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut pending: Option<(Severity, String)> = None;

    for line in lines {
        // Detect severity prefix
        if let Some(rest) = strip_severity_prefix(line) {
            pending = Some(rest);
            continue;
        }

        // Arrow line " --> file:line:col"
        if let Some((file, ln, col)) = parse_rustc_arrow(line) {
            if let Some((severity, message)) = pending.take() {
                diagnostics.push(BuildDiagnostic {
                    file: Some(file),
                    line: ln,
                    col,
                    severity,
                    message,
                });
            }
            continue;
        }

        // If we see a blank line with pending, flush without location
        if line.trim().is_empty() {
            if let Some((severity, message)) = pending.take() {
                diagnostics.push(BuildDiagnostic {
                    file: None,
                    line: None,
                    col: None,
                    severity,
                    message,
                });
            }
        }
    }

    // Flush any remaining pending
    if let Some((severity, message)) = pending.take() {
        diagnostics.push(BuildDiagnostic {
            file: None,
            line: None,
            col: None,
            severity,
            message,
        });
    }

    diagnostics
}

/// Strips "error", "error[E...]", "warning", "note" prefix.
/// Returns `Some((severity, message_text))`.
fn strip_severity_prefix(line: &str) -> Option<(Severity, String)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("error[") || trimmed.starts_with("error:") {
        let msg = after_colon(trimmed).trim().to_string();
        return Some((Severity::Error, msg));
    }
    if trimmed.starts_with("warning:") {
        let msg = after_colon(trimmed).trim().to_string();
        return Some((Severity::Warning, msg));
    }
    if trimmed.starts_with("note:") {
        let msg = after_colon(trimmed).trim().to_string();
        return Some((Severity::Note, msg));
    }
    None
}

fn after_colon(s: &str) -> &str {
    s.find(':').map_or("", |i| &s[i + 1..])
}

/// Parses ` --> src/main.rs:10:5` into `(path, Some(10), Some(5))`.
fn parse_rustc_arrow(line: &str) -> Option<(PathBuf, Option<u32>, Option<u32>)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("-->")?;
    let rest = rest.trim();
    parse_file_line_col(rest)
}

// ---------------------------------------------------------------------------
// go build parser
//
// go build emits:
//   ./main.go:12:5: undefined: foo
//   ./pkg/util.go:3:1: syntax error: unexpected semicolon
// ---------------------------------------------------------------------------

fn parse_go<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<BuildDiagnostic> {
    let mut diagnostics = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // file:line:col: message
        if let Some(diag) = parse_go_line(line) {
            diagnostics.push(diag);
        }
    }

    diagnostics
}

fn parse_go_line(line: &str) -> Option<BuildDiagnostic> {
    // Pattern: <path>:<line>:<col>: <message>
    // The path may start with ./ or be absolute.
    let (location, message) = split_go_location(line)?;
    let (file, ln, col) = parse_file_line_col(location)?;

    // go vet uses "# pkg" prefix — not an error line, skip
    if line.starts_with('#') {
        return None;
    }

    // Severity: go build errors are always errors unless they contain "warning"
    let severity = if message.to_lowercase().starts_with("warning") {
        Severity::Warning
    } else {
        Severity::Error
    };

    Some(BuildDiagnostic {
        file: Some(file),
        line: ln,
        col,
        severity,
        message: message.to_string(),
    })
}

/// Splits `./foo.go:3:1: message` into `("./foo.go:3:1", "message")`.
fn split_go_location(line: &str) -> Option<(&str, &str)> {
    // Find the third colon (after path, line, col)
    let mut colon_count = 0;
    for (i, ch) in line.char_indices() {
        if ch == ':' {
            colon_count += 1;
            if colon_count == 3 {
                return Some((&line[..i], line[i + 1..].trim()));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// tsc (TypeScript compiler) parser
//
// tsc emits:
//   src/index.ts(10,5): error TS2322: Type 'number' is not assignable to type 'string'.
//   src/index.ts(3,1): warning TS...
// ---------------------------------------------------------------------------

fn parse_tsc<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<BuildDiagnostic> {
    let mut diagnostics = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(diag) = parse_tsc_line(line) {
            diagnostics.push(diag);
        }
    }

    diagnostics
}

fn parse_tsc_line(line: &str) -> Option<BuildDiagnostic> {
    // Pattern: <file>(<line>,<col>): <severity> TS<code>: <message>
    let paren_open = line.find('(')?;
    let paren_close = line.find(')')?;
    if paren_close <= paren_open {
        return None;
    }

    let file = PathBuf::from(&line[..paren_open]);
    let coords = &line[paren_open + 1..paren_close];
    let rest = line[paren_close + 1..].trim();
    let rest = rest.strip_prefix(':')?;
    let rest = rest.trim();

    let (ln, col) = parse_line_col(coords);

    let severity = if rest.starts_with("error") {
        Severity::Error
    } else if rest.starts_with("warning") {
        Severity::Warning
    } else {
        return None;
    };

    let message = rest
        .find(':')
        .map(|i| rest[i + 1..].trim().to_string())
        .unwrap_or_else(|| rest.to_string());

    Some(BuildDiagnostic {
        file: Some(file),
        line: ln,
        col,
        severity,
        message,
    })
}

// ---------------------------------------------------------------------------
// ruff (Python linter) parser
//
// ruff emits:
//   src/foo.py:10:5: E501 Line too long (88 > 79 characters)
//   src/bar.py:3:1: W291 trailing whitespace
// ---------------------------------------------------------------------------

fn parse_ruff<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<BuildDiagnostic> {
    let mut diagnostics = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(diag) = parse_ruff_line(line) {
            diagnostics.push(diag);
        }
    }

    diagnostics
}

fn parse_ruff_line(line: &str) -> Option<BuildDiagnostic> {
    // Pattern: <file>:<line>:<col>: <code> <message>
    // ruff uses the same colon-separated format as go build for the location.
    let (location, rest) = split_go_location(line)?;
    let (file, ln, col) = parse_file_line_col(location)?;

    // Ruff rule codes: E/W = warning-class, F/B/C/N/D/I/etc. = error-class
    let severity = if rest.starts_with('W') {
        Severity::Warning
    } else {
        Severity::Error
    };

    Some(BuildDiagnostic {
        file: Some(file),
        line: ln,
        col,
        severity,
        message: rest.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parse `file:line:col` or `file:line` into components.
fn parse_file_line_col(s: &str) -> Option<(PathBuf, Option<u32>, Option<u32>)> {
    // Split from the end to handle paths with colons (Windows drive letters).
    let parts: Vec<&str> = s.rsplitn(3, ':').collect();
    match parts.as_slice() {
        [col_str, line_str, file] => {
            let col = col_str.trim().parse::<u32>().ok();
            let ln = line_str.trim().parse::<u32>().ok();
            if col.is_some() || ln.is_some() {
                Some((PathBuf::from(file), ln, col))
            } else {
                Some((PathBuf::from(s), None, None))
            }
        }
        [line_str, file] => {
            let ln = line_str.trim().parse::<u32>().ok();
            Some((PathBuf::from(file), ln, None))
        }
        _ => Some((PathBuf::from(s), None, None)),
    }
}

/// Parse `"10,5"` → `(Some(10), Some(5))`.
fn parse_line_col(s: &str) -> (Option<u32>, Option<u32>) {
    let mut parts = s.splitn(2, ',');
    let ln = parts.next().and_then(|p| p.trim().parse::<u32>().ok());
    let col = parts.next().and_then(|p| p.trim().parse::<u32>().ok());
    (ln, col)
}
