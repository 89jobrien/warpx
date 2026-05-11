use std::path::PathBuf;
use warpui::{Entity, ModelContext};

/// A single checkpoint row from `handup.db`.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct HandupCheckpoint {
    pub id: i64,
    pub project: String,
    pub cwd: String,
    pub generated: String,
    pub recommendation: Option<String>,
    pub json_path: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

pub struct HandupModel {
    pub checkpoints: Vec<HandupCheckpoint>,
    pub load_state: LoadState,
}

impl HandupModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            checkpoints: Vec::new(),
            load_state: LoadState::NotLoaded,
        }
    }

    fn db_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".ctx").join("handoffs").join("handup.db"))
    }

    fn query_checkpoints(path: &std::path::Path) -> Result<Vec<HandupCheckpoint>, String> {
        let conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("{e}"))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, project, cwd, generated, recommendation, json_path, created_at \
                 FROM checkpoints ORDER BY created_at DESC",
            )
            .map_err(|e| format!("{e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(HandupCheckpoint {
                    id: row.get(0)?,
                    project: row.get(1)?,
                    cwd: row.get(2)?,
                    generated: row.get(3)?,
                    recommendation: row.get(4)?,
                    json_path: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("{e}"))?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("{e}"))?);
        }
        Ok(out)
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();

        ctx.spawn(
            async move {
                let path =
                    Self::db_path().ok_or_else(|| "cannot resolve home directory".to_string())?;
                if !path.exists() {
                    return Err(format!("database not found: {}", path.display()));
                }
                Self::query_checkpoints(&path)
            },
            |me, result, ctx| match result {
                Ok(checkpoints) => {
                    me.checkpoints = checkpoints;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(HandupModelEvent::Loaded);
                    ctx.notify();
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e.clone());
                    ctx.emit(HandupModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }
}

impl Entity for HandupModel {
    type Event = HandupModelEvent;
}

#[derive(Clone, Debug)]
pub enum HandupModelEvent {
    Loaded,
    #[allow(dead_code)]
    Error(String),
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
