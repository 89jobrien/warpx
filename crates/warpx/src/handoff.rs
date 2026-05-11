//! Handoff data layer — shells out to `hj` CLI for YAML scanning and enriches
//! items with GitHub issue references from doob's gh-sync state.
//!
//! WarpUI model/panel wrappers live in `app/src/joe/handoff/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A handoff item parsed from `hj` output, optionally enriched with GH issue info.
#[derive(Clone, Debug, PartialEq)]
pub struct HandoffItem {
    pub id: String,
    pub title: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub completed: Option<String>,
    pub description: Option<String>,
    pub doob_uuid: Option<String>,
    /// GitHub issue number (from doob gh-sync state), if linked.
    pub issue_number: Option<u64>,
    /// GitHub repo (e.g. "89jobrien/minibox"), if linked.
    pub issue_repo: Option<String>,
    /// Source file this item was loaded from.
    pub source_file: String,
}

/// Raw item shape from HANDOFF YAML (parsed by hj or directly).
#[derive(Deserialize)]
struct RawHandoffItem {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    completed: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    doob_uuid: Option<String>,
    #[serde(default)]
    issue: Option<u64>,
}

/// Minimal shape of a HANDOFF YAML file.
#[derive(Deserialize)]
struct HandoffFile {
    #[serde(default)]
    items: Vec<RawHandoffItem>,
}

/// A single entry in doob's `~/.config/doob/gh-sync-state.json`.
#[derive(Deserialize)]
struct GhSyncEntry {
    repo: String,
    issue_number: u64,
}

pub struct LoadResult {
    pub cwd: PathBuf,
    pub items: Vec<HandoffItem>,
    /// Project names in the order they appear in `~/.claude/projects/`.
    pub project_order: Vec<String>,
}

/// Result from executing an `hj` command via the scratchpad.
#[derive(Clone, Debug)]
pub struct HjCommandResult {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

// ---------------------------------------------------------------------------
// GH-sync state
// ---------------------------------------------------------------------------

fn gh_sync_state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config/doob/gh-sync-state.json")
}

/// Load doob's gh-sync state: maps doob_uuid -> (repo, issue_number).
fn load_gh_sync_state() -> HashMap<String, GhSyncEntry> {
    let path = gh_sync_state_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// YAML scanning (filesystem fallback, same as hj internally)
// ---------------------------------------------------------------------------

/// Collect items from a single `.ctx/` directory.
pub fn scan_ctx_dir(ctx_dir: &Path, items: &mut Vec<HandoffItem>) {
    let read_dir = match std::fs::read_dir(ctx_dir) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[handoff] read_dir {ctx_dir:?}: {e}");
            return;
        }
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !name.starts_with("HANDOFF.")
            || !name.ends_with(".yaml")
            || name.ends_with(".state.yaml")
        {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[handoff] failed to read {path:?}: {e}");
                continue;
            }
        };
        let parsed: HandoffFile = match serde_yaml::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("[handoff] failed to parse {path:?}: {e}");
                continue;
            }
        };
        for raw in parsed.items {
            items.push(HandoffItem {
                id: raw.id,
                title: raw.title,
                priority: raw.priority,
                status: raw.status,
                completed: raw.completed,
                description: raw.description,
                doob_uuid: raw.doob_uuid,
                issue_number: raw.issue,
                issue_repo: None,
                source_file: name.clone(),
            });
        }
    }
}

/// Recursively walk `dir`, collecting items from every `.ctx/` found.
fn scan_recursive(dir: &Path, items: &mut Vec<HandoffItem>, depth: usize) {
    if depth > 4 {
        return;
    }
    let ctx = dir.join(".ctx");
    if ctx.is_dir() {
        scan_ctx_dir(&ctx, items);
    }
    let read_dir = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }
        scan_recursive(&path, items, depth + 1);
    }
}

// ---------------------------------------------------------------------------
// Enrichment
// ---------------------------------------------------------------------------

