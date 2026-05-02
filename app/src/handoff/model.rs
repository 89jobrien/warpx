use std::path::PathBuf;
use warpui::{Entity, ModelContext};

/// A handoff item parsed from a `.ctx/HANDOFF.*.yaml` file.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
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

/// Load state for the panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

/// Minimal shape of a HANDOFF YAML file — only the fields we need.
#[derive(serde::Deserialize)]
struct HandoffFile {
    #[serde(default)]
    items: Vec<HandoffItem>,
}

pub struct HandoffModel {
    pub cwd: PathBuf,
    pub items: Vec<HandoffItem>,
    pub load_state: LoadState,
}

impl HandoffModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            cwd: PathBuf::new(),
            items: Vec::new(),
            load_state: LoadState::NotLoaded,
        }
    }

    /// Find the nearest `.ctx/` directory by walking up from `dir`.
    pub fn find_ctx_dir(dir: &std::path::Path) -> Option<PathBuf> {
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

    /// Load items from HANDOFF YAML files found in `.ctx/` near `dir`.
    /// Runs synchronously (called from spawn).
    pub fn load_for_directory(dir: PathBuf) -> Result<LoadResult, String> {
        log::info!("[handoff] load_for_directory dir={dir:?}");

        let ctx_dir = match Self::find_ctx_dir(&dir) {
            Some(d) => d,
            None => {
                log::info!("[handoff] no .ctx/ dir found from {dir:?}");
                return Ok(LoadResult {
                    cwd: dir,
                    items: Vec::new(),
                });
            }
        };

        log::info!("[handoff] scanning {ctx_dir:?}");

        let mut items: Vec<HandoffItem> = Vec::new();

        let read_dir =
            std::fs::read_dir(&ctx_dir).map_err(|e| format!("read_dir {ctx_dir:?}: {e}"))?;

        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            // Match HANDOFF.*.yaml but exclude *.state.yaml and *.state.json
            if !name.starts_with("HANDOFF.") {
                continue;
            }
            if !name.ends_with(".yaml") {
                continue;
            }
            if name.ends_with(".state.yaml") {
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

            let source = name.clone();
            for mut item in parsed.items {
                item.source_file = source.clone();
                items.push(item);
            }
        }

        // Sort: open first, then blocked, then done; within each group by priority.
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

        log::info!("[handoff] loaded {} items from {ctx_dir:?}", items.len());
        Ok(LoadResult { cwd: dir, items })
    }

    pub fn load(&mut self, cwd: PathBuf, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();

        ctx.spawn(
            async move { HandoffModel::load_for_directory(cwd) },
            |me, result, ctx| match result {
                Ok(loaded) => {
                    me.cwd = loaded.cwd;
                    me.items = loaded.items;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(HandoffModelEvent::Loaded);
                    ctx.notify();
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e.clone());
                    ctx.emit(HandoffModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }
}

pub struct LoadResult {
    pub cwd: PathBuf,
    pub items: Vec<HandoffItem>,
}

impl Entity for HandoffModel {
    type Event = HandoffModelEvent;
}

#[derive(Clone, Debug)]
pub enum HandoffModelEvent {
    Loaded,
    #[allow(dead_code)]
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::{HandoffItem, HandoffModel};
    use std::path::PathBuf;

    fn make_ctx_dir(base: &tempfile::TempDir, name: &str) -> PathBuf {
        let root = base.path().join(name);
        std::fs::create_dir_all(root.join(".ctx")).unwrap();
        root
    }

    #[test]
    fn find_ctx_dir_finds_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        let root = make_ctx_dir(&tmp, "myproject");
        assert_eq!(HandoffModel::find_ctx_dir(&root), Some(root.join(".ctx")));
    }

    #[test]
    fn find_ctx_dir_walks_up() {
        let tmp = tempfile::tempdir().unwrap();
        let root = make_ctx_dir(&tmp, "myproject");
        let subdir = root.join("app").join("src");
        std::fs::create_dir_all(&subdir).unwrap();
        assert_eq!(HandoffModel::find_ctx_dir(&subdir), Some(root.join(".ctx")));
    }

    #[test]
    fn find_ctx_dir_returns_none_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("noctx");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(HandoffModel::find_ctx_dir(&dir), None);
    }

    #[test]
    fn load_for_directory_reads_handoff_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx_dir = tmp.path().join(".ctx");
        std::fs::create_dir_all(&ctx_dir).unwrap();

        std::fs::write(
            ctx_dir.join("HANDOFF.myproject.proj.yaml"),
            "items:\n- id: t1\n  title: Fix bug\n  priority: P1\n  status: open\n",
        )
        .unwrap();

        let result = HandoffModel::load_for_directory(tmp.path().to_path_buf()).unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "t1");
        assert_eq!(result.items[0].title.as_deref(), Some("Fix bug"));
        assert_eq!(result.items[0].priority.as_deref(), Some("P1"));
    }

    #[test]
    fn load_for_directory_excludes_state_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx_dir = tmp.path().join(".ctx");
        std::fs::create_dir_all(&ctx_dir).unwrap();

        // state files should be skipped
        std::fs::write(
            ctx_dir.join("HANDOFF.warp_core.proj.state.yaml"),
            "items:\n- id: s1\n  title: Should be ignored\n  status: open\n",
        )
        .unwrap();
        // real handoff file should be included
        std::fs::write(
            ctx_dir.join("HANDOFF.myproject.proj.yaml"),
            "items:\n- id: t1\n  title: Real item\n  status: open\n",
        )
        .unwrap();

        let result = HandoffModel::load_for_directory(tmp.path().to_path_buf()).unwrap();
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

        let result = HandoffModel::load_for_directory(tmp.path().to_path_buf()).unwrap();
        assert_eq!(result.items[0].id, "o1");
        assert_eq!(result.items[1].id, "d1");
    }

    #[test]
    fn handoff_item_derives_clone_debug_partialeq() {
        let item = HandoffItem {
            id: "x".into(),
            title: None,
            priority: None,
            status: None,
            completed: None,
            description: None,
            source_file: String::new(),
        };
        let _ = item.clone();
        let _ = format!("{item:?}");
        assert_eq!(item, item.clone());
    }
}
