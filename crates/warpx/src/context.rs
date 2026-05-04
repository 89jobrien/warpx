//! Project context data layer — types and path resolution.
//!
//! Reads `.ctx/context.json` walking up from CWD.
//! WarpUI model/panel wrappers live in `app/src/joe/project_context/`.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct GitSection {
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub dirty_count: usize,
    #[serde(default)]
    pub recent_commits: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct AiSection {
    #[serde(default)]
    pub claude_md_files: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct HandoffItem {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct HandoffSection {
    #[serde(default)]
    pub items: Vec<HandoffItem>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct TodoItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct TodoSection {
    #[serde(default)]
    pub pending: Vec<TodoItem>,
}

/// Context state read from the project-local `.ctx/context.json`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ProjectContextState {
    #[serde(default)]
    pub git: GitSection,
    #[serde(default)]
    pub ai: AiSection,
    #[serde(default)]
    pub handoff: HandoffSection,
    #[serde(default)]
    pub todos: TodoSection,
}

/// Walk up from `dir` looking for `.ctx/context.json`.
/// Falls back to `~/.warp-oss/context.json` if not found.
pub fn resolve_context_path(cwd: &std::path::Path) -> PathBuf {
    let mut current = cwd;
    loop {
        let candidate = current.join(".ctx").join("context.json");
        if candidate.is_file() {
            return candidate;
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".warp-oss").join("context.json")
}

/// Read and parse `.ctx/context.json` from the nearest ancestor of `cwd`.
pub fn load_context(cwd: &std::path::Path) -> Result<ProjectContextState, String> {
    let path = resolve_context_path(cwd);
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("No context found at {path:?} — run warp-context-gen"))?;
    serde_json::from_str(&text).map_err(|e| format!("failed to parse context.json: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    const FIXTURE: &str = r#"{
      "git": { "branch": "develop", "dirty_count": 2,
               "recent_commits": ["abc1234 feat: foo", "def5678 fix: bar"] },
      "ai": { "claude_md_files": ["/Users/joe/.claude/CLAUDE.md"],
              "mcp_servers": ["pieces", "personal-mcp"] },
      "handoff": { "items": [
        { "summary": "Implement foo", "priority": "P1", "status": "open" },
        { "summary": "Done thing",    "priority": "P2", "status": "done" }
      ]},
      "todos": { "pending": [
        { "name": "write tests", "priority": "P0", "status": "pending" }
      ]}
    }"#;

    fn make_ctx_dir_at_depth(depth: usize) -> (TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ctx_dir = tmp.path().join(".ctx");
        std::fs::create_dir_all(&ctx_dir).unwrap();
        std::fs::write(ctx_dir.join("context.json"), FIXTURE).unwrap();
        let mut leaf = tmp.path().to_path_buf();
        for i in 0..depth {
            leaf = leaf.join(format!("level{i}"));
            std::fs::create_dir_all(&leaf).unwrap();
        }
        (tmp, leaf)
    }

    #[test]
    fn deserialize_full_context_json() {
        let state: ProjectContextState = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(state.git.branch, "develop");
        assert_eq!(state.git.dirty_count, 2);
        assert_eq!(state.ai.mcp_servers, vec!["pieces", "personal-mcp"]);
        assert_eq!(state.handoff.items[0].priority, "P1");
        assert_eq!(state.todos.pending[0].name, "write tests");
    }

    #[test]
    fn deserialize_empty_json_uses_defaults() {
        let state: ProjectContextState = serde_json::from_str("{}").unwrap();
        assert_eq!(state.git, GitSection::default());
        assert_eq!(state.handoff, HandoffSection::default());
    }

    #[test]
    fn deserialize_unknown_fields_are_ignored() {
        let json = r#"{ "unknown_key": 42, "git": { "branch": "feat" } }"#;
        let state: ProjectContextState = serde_json::from_str(json).unwrap();
        assert_eq!(state.git.branch, "feat");
    }

    #[test]
    fn walk_up_finds_ctx_three_levels_deep() {
        let (tmp, leaf) = make_ctx_dir_at_depth(3);
        let found = resolve_context_path(&leaf);
        assert_eq!(found, tmp.path().join(".ctx").join("context.json"));
    }

    #[test]
    fn walk_up_falls_back_when_no_ctx_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let leaf = tmp.path().join("a").join("b");
        std::fs::create_dir_all(&leaf).unwrap();
        let found = resolve_context_path(&leaf);
        assert!(found.to_string_lossy().contains(".warp-oss"));
    }
}
