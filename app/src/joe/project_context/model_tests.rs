use std::path::PathBuf;

use tempfile::TempDir;

use super::model::{GitSection, HandoffItem, HandoffSection, ProjectContextState, TodoSection};

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

pub(crate) const FIXTURE_CONTEXT_JSON: &str = r#"{
  "git": {
    "branch": "develop",
    "dirty_count": 2,
    "recent_commits": ["abc1234 feat: foo", "def5678 fix: bar"]
  },
  "ai": {
    "claude_md_files": ["/Users/joe/.claude/CLAUDE.md"],
    "mcp_servers": ["pieces", "personal-mcp"]
  },
  "handoff": {
    "items": [
      { "summary": "Implement foo", "priority": "P1", "status": "open" },
      { "summary": "Done thing",    "priority": "P2", "status": "done" }
    ]
  },
  "todos": {
    "pending": [
      { "name": "write tests", "priority": "P0", "status": "pending" }
    ]
  }
}"#;

/// Create a temp dir with `.ctx/context.json` at the given depth below root.
pub(crate) fn make_ctx_dir_at_depth(depth: usize) -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ctx_dir = tmp.path().join(".ctx");
    std::fs::create_dir_all(&ctx_dir).unwrap();
    std::fs::write(ctx_dir.join("context.json"), FIXTURE_CONTEXT_JSON).unwrap();

    let mut leaf = tmp.path().to_path_buf();
    for i in 0..depth {
        leaf = leaf.join(format!("level{i}"));
        std::fs::create_dir_all(&leaf).unwrap();
    }
    (tmp, leaf)
}

// ---------------------------------------------------------------------------
// Deserialization tests
// ---------------------------------------------------------------------------

#[test]
fn deserialize_full_context_json() {
    let state: ProjectContextState =
        serde_json::from_str(FIXTURE_CONTEXT_JSON).expect("valid JSON");

    assert_eq!(state.git.branch, "develop");
    assert_eq!(state.git.dirty_count, 2);
    assert_eq!(state.git.recent_commits.len(), 2);

    assert_eq!(state.ai.claude_md_files.len(), 1);
    assert_eq!(state.ai.mcp_servers, vec!["pieces", "personal-mcp"]);

    assert_eq!(state.handoff.items.len(), 2);
    assert_eq!(state.handoff.items[0].priority, "P1");

    assert_eq!(state.todos.pending.len(), 1);
    assert_eq!(state.todos.pending[0].name, "write tests");
}

#[test]
fn deserialize_empty_json_uses_defaults() {
    let state: ProjectContextState = serde_json::from_str("{}").expect("empty object");
    assert_eq!(state.git, GitSection::default());
    assert_eq!(state.handoff, HandoffSection::default());
    assert_eq!(state.todos, TodoSection::default());
}

#[test]
fn deserialize_partial_json_fills_missing_fields() {
    let json = r#"{ "git": { "branch": "main" } }"#;
    let state: ProjectContextState = serde_json::from_str(json).expect("partial JSON");
    assert_eq!(state.git.branch, "main");
    assert_eq!(state.git.dirty_count, 0);
    assert!(state.git.recent_commits.is_empty());
    assert!(state.ai.mcp_servers.is_empty());
}

#[test]
fn deserialize_unknown_fields_are_ignored() {
    let json = r#"{ "unknown_key": 42, "git": { "branch": "feat" } }"#;
    let state: ProjectContextState = serde_json::from_str(json).expect("extra fields");
    assert_eq!(state.git.branch, "feat");
}

// ---------------------------------------------------------------------------
// Priority badge logic
// ---------------------------------------------------------------------------

#[test]
fn handoff_item_priorities_roundtrip() {
    let items = [
        HandoffItem {
            summary: "urgent".into(),
            priority: "P0".into(),
            status: "open".into(),
        },
        HandoffItem {
            summary: "important".into(),
            priority: "P1".into(),
            status: "open".into(),
        },
        HandoffItem {
            summary: "normal".into(),
            priority: "P2".into(),
            status: "done".into(),
        },
    ];
    assert_eq!(items[0].priority, "P0");
    assert_eq!(items[1].priority, "P1");
    assert_eq!(items[2].status, "done");
}

// ---------------------------------------------------------------------------
// Walk-up path resolution
// ---------------------------------------------------------------------------

#[test]
fn walk_up_finds_ctx_at_root() {
    let (_tmp, leaf) = make_ctx_dir_at_depth(0);
    let candidate = leaf.join(".ctx").join("context.json");
    assert!(
        candidate.exists(),
        "context.json should exist at root depth"
    );
}

#[test]
fn walk_up_finds_ctx_three_levels_deep() {
    let (tmp, leaf) = make_ctx_dir_at_depth(3);
    let found = walk_up_for_ctx(&leaf);
    assert_eq!(
        found.as_deref(),
        Some(tmp.path().join(".ctx").join("context.json")).as_deref(),
        "walk-up should find .ctx/context.json at root"
    );
}

#[test]
fn walk_up_returns_none_when_no_ctx_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let leaf = tmp.path().join("a").join("b");
    std::fs::create_dir_all(&leaf).unwrap();
    let found = walk_up_for_ctx(&leaf);
    assert!(found.is_none(), "should not find .ctx when absent");
}

fn walk_up_for_ctx(start: &std::path::Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        let candidate = current.join(".ctx").join("context.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}
