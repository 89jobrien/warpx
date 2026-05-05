use serde::Deserialize;
use std::path::PathBuf;
use warpui::{Entity, ModelContext};

// Re-use the same JSON shape as CtxWindowModel — same structs, local CWD-relative source.
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

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loaded,
    Error(String),
}

pub struct ProjectContextModel {
    pub state: ProjectContextState,
    pub load_state: LoadState,
    /// The `.ctx/context.json` path resolved for `cached_cwd`. `None` means
    /// no `.ctx/` was found in the walk-up from that cwd.
    pub resolved_ctx_dir: Option<PathBuf>,
    /// The cwd that was in effect when `resolved_ctx_dir` was last populated.
    /// When the cwd changes, this cache entry is invalidated.
    pub cached_cwd: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum ProjectContextModelEvent {
    Updated,
    Error(()),
}

impl ProjectContextModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            state: ProjectContextState::default(),
            load_state: LoadState::NotLoaded,
            resolved_ctx_dir: None,
            cached_cwd: None,
        }
    }

    /// Construct a bare model without a `ModelContext` — for unit tests only.
    #[cfg(test)]
    pub fn new_bare() -> Self {
        Self {
            state: ProjectContextState::default(),
            load_state: LoadState::NotLoaded,
            resolved_ctx_dir: None,
            cached_cwd: None,
        }
    }

    /// Walk up from `start` looking for `.ctx/context.json`.
    /// Returns `Some(path)` if found, `None` otherwise.
    fn walk_up(start: &std::path::Path) -> Option<PathBuf> {
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

    /// Return the cached `.ctx/context.json` path for `cwd`, re-walking only
    /// on a cache miss or when `cwd` differs from the previously cached cwd.
    pub fn resolve_ctx_dir_for(&mut self, cwd: &std::path::Path) -> Option<PathBuf> {
        // Cache hit: same cwd as last resolution.
        if self.cached_cwd.as_deref() == Some(cwd) {
            return self.resolved_ctx_dir.clone();
        }

        // Cache miss or cwd changed — re-walk and store.
        let resolved = Self::walk_up(cwd);
        self.resolved_ctx_dir = resolved.clone();
        self.cached_cwd = Some(cwd.to_path_buf());
        resolved
    }

    /// Walk up from `dir` looking for `.ctx/context.json`.
    /// Falls back to `~/.warp-oss/context.json` if not found.
    fn resolve_context_path(&mut self) -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        if let Some(path) = self.resolve_ctx_dir_for(&cwd) {
            return path;
        }

        // Fallback: ~/.warp-oss/context.json
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".warp-oss").join("context.json")
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        let path = self.resolve_context_path();
        ctx.spawn(
            async move {
                let state = tokio::fs::read_to_string(&path)
                    .await
                    .ok()
                    .and_then(|c| serde_json::from_str::<ProjectContextState>(&c).ok())
                    .ok_or_else(|| {
                        format!("No context found at {path:?} — run warp-context-gen")
                    })?;
                Ok::<ProjectContextState, String>(state)
            },
            |me, result, ctx| match result {
                Ok(state) => {
                    me.state = state;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(ProjectContextModelEvent::Updated);
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e);
                    ctx.emit(ProjectContextModelEvent::Error(()));
                }
            },
        );
    }
}

impl Entity for ProjectContextModel {
    type Event = ProjectContextModelEvent;
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