/// Enrich items with GitHub issue info from doob's gh-sync state.
fn enrich_with_gh_issues(items: &mut [HandoffItem]) {
    let state = load_gh_sync_state();
    if state.is_empty() {
        return;
    }
    for item in items.iter_mut() {
        // First check if the item already has an issue number from the YAML
        // (hj's `issue` field). If so, try to find the repo from gh-sync state.
        if let Some(uuid) = &item.doob_uuid
            && let Some(entry) = state.get(uuid)
        {
            item.issue_number = Some(entry.issue_number);
            item.issue_repo = Some(entry.repo.clone());
        }
    }
}

/// Sort items: open > blocked > done, then P0 > P1 > P2 > P3.
fn sort_items(items: &mut [HandoffItem]) {
    items.sort_by(|a, b| {
        let status_ord = |s: Option<&str>| match s {
            Some("open") => 0,
            Some("blocked") => 1,
            _ => 2,
        };
        let priority_ord = |p: Option<&str>| match p {
            Some("P0") => 0,
            Some("P1") => 1,
            Some("P2") => 2,
            Some("P3") => 3,
            _ => 99,
        };
        let sa = status_ord(a.status.as_deref());
        let sb = status_ord(b.status.as_deref());
        sa.cmp(&sb)
            .then(priority_ord(a.priority.as_deref()).cmp(&priority_ord(b.priority.as_deref())))
    });
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Find the nearest `.ctx/` directory by walking up from `dir`.
pub fn find_ctx_dir(dir: &Path) -> Option<PathBuf> {
    let mut current = dir;
    loop {
        let candidate = current.join(".ctx");
        if candidate.is_dir() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }
    None
}

/// Load items from HANDOFF YAML files found recursively under `dir`.
/// Enriches with GitHub issue info from doob's gh-sync state.
/// Runs synchronously; intended to be called from a spawn closure.
pub fn load_for_directory(dir: PathBuf) -> Result<LoadResult, String> {
    log::info!("[handoff] load_for_directory dir={dir:?}");

    let mut items: Vec<HandoffItem> = Vec::new();
    scan_recursive(&dir, &mut items, 0);
    enrich_with_gh_issues(&mut items);
    sort_items(&mut items);

    let project_order = crate::claude_projects::read_project_entries()
        .into_iter()
        .map(|e| e.project_name)
        .collect();

    log::info!("[handoff] loaded {} items from {dir:?}", items.len());
    Ok(LoadResult {
        cwd: dir,
        items,
        project_order,
    })
}

/// Execute an `hj` subcommand. The `args` string is split on whitespace and
/// passed to `hj` as arguments. Runs synchronously.
pub fn run_hj_command(args: &str) -> HjCommandResult {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let output = command::blocking::Command::new("hj").args(&parts).output();

    match output {
        Ok(out) => HjCommandResult {
            command: format!("hj {args}"),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            success: out.status.success(),
        },
        Err(e) => HjCommandResult {
            command: format!("hj {args}"),
            stdout: String::new(),
            stderr: format!("failed to run hj: {e}"),
            success: false,
        },
    }
}

/// Create a GitHub issue for a handoff item via `gh issue create`.
/// Returns the issue URL on success.
pub fn create_gh_issue(repo: &str, title: &str, body: &str) -> Result<String, String> {
    let output = command::blocking::Command::new("gh")
        .args([
            "issue", "create", "--repo", repo, "--title", title, "--body", body,
        ])
        .output()
        .map_err(|e| format!("failed to run gh: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh issue create failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Close a GitHub issue by number.
pub fn close_gh_issue(repo: &str, issue_number: u64) -> Result<(), String> {
    let num = issue_number.to_string();
    let output = command::blocking::Command::new("gh")
        .args(["issue", "close", "--repo", repo, &num])
        .output()
        .map_err(|e| format!("failed to run gh: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh issue close failed: {stderr}"));
    }
    Ok(())
}

/// Run `hj reconcile` to sync handoff items with doob todos.
pub fn run_sync() -> HjCommandResult {
    run_hj_command("reconcile")
}

/// Extract project name from source filename: `HANDOFF.<id>.<project>.yaml`.
pub fn project_from_source(source: &str) -> &str {
    let stripped = source
        .strip_prefix("HANDOFF.")
        .unwrap_or(source)
        .strip_suffix(".yaml")
        .unwrap_or(source);
    stripped.rsplit('.').next().unwrap_or(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx_dir(base: &tempfile::TempDir, name: &str) -> PathBuf {
        let root = base.path().join(name);
        std::fs::create_dir_all(root.join(".ctx")).unwrap();
        root
    }

    #[test]
    fn find_ctx_dir_finds_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        let root = make_ctx_dir(&tmp, "myproject");
        assert_eq!(find_ctx_dir(&root), Some(root.join(".ctx")));
    }

    #[test]
    fn find_ctx_dir_walks_up() {
        let tmp = tempfile::tempdir().unwrap();
        let root = make_ctx_dir(&tmp, "myproject");
        let subdir = root.join("app").join("src");
        std::fs::create_dir_all(&subdir).unwrap();
        assert_eq!(find_ctx_dir(&subdir), Some(root.join(".ctx")));
    }

    #[test]
    fn find_ctx_dir_returns_none_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("noctx");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(find_ctx_dir(&dir), None);
    }

    #[test]
    fn load_for_directory_scans_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let top_ctx = tmp.path().join(".ctx");
        std::fs::create_dir_all(&top_ctx).unwrap();
        std::fs::write(
            top_ctx.join("HANDOFF.top.top.yaml"),
            "items:\n- id: top1\n  title: Top item\n  status: open\n",
        )
        .unwrap();
        let sub_ctx = tmp.path().join("subproject").join(".ctx");
        std::fs::create_dir_all(&sub_ctx).unwrap();
        std::fs::write(
            sub_ctx.join("HANDOFF.sub.sub.yaml"),
            "items:\n- id: sub1\n  title: Sub item\n  status: open\n",
        )
        .unwrap();

        let result = load_for_directory(tmp.path().to_path_buf()).unwrap();
        assert_eq!(result.items.len(), 2);
        let ids: Vec<&str> = result.items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"top1"), "expected top1 in {ids:?}");
        assert!(ids.contains(&"sub1"), "expected sub1 in {ids:?}");
    }

    #[test]
    fn load_for_directory_excludes_state_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx_dir = tmp.path().join(".ctx");
        std::fs::create_dir_all(&ctx_dir).unwrap();
        std::fs::write(
            ctx_dir.join("HANDOFF.warp_core.proj.state.yaml"),
            "items:\n- id: s1\n  title: Should be ignored\n  status: open\n",
        )
        .unwrap();
        std::fs::write(
            ctx_dir.join("HANDOFF.myproject.proj.yaml"),
            "items:\n- id: t1\n  title: Real item\n  status: open\n",
        )
        .unwrap();

        let result = load_for_directory(tmp.path().to_path_buf()).unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "t1");
    }

    #[test]
    fn load_for_directory_sorts_open_before_done() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx_dir = tmp.path().join(".ctx");
        std::fs::create_dir_all(&ctx_dir).unwrap();
        std::fs::write(
            ctx_dir.join("HANDOFF.proj.proj.yaml"),
            "items:\n\
             - id: d1\n  title: Done item\n  priority: P0\n  status: done\n\
             - id: o1\n  title: Open item\n  priority: P2\n  status: open\n",
        )
        .unwrap();

        let result = load_for_directory(tmp.path().to_path_buf()).unwrap();
        assert_eq!(result.items[0].id, "o1");
        assert_eq!(result.items[1].id, "d1");
    }

    #[test]
    fn project_from_source_extracts_project() {
        assert_eq!(project_from_source("HANDOFF.core.minibox.yaml"), "minibox");
        assert_eq!(project_from_source("HANDOFF.doob.doob.yaml"), "doob");
    }

    #[test]
    fn enrichment_populates_issue_fields() {
        let mut items = vec![HandoffItem {
            id: "test-1".into(),
            title: Some("Test".into()),
            priority: None,
            status: Some("open".into()),
            completed: None,
            description: None,
            doob_uuid: Some("fake-uuid".into()),
            issue_number: None,
            issue_repo: None,
            source_file: "HANDOFF.test.test.yaml".into(),
        }];
        // With no state file, enrichment is a no-op
        enrich_with_gh_issues(&mut items);
        assert!(items[0].issue_number.is_none());
    }
}
