use super::model::{TestRunnerModel, TestStatus};

// ---------------------------------------------------------------------------
// failure_record helpers
// ---------------------------------------------------------------------------

#[test]
fn test_failure_record_with_failures() {
    let mut model = TestRunnerModel::new();
    model.parse_nextest_json(
        r#"{"type":"test-finished","suite-id":"s","test-name":"a","outcome":"failed"}"#,
    );
    model.parse_nextest_json(
        r#"{"type":"test-finished","suite-id":"s","test-name":"b","outcome":"passed"}"#,
    );
    let record = model
        .failure_record("cargo nextest run")
        .expect("should have failures");
    assert_eq!(record.test_names, vec!["a"]);
    assert_eq!(record.rerun_command, "cargo nextest run");
    assert!(record.timestamp_secs > 0);
}

#[test]
fn test_failure_record_all_pass_returns_none() {
    let mut model = TestRunnerModel::new();
    model.parse_nextest_json(
        r#"{"type":"test-finished","suite-id":"s","test-name":"a","outcome":"passed"}"#,
    );
    assert!(
        model.failure_record("cargo nextest run").is_none(),
        "no failures → None"
    );
}

// ---------------------------------------------------------------------------
// nextest JSON parsing
// ---------------------------------------------------------------------------

#[test]
fn test_nextest_pass() {
    let mut model = TestRunnerModel::new();
    let json = r#"{"type":"test-finished","suite-id":"warp_core","test-name":"foo::bar","outcome":"passed"}"#;
    model.parse_nextest_json(json);
    assert_eq!(model.suites.len(), 1);
    let suite = &model.suites[0];
    assert_eq!(suite.name, "warp_core");
    assert_eq!(suite.tests.len(), 1);
    assert_eq!(suite.tests[0].status, TestStatus::Pass);
    assert_eq!(suite.tests[0].name, "foo::bar");
}

#[test]
fn test_nextest_fail_with_output() {
    let mut model = TestRunnerModel::new();
    let json = r#"{"type":"test-finished","suite-id":"warp_core","test-name":"foo::fail","outcome":"failed","stderr":"assertion failed"}"#;
    model.parse_nextest_json(json);
    let test = &model.suites[0].tests[0];
    assert_eq!(test.status, TestStatus::Fail);
    assert_eq!(test.output.as_deref(), Some("assertion failed"));
}

#[test]
fn test_nextest_skip() {
    let mut model = TestRunnerModel::new();
    let json = r#"{"type":"test-finished","suite-id":"warp_core","test-name":"foo::skip","outcome":"skipped"}"#;
    model.parse_nextest_json(json);
    assert_eq!(model.suites[0].tests[0].status, TestStatus::Skip);
}

#[test]
fn test_nextest_multiple_suites() {
    let mut model = TestRunnerModel::new();
    let lines = [
        r#"{"type":"test-finished","suite-id":"crate_a","test-name":"a::one","outcome":"passed"}"#,
        r#"{"type":"test-finished","suite-id":"crate_b","test-name":"b::two","outcome":"failed"}"#,
        r#"{"type":"test-finished","suite-id":"crate_a","test-name":"a::three","outcome":"skipped"}"#,
    ];
    for line in &lines {
        model.parse_nextest_json(line);
    }
    assert_eq!(model.suites.len(), 2);
    let (passed, failed, skipped) = model.summary();
    assert_eq!(passed, 1);
    assert_eq!(failed, 1);
    assert_eq!(skipped, 1);
}

#[test]
fn test_nextest_ignores_unknown_events() {
    let mut model = TestRunnerModel::new();
    let json = r#"{"type":"run-started","foo":"bar"}"#;
    model.parse_nextest_json(json);
    assert!(model.suites.is_empty());
}

#[test]
fn test_nextest_rerun_command() {
    let mut model = TestRunnerModel::new();
    let json =
        r#"{"type":"test-finished","suite-id":"s","test-name":"my::test","outcome":"passed"}"#;
    model.parse_nextest_json(json);
    assert_eq!(
        model.suites[0].tests[0].rerun_command,
        "cargo nextest run --test-name my::test"
    );
}

// ---------------------------------------------------------------------------
// go test JSON parsing
// ---------------------------------------------------------------------------

#[test]
fn test_go_pass() {
    let mut model = TestRunnerModel::new();
    let json = r#"{"Action":"pass","Package":"github.com/foo/bar","Test":"TestFoo"}"#;
    model.parse_go_test_json(json);
    assert_eq!(model.suites.len(), 1);
    assert_eq!(model.suites[0].tests[0].status, TestStatus::Pass);
    assert_eq!(model.suites[0].tests[0].name, "TestFoo");
}

#[test]
fn test_go_fail_with_output() {
    let mut model = TestRunnerModel::new();
    let json =
        r#"{"Action":"fail","Package":"github.com/foo/bar","Test":"TestBad","Output":"FAIL\n"}"#;
    model.parse_go_test_json(json);
    assert_eq!(model.suites[0].tests[0].status, TestStatus::Fail);
    assert!(model.suites[0].tests[0].output.is_some());
}

#[test]
fn test_go_package_level_event_ignored() {
    // Package-level events have no "Test" field — should not produce a TestCase.
    let mut model = TestRunnerModel::new();
    let json = r#"{"Action":"pass","Package":"github.com/foo/bar"}"#;
    model.parse_go_test_json(json);
    assert!(model.suites.is_empty() || model.suites[0].tests.is_empty());
}

#[test]
fn test_go_skip() {
    let mut model = TestRunnerModel::new();
    let json = r#"{"Action":"skip","Package":"pkg","Test":"TestSkipped"}"#;
    model.parse_go_test_json(json);
    assert_eq!(model.suites[0].tests[0].status, TestStatus::Skip);
}

// ---------------------------------------------------------------------------
// Summary helpers
// ---------------------------------------------------------------------------

#[test]
fn test_suite_counts() {
    let mut model = TestRunnerModel::new();
    model.parse_nextest_json(
        r#"{"type":"test-finished","suite-id":"s","test-name":"a","outcome":"passed"}"#,
    );
    model.parse_nextest_json(
        r#"{"type":"test-finished","suite-id":"s","test-name":"b","outcome":"failed"}"#,
    );
    model.parse_nextest_json(
        r#"{"type":"test-finished","suite-id":"s","test-name":"c","outcome":"skipped"}"#,
    );
    let s = &model.suites[0];
    assert_eq!(s.passed(), 1);
    assert_eq!(s.failed(), 1);
    assert_eq!(s.skipped(), 1);
}
