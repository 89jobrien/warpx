#![allow(dead_code)]

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
}

#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: String,
    pub status: TestStatus,
    /// Captured stdout/stderr for failed tests.
    pub output: Option<String>,
    /// Command that re-runs this single test (injected into the active pane).
    pub rerun_command: String,
}

#[derive(Clone, Debug, Default)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
}

impl TestSuite {
    pub fn passed(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.status == TestStatus::Pass)
            .count()
    }

    pub fn failed(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.status == TestStatus::Fail)
            .count()
    }

    pub fn skipped(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.status == TestStatus::Skip)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct TestRunnerModel {
    pub suites: Vec<TestSuite>,
}

impl TestRunnerModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total counts across all suites.
    pub fn summary(&self) -> (usize, usize, usize) {
        let passed = self.suites.iter().map(|s| s.passed()).sum();
        let failed = self.suites.iter().map(|s| s.failed()).sum();
        let skipped = self.suites.iter().map(|s| s.skipped()).sum();
        (passed, failed, skipped)
    }

    // -----------------------------------------------------------------------
    // nextest JSON parsing
    // -----------------------------------------------------------------------

    /// Parse one or more lines of `cargo nextest … --message-format json` output.
    /// Each line is a JSON object; we accumulate into suites keyed by `suite_id`.
    pub fn parse_nextest_json(&mut self, input: &str) {
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<NextestEvent>(line) {
                self.handle_nextest_event(event);
            }
        }
    }

    fn handle_nextest_event(&mut self, event: NextestEvent) {
        match event.event_type.as_str() {
            "test-finished" => {
                let suite_name = event.suite_id.unwrap_or_else(|| "default".to_string());
                let test_name = event.test_name.unwrap_or_else(|| "unknown".to_string());
                let status = match event.outcome.as_deref() {
                    Some("passed") => TestStatus::Pass,
                    Some("failed") | Some("exec-fail") => TestStatus::Fail,
                    Some("skipped") => TestStatus::Skip,
                    _ => TestStatus::Skip,
                };
                let output = event.stdout.or(event.stderr);
                let rerun_command =
                    format!("cargo nextest run --test-name {}", shell_escape(&test_name));
                let case = TestCase {
                    name: test_name,
                    status,
                    output,
                    rerun_command,
                };
                let suite = self.get_or_create_suite(&suite_name);
                suite.tests.push(case);
            }
            "suite-started" => {
                if let Some(id) = event.suite_id {
                    self.get_or_create_suite(&id);
                }
            }
            _ => {}
        }
    }

    fn get_or_create_suite(&mut self, name: &str) -> &mut TestSuite {
        if let Some(pos) = self.suites.iter().position(|s| s.name == name) {
            &mut self.suites[pos]
        } else {
            self.suites.push(TestSuite {
                name: name.to_string(),
                tests: Vec::new(),
            });
            self.suites.last_mut().expect("just pushed")
        }
    }

    // -----------------------------------------------------------------------
    // go test JSON parsing
    // -----------------------------------------------------------------------

    /// Parse one or more lines of `go test -json` output.
    pub fn parse_go_test_json(&mut self, input: &str) {
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<GoTestEvent>(line) {
                self.handle_go_test_event(event);
            }
        }
    }

    fn handle_go_test_event(&mut self, event: GoTestEvent) {
        // action == "pass" | "fail" | "skip" applies to a finished test
        let action = event.action.as_str();
        if !matches!(action, "pass" | "fail" | "skip") {
            return;
        }
        // Package-level events have no Test field — skip them as individual cases.
        let test_name = match event.test {
            Some(t) => t,
            None => return,
        };
        let suite_name = event.package.unwrap_or_else(|| "default".to_string());
        let status = match action {
            "pass" => TestStatus::Pass,
            "fail" => TestStatus::Fail,
            "skip" => TestStatus::Skip,
            _ => TestStatus::Skip,
        };
        let rerun_command = format!("go test -run {} {}", shell_escape(&test_name), suite_name);
        let case = TestCase {
            name: test_name,
            status,
            output: event.output,
            rerun_command,
        };
        let suite = self.get_or_create_suite(&suite_name);
        suite.tests.push(case);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

// ---------------------------------------------------------------------------
// Deserialization types — private
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct NextestEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "suite-id")]
    suite_id: Option<String>,
    #[serde(rename = "test-name")]
    test_name: Option<String>,
    outcome: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Deserialize)]
struct GoTestEvent {
    #[serde(rename = "Action")]
    action: String,
    #[serde(rename = "Package")]
    package: Option<String>,
    #[serde(rename = "Test")]
    test: Option<String>,
    #[serde(rename = "Output")]
    output: Option<String>,
}
