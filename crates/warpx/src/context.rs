//! Project context data layer — types, generation, and path resolution.
//!
//! [`generate_context`] produces a [`ProjectContextState`] by running git
//! commands, scanning CLAUDE.md files, reading MCP config, parsing HANDOFF
//! YAML, and calling the `doob` CLI.
//!
//! WarpUI model/panel wrappers live in `app/src/context_window/`.

use std::path::{Path, PathBuf};

use command::blocking::Command;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct GitSection {
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub dirty_count: usize,
    #[serde(default)]
    pub recent_commits: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AiSection {
    #[serde(default)]
    pub claude_md_files: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct HandoffItem {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct HandoffSection {
    #[serde(default)]
    pub items: Vec<HandoffItem>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TodoItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TodoSection {
    #[serde(default)]
    pub pending: Vec<TodoItem>,
}

/// Context state for a project directory.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
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

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

/// Build a fresh [`ProjectContextState`] by inspecting `cwd`.
///
/// This is a blocking call — it shells out to `git` and `doob`.
pub fn generate_context(cwd: &Path) -> ProjectContextState {
    ProjectContextState {
        git: git_section(cwd),
        ai: ai_section(cwd),
        handoff: handoff_section(cwd),
        todos: todo_section(cwd),
    }
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

fn run_git(cwd: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn git_section(cwd: &Path) -> GitSection {
    let branch = run_git(cwd, &["branch", "--show-current"]).unwrap_or_else(|| "unknown".into());
    let status = run_git(cwd, &["status", "--porcelain"]).unwrap_or_default();
    let dirty_count = status.lines().filter(|l| !l.is_empty()).count();
    let log = run_git(cwd, &["log", "--oneline", "-5"]).unwrap_or_default();
    let recent_commits: Vec<String> = log.lines().map(|l| l.to_string()).collect();
    GitSection {
        branch,
        dirty_count,
        recent_commits,
    }
}

fn ai_section(cwd: &Path) -> AiSection {
    let home = home_dir();
    let mut claude_files = Vec::new();

    let global = home.join(".claude/CLAUDE.md");
    if global.exists() {
        claude_files.push(global.display().to_string());
    }
    let mut dir: Option<&Path> = Some(cwd);
    while let Some(d) = dir {
        let candidate = d.join("CLAUDE.md");
        if candidate.exists() {
            claude_files.push(candidate.display().to_string());
        }
        dir = d.parent();
    }
    claude_files.dedup();

    let mcp_servers = read_mcp_server_names(&home.join(".warp/.mcp.json"));
    AiSection {
        claude_md_files: claude_files,
        mcp_servers,
    }
}

fn read_mcp_server_names(path: &Path) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) else {
        return vec![];
    };
    val.get("mcpServers")
        .or_else(|| val.get("servers"))
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

fn handoff_section(cwd: &Path) -> HandoffSection {
    let mut items = Vec::new();
    let mut all_paths: Vec<PathBuf> = Vec::new();

    let root_handoff = cwd.join("HANDOFF.yaml");
    if root_handoff.exists() {
        all_paths.push(root_handoff);
    }

    let ctx_dir = cwd.join(".ctx");
    if ctx_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&ctx_dir)
    {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("HANDOFF.") && name.ends_with(".yaml") {
                all_paths.push(entry.path());
            }
        }
    }

    for path in all_paths {
        if let Ok(content) = std::fs::read_to_string(&path) {
            parse_handoff_yaml(&content, &mut items);
        }
    }
    HandoffSection { items }
}

fn parse_handoff_yaml(content: &str, items: &mut Vec<HandoffItem>) {
    let mut in_items = false;
    let mut title = String::new();
    let mut priority = String::new();
    let mut status = String::new();

    for line in content.lines() {
        if line == "items:" {
            in_items = true;
            continue;
        }
        if in_items && !line.starts_with(' ') && !line.starts_with('-') && !line.is_empty() {
            break;
        }
        if !in_items {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with("- ") {
            if !title.is_empty() {
                items.push(HandoffItem {
                    summary: std::mem::take(&mut title),
                    priority: std::mem::take(&mut priority),
                    status: std::mem::take(&mut status),
                });
            }
            let after = trimmed.strip_prefix("- ").unwrap_or("");
            if let Some(val) = after.strip_prefix("title:") {
                title = val.trim().trim_matches('\'').trim_matches('"').to_string();
            }
        } else if let Some(val) = trimmed.strip_prefix("title:") {
            title = val.trim().trim_matches('\'').trim_matches('"').to_string();
        } else if let Some(val) = trimmed.strip_prefix("priority:") {
            priority = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = trimmed.strip_prefix("status:") {
            status = val.trim().trim_matches('"').to_string();
        }
    }
    if !title.is_empty() {
        items.push(HandoffItem {
            summary: title,
            priority,
            status,
        });
    }
}

fn todo_section(cwd: &Path) -> TodoSection {
    let output = Command::new("doob")
        .args(["todo", "list", "--status", "pending", "--format", "json"])
        .current_dir(cwd)
        .output();

    let pending = match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            parse_doob_json(&text, cwd)
        }
        _ => vec![],
    };
    TodoSection { pending }
}

fn parse_doob_json(text: &str, cwd: &Path) -> Vec<TodoItem> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![];
    };
    let Some(arr) = val.as_array() else {
        return vec![];
    };
    let cwd_str = cwd.display().to_string();
    arr.iter()
        .filter(|item| {
            item.get("project")
                .and_then(|p| p.as_str())
                .map(|p| cwd_str.contains(p))
                .unwrap_or(true)
        })
        .take(15)
        .map(|item| TodoItem {
            name: item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            priority: item
                .get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            status: item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending")
                .to_string(),
        })
        .collect()
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
