use std::path::PathBuf;

use tempfile::TempDir;

use super::{GitSection, HandoffItem, HandoffSection, ProjectContextState, TodoSection};

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
/// Returns (TempDir, path_to_leaf_dir) where `leaf` is depth directories below root.
pub(crate) fn make_ctx_dir_at_depth(depth: usize) -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ctx_dir = tmp.path().join(".ctx");
    std::fs::create_dir_all(&ctx_dir).unwrap();
    std::fs::write(ctx_dir.join("context.json"), FIXTURE_CONTEXT_JSON).unwrap();

    // Build a leaf dir `depth` levels below root (without .ctx).
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
    let items = vec![
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
// Walk-up path resolution (tests resolve_context_path indirectly via env)
// ---------------------------------------------------------------------------

#[test]
fn walk_up_finds_ctx_at_root() {
    let (_tmp, leaf) = make_ctx_dir_at_depth(0);
    // .ctx is directly in root — leaf == root
    let candidate = leaf.join(".ctx").join("context.json");
    assert!(
        candidate.exists(),
        "context.json should exist at root depth"
    );
}

#[test]
fn walk_up_finds_ctx_three_levels_deep() {
    let (tmp, leaf) = make_ctx_dir_at_depth(3);
    // Walk from leaf upward manually — mirrors resolve_context_path logic
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
    // No .ctx dir anywhere in the tree
    let found = walk_up_for_ctx(&leaf);
    assert!(found.is_none(), "should not find .ctx when absent");
}

/// Pure walk-up helper — mirrors the logic in `resolve_context_path` without
/// touching `std::env::current_dir` or the real filesystem outside `start`.
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

// ---------------------------------------------------------------------------
// Ctx dir caching — cache hit/miss/invalidation
// ---------------------------------------------------------------------------

use super::ProjectContextModel;

#[test]
fn cache_is_empty_on_new_model() {
    let model = ProjectContextModel::new_bare();
    assert!(
        model.resolved_ctx_dir.is_none(),
        "resolved_ctx_dir should start as None"
    );
    assert!(
        model.cached_cwd.is_none(),
        "cached_cwd should start as None"
    );
}

#[test]
fn cache_miss_fires_walk_and_stores_result() {
    let (tmp, leaf) = make_ctx_dir_at_depth(2);
    let mut model = ProjectContextModel::new_bare();

    // Simulate a "load from cwd" by calling resolve_and_cache with the leaf dir.
    let resolved = model.resolve_ctx_dir_for(&leaf);

    let expected = tmp.path().join(".ctx").join("context.json");
    assert_eq!(
        resolved.as_deref(),
        Some(expected.as_path()),
        "walk-up should find context.json"
    );
    assert_eq!(
        model.resolved_ctx_dir.as_deref(),
        Some(expected.as_path()),
        "resolved path should be cached"
    );
    assert_eq!(
        model.cached_cwd.as_deref(),
        Some(leaf.as_path()),
        "cwd used for resolution should be cached"
    );
}

#[test]
fn cache_hit_returns_stored_path_without_rewalk() {
    let (tmp, leaf) = make_ctx_dir_at_depth(1);
    let mut model = ProjectContextModel::new_bare();

    // Prime the cache.
    let first = model.resolve_ctx_dir_for(&leaf);

    // Now remove the actual file — a real walk would return None.
    std::fs::remove_file(tmp.path().join(".ctx").join("context.json"))
        .expect("remove context.json");

    // Second call with same cwd — should return cached value without rewalk.
    let second = model.resolve_ctx_dir_for(&leaf);
    assert_eq!(
        first, second,
        "cache hit must return stored result even after file removal"
    );
}

#[test]
fn cwd_change_invalidates_cache() {
    let (tmp1, leaf1) = make_ctx_dir_at_depth(1);
    let (tmp2, leaf2) = make_ctx_dir_at_depth(0);
    let mut model = ProjectContextModel::new_bare();

    // Prime cache with leaf1.
    model.resolve_ctx_dir_for(&leaf1);
    assert_eq!(
        model.cached_cwd.as_deref(),
        Some(leaf1.as_path()),
        "cached_cwd should point to leaf1"
    );

    // Different cwd — cache should invalidate and re-walk.
    let resolved2 = model.resolve_ctx_dir_for(&leaf2);
    let expected2 = tmp2.path().join(".ctx").join("context.json");
    assert_eq!(
        resolved2.as_deref(),
        Some(expected2.as_path()),
        "should resolve new cwd"
    );
    assert_eq!(
        model.cached_cwd.as_deref(),
        Some(leaf2.as_path()),
        "cached_cwd should update to leaf2"
    );

    drop(tmp1); // suppress unused-variable warning
}
