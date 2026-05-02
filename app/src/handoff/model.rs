use rusqlite::Connection;
use std::path::PathBuf;
use warpui::{Entity, ModelContext};

/// A handoff item from `~/.ctx/handoff.db`.
#[derive(Clone, Debug, PartialEq)]
pub struct HandoffItem {
    pub project: String,
    pub id: String,
    pub name: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub completed: Option<String>,
    pub updated: String,
    pub issue: Option<i64>,
}

/// An extra annotation attached to a handoff item.
#[derive(Clone, Debug, PartialEq)]
pub struct HandoffExtra {
    pub extra_id: String,
    pub date: String,
    pub kind: String,
    pub field: Option<String>,
    pub value: Option<String>,
    pub note: Option<String>,
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

pub struct HandoffModel {
    pub project: String,
    pub items: Vec<HandoffItem>,
    pub extras: std::collections::HashMap<String, Vec<HandoffExtra>>,
    pub load_state: LoadState,
}

impl HandoffModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            project: String::new(),
            items: Vec::new(),
            extras: std::collections::HashMap::new(),
            load_state: LoadState::NotLoaded,
        }
    }

    fn db_path() -> PathBuf {
        dirs::home_dir()
            .expect("unable to determine home directory for handoff database path")
            .join(".ctx")
            .join("handoff.db")
    }

    /// Derive project name from the git root directory name.
    ///
    /// Uses the directory name of the nearest git root, which matches how
    /// `hj`/`handoff-detect` keys items in the handoff database.
    pub fn detect_project(dir: &std::path::Path) -> Option<String> {
        // Walk up to find .git, use that directory's name as the project key.
        let mut current = dir;
        loop {
            if current.join(".git").exists() {
                return current
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string());
            }
            match current.parent() {
                Some(parent) if parent != current => current = parent,
                _ => break,
            }
        }
        // Fallback: use the leaf directory name.
        dir.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    /// Load items from the DB for the given cwd. Runs synchronously (called from spawn).
    pub fn load_for_directory(dir: PathBuf) -> Result<LoadResult, String> {
        let project = Self::detect_project(&dir).unwrap_or_else(|| "unknown".to_string());
        log::info!("[handoff] load_for_directory dir={dir:?} project={project:?}");

        let db_path = Self::db_path();
        if !db_path.exists() {
            return Ok(LoadResult {
                project,
                items: Vec::new(),
                extras: std::collections::HashMap::new(),
            });
        }

        let conn = Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("pragma: {e}"))?;

        let mut stmt = conn
            .prepare(
                "SELECT project, id, name, priority, status, completed, updated, issue \
                 FROM items WHERE project = ? ORDER BY \
                 CASE status WHEN 'open' THEN 0 WHEN 'blocked' THEN 1 ELSE 2 END, \
                 CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 \
                 WHEN 'P2' THEN 2 WHEN 'P3' THEN 3 ELSE 99 END",
            )
            .map_err(|e| format!("prepare items: {e}"))?;

        let items = stmt
            .query_map([&project], |row| {
                Ok(HandoffItem {
                    project: row.get(0)?,
                    id: row.get(1)?,
                    name: row.get(2)?,
                    priority: row.get(3)?,
                    status: row.get(4)?,
                    completed: row.get(5)?,
                    updated: row.get(6)?,
                    issue: row.get(7)?,
                })
            })
            .map_err(|e| format!("query items: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect items: {e}"))?;

        let mut extras: std::collections::HashMap<String, Vec<HandoffExtra>> =
            std::collections::HashMap::new();
        if !items.is_empty() {
            let mut estmt = conn
                .prepare(
                    "SELECT item_id, extra_id, date, type, field, value, note \
                     FROM extras WHERE project = ? ORDER BY date DESC",
                )
                .map_err(|e| format!("prepare extras: {e}"))?;

            let extra_rows = estmt
                .query_map([&project], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        HandoffExtra {
                            extra_id: row.get(1)?,
                            date: row.get(2)?,
                            kind: row.get(3)?,
                            field: row.get(4)?,
                            value: row.get(5)?,
                            note: row.get(6)?,
                        },
                    ))
                })
                .map_err(|e| format!("query extras: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect extras: {e}"))?;

            for (item_id, extra) in extra_rows {
                extras.entry(item_id).or_default().push(extra);
            }
        }

        log::info!(
            "[handoff] loaded {} items for project {project:?}",
            items.len()
        );
        Ok(LoadResult {
            project,
            items,
            extras,
        })
    }

    /// Update item status in the DB.
    pub fn set_item_status(item_id: &str, project: &str, status: &str) -> Result<(), String> {
        let db_path = Self::db_path();
        let conn = Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE items SET status = ?, updated = ? WHERE project = ? AND id = ?",
            rusqlite::params![status, now, project, item_id],
        )
        .map_err(|e| format!("update status: {e}"))?;
        Ok(())
    }

    /// Mark item as completed in the DB.
    pub fn complete_item(item_id: &str, project: &str) -> Result<(), String> {
        let db_path = Self::db_path();
        let conn = Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE items SET status = 'done', completed = ?, updated = ? \
             WHERE project = ? AND id = ?",
            rusqlite::params![now, now, project, item_id],
        )
        .map_err(|e| format!("complete item: {e}"))?;
        Ok(())
    }

    /// Add a note extra to an item.
    pub fn add_note(item_id: &str, project: &str, note: &str) -> Result<(), String> {
        let db_path = Self::db_path();
        let conn = Connection::open(&db_path).map_err(|e| format!("open db: {e}"))?;
        let now = chrono::Utc::now().to_rfc3339();
        let extra_id = format!("{}-note-{}", item_id, &now);
        conn.execute(
            "INSERT INTO extras (project, item_id, extra_id, date, type, note, committed) \
             VALUES (?, ?, ?, ?, 'note', ?, 1)",
            rusqlite::params![project, item_id, extra_id, now, note],
        )
        .map_err(|e| format!("insert note: {e}"))?;
        Ok(())
    }

    pub fn load(&mut self, cwd: PathBuf, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();

        ctx.spawn(
            async move { HandoffModel::load_for_directory(cwd) },
            |me, result, ctx| match result {
                Ok(loaded) => {
                    me.project = loaded.project;
                    me.items = loaded.items;
                    me.extras = loaded.extras;
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
    pub project: String,
    pub items: Vec<HandoffItem>,
    pub extras: std::collections::HashMap<String, Vec<HandoffExtra>>,
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
    use super::HandoffModel;
    use std::path::PathBuf;

    fn make_git_root(base: &tempfile::TempDir, name: &str) -> PathBuf {
        let root = base.path().join(name);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        root
    }

    #[test]
    fn detect_project_returns_git_root_dir_name() {
        let tmp = tempfile::tempdir().unwrap();
        let root = make_git_root(&tmp, "warpx");
        assert_eq!(
            HandoffModel::detect_project(&root),
            Some("warpx".to_string())
        );
    }

    #[test]
    fn detect_project_walks_up_from_subdir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = make_git_root(&tmp, "myproject");
        let subdir = root.join("app").join("src");
        std::fs::create_dir_all(&subdir).unwrap();
        assert_eq!(
            HandoffModel::detect_project(&subdir),
            Some("myproject".to_string())
        );
    }

    #[test]
    fn detect_project_fallback_when_no_git() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("somedir");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(
            HandoffModel::detect_project(&dir),
            Some("somedir".to_string())
        );
    }

    /// Regression: must NOT return the Cargo.toml package name.
    /// The warpx repo has `name = "warp"` in Cargo.toml but hj keys
    /// items by directory name "warpx". This test ensures we use the
    /// directory name, never file content.
    #[test]
    fn detect_project_ignores_cargo_toml_name() {
        let tmp = tempfile::tempdir().unwrap();
        // Directory is named "warpx", Cargo.toml says name = "warp"
        let root = make_git_root(&tmp, "warpx");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"warp\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            HandoffModel::detect_project(&root),
            Some("warpx".to_string()),
            "project key must be dir name, not Cargo.toml package name"
        );
    }
}
