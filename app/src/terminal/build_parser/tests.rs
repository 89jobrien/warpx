use super::*;

// ---------------------------------------------------------------------------
// detect_tool tests
// ---------------------------------------------------------------------------

#[test]
fn detect_cargo_build() {
    let p = BuildOutputParser::detect("cargo build --release");
    assert!(p.is_some());
    assert_eq!(p.unwrap().tool(), BuildTool::Cargo);
}

#[test]
fn detect_cargo_check() {
    let p = BuildOutputParser::detect("cargo check --workspace");
    assert_eq!(p.unwrap().tool(), BuildTool::Cargo);
}

#[test]
fn detect_cargo_clippy() {
    let p = BuildOutputParser::detect("cargo clippy -- -D warnings");
    assert_eq!(p.unwrap().tool(), BuildTool::Cargo);
}

#[test]
fn detect_go_build() {
    let p = BuildOutputParser::detect("go build ./...");
    assert_eq!(p.unwrap().tool(), BuildTool::GoBuild);
}

#[test]
fn detect_tsc() {
    let p = BuildOutputParser::detect("tsc --noEmit");
    assert_eq!(p.unwrap().tool(), BuildTool::Tsc);
}

#[test]
fn detect_tsc_npx() {
    let p = BuildOutputParser::detect("npx tsc");
    assert_eq!(p.unwrap().tool(), BuildTool::Tsc);
}

#[test]
fn detect_ruff() {
    let p = BuildOutputParser::detect("ruff check src/");
    assert_eq!(p.unwrap().tool(), BuildTool::Ruff);
}

#[test]
fn detect_unknown_returns_none() {
    assert!(BuildOutputParser::detect("ls -la").is_none());
    assert!(BuildOutputParser::detect("echo hello").is_none());
    assert!(BuildOutputParser::detect("").is_none());
}

// ---------------------------------------------------------------------------
// cargo parser tests
// ---------------------------------------------------------------------------

#[test]
fn parse_cargo_error_with_location() {
    let output = [
        "error[E0382]: borrow of moved value: `x`",
        " --> src/main.rs:10:5",
        "  |",
        "10 |     println!(\"{}\", x);",
    ];
    let parser = BuildOutputParser::detect("cargo build").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, Some("src/main.rs".into()));
    assert_eq!(d.line, Some(10));
    assert_eq!(d.col, Some(5));
    assert!(d.message.contains("borrow of moved value"));
}

#[test]
fn parse_cargo_warning_with_location() {
    let output = ["warning: unused variable `y`", " --> src/lib.rs:3:9"];
    let parser = BuildOutputParser::detect("cargo check").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Warning);
    assert_eq!(d.file, Some("src/lib.rs".into()));
    assert_eq!(d.line, Some(3));
    assert_eq!(d.col, Some(9));
}

#[test]
fn parse_cargo_note() {
    let output = ["note: run with RUST_BACKTRACE=1 for a backtrace", ""];
    let parser = BuildOutputParser::detect("cargo test").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Severity::Note);
}

#[test]
fn parse_cargo_multiple_errors() {
    let output = [
        "error[E0308]: mismatched types",
        " --> src/a.rs:5:3",
        "",
        "error[E0425]: cannot find value `foo` in this scope",
        " --> src/b.rs:12:1",
        "",
    ];
    let parser = BuildOutputParser::detect("cargo clippy").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].file, Some("src/a.rs".into()));
    assert_eq!(diags[1].file, Some("src/b.rs".into()));
}

// ---------------------------------------------------------------------------
// go parser tests
// ---------------------------------------------------------------------------

#[test]
fn parse_go_error() {
    let output = ["./main.go:12:5: undefined: foo"];
    let parser = BuildOutputParser::detect("go build ./...").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, Some("./main.go".into()));
    assert_eq!(d.line, Some(12));
    assert_eq!(d.col, Some(5));
    assert_eq!(d.message, "undefined: foo");
}

#[test]
fn parse_go_multiple() {
    let output = [
        "./pkg/util.go:3:1: syntax error: unexpected semicolon",
        "./main.go:8:12: cannot use x (type int) as type string",
    ];
    let parser = BuildOutputParser::detect("go build").unwrap();
    let diags = parser.parse(output.iter().copied());
    assert_eq!(diags.len(), 2);
}

// ---------------------------------------------------------------------------
// tsc parser tests
// ---------------------------------------------------------------------------

#[test]
fn parse_tsc_error() {
    let output =
        ["src/index.ts(10,5): error TS2322: Type 'number' is not assignable to type 'string'."];
    let parser = BuildOutputParser::detect("tsc --noEmit").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, Some("src/index.ts".into()));
    assert_eq!(d.line, Some(10));
    assert_eq!(d.col, Some(5));
    assert!(d.message.contains("not assignable"));
}

#[test]
fn parse_tsc_warning() {
    let output = ["src/util.ts(3,1): warning TS6133: 'x' is declared but its value is never read."];
    let parser = BuildOutputParser::detect("tsc").unwrap();
    let diags = parser.parse(output.iter().copied());
    assert_eq!(diags[0].severity, Severity::Warning);
}

// ---------------------------------------------------------------------------
// ruff parser tests
// ---------------------------------------------------------------------------

#[test]
fn parse_ruff_error() {
    let output = ["src/foo.py:10:5: E501 Line too long (88 > 79 characters)"];
    let parser = BuildOutputParser::detect("ruff check src/").unwrap();
    let diags = parser.parse(output.iter().copied());

    assert_eq!(diags.len(), 1);
    let d = &diags[0];
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.file, Some("src/foo.py".into()));
    assert_eq!(d.line, Some(10));
    assert_eq!(d.col, Some(5));
    assert!(d.message.contains("E501"));
}

#[test]
fn parse_ruff_warning() {
    let output = ["src/bar.py:3:1: W291 trailing whitespace"];
    let parser = BuildOutputParser::detect("ruff check").unwrap();
    let diags = parser.parse(output.iter().copied());
    assert_eq!(diags[0].severity, Severity::Warning);
}

// ---------------------------------------------------------------------------
// counts helper
// ---------------------------------------------------------------------------

#[test]
fn counts_summary() {
    let diags = vec![
        BuildDiagnostic {
            file: None,
            line: None,
            col: None,
            severity: Severity::Error,
            message: "e1".into(),
        },
        BuildDiagnostic {
            file: None,
            line: None,
            col: None,
            severity: Severity::Error,
            message: "e2".into(),
        },
        BuildDiagnostic {
            file: None,
            line: None,
            col: None,
            severity: Severity::Warning,
            message: "w1".into(),
        },
    ];
    let (errors, warnings) = BuildOutputParser::counts(&diags);
    assert_eq!(errors, 2);
    assert_eq!(warnings, 1);
}
