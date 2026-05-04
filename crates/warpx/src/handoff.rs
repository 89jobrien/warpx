//! Handoff data layer — types, YAML scanning, directory walk-up.
//!
//! WarpUI model/panel wrappers live in `app/src/joe/handoff/`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A handoff item parsed from a `.ctx/HANDOFF.*.yaml` file.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct HandoffItem {
    pub id: String,
    pub title: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub completed: Option<String>,
    pub description: Option<String>,
    /// Source file this item was loaded from (not in YAML; set after parse).
    #[serde(skip)]
    pub source_file: String,
}

/// Minimal shape of a HANDOFF YAML file — only the fields we need.
#[derive(Deserialize)]
struct HandoffFile {
    #[serde(default)]
    items: Vec<HandoffItem>,
}

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

/// Collect items from a single `.ctx/` directory into `items`.
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
        for mut item in parsed.items {
            item.source_file = name.clone();
            items.push(item);
        }
    }
}

/// Recursively walk `dir`, collecting items from every `.ctx/` found.
/// Skips hidden directories (starting with `.`) other than `.ctx` itself.
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

pub struct LoadResult {
    pub cwd: PathBuf,
    pub items: Vec<HandoffItem>,
    /// Project names in the order they appear in `~/.claude/projects/`.
    pub project_order: Vec<String>,
}

/// Load items from HANDOFF YAML files found recursively under `dir`.
/// Runs synchronously; intended to be called from a spawn closure.
pub fn load_for_directory(dir: PathBuf) -> Result<LoadResult, String> {
    log::info!("[handoff] load_for_directory dir={dir:?}");

    let mut items: Vec<HandoffItem> = Vec::new();
    scan_recursive(&dir, &mut items, 0);

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
}
