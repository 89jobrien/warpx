use serde::Deserialize;
use std::path::PathBuf;
use warpui::{Entity, ModelContext};

use crate::claude_projects;
use crate::handoff::model as handoff_model;

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

/// Handoff items aggregated from a single project's `.ctx/` directory.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectHandoff {
    pub project_name: String,
    pub items: Vec<HandoffItem>,
}

/// Context state read from `~/.warp/context.json` (git/ai/todos)
/// plus live cross-project handoff data from `~/.claude/projects/`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ContextState {
    #[serde(default)]
    pub git: GitSection,
    #[serde(default)]
    pub ai: AiSection,
    #[serde(default)]
    pub handoff: HandoffSection,
    #[serde(default)]
    pub todos: TodoSection,
    /// Live cross-project handoff items scanned from `~/.claude/projects/`.
    /// Not deserialized from JSON — populated separately after load.
    #[serde(skip)]
    pub projects: Vec<ProjectHandoff>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loaded,
    Error(String),
}

pub struct CtxWindowModel {
    pub state: ContextState,
    pub load_state: LoadState,
}

#[derive(Clone, Debug)]
pub enum CtxWindowModelEvent {
    Updated,
    Error(()),
}

impl CtxWindowModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            state: ContextState::default(),
            load_state: LoadState::NotLoaded,
        }
    }

    fn state_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".warp").join("context.json")
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        let path = Self::state_path();
        ctx.spawn(
            async move {
                // Load cwd-specific context from context.json (best-effort).
                let mut state = tokio::fs::read_to_string(&path)
                    .await
                    .ok()
                    .and_then(|c| serde_json::from_str::<ContextState>(&c).ok())
                    .unwrap_or_default();

                // Aggregate live handoff data across all projects in ~/.claude/projects/.
                state.projects = Self::scan_all_projects();
                Ok::<ContextState, String>(state)
            },
            |me, result, ctx| match result {
                Ok(state) => {
                    me.state = state;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(CtxWindowModelEvent::Updated);
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e);
                    ctx.emit(CtxWindowModelEvent::Error(()));
                }
            },
        );
    }

    /// Scan all project directories listed in `~/.claude/projects/` and
    /// aggregate their `.ctx/HANDOFF.*.yaml` items into `ProjectHandoff` entries.
    /// Returns entries in the same order as `~/.claude/projects/`.
    pub(crate) fn scan_all_projects() -> Vec<ProjectHandoff> {
        claude_projects::read_project_entries()
            .into_iter()
            .filter_map(|entry| {
                let path = &entry.decoded_path;
                if !path.is_dir() {
                    return None;
                }
                let mut raw_items: Vec<handoff_model::HandoffItem> = Vec::new();
                handoff_model::HandoffModel::scan_ctx_dir_pub(&path.join(".ctx"), &mut raw_items);
                if raw_items.is_empty() {
                    return None;
                }
                let items = raw_items
                    .into_iter()
                    .map(|i| HandoffItem {
                        summary: i.title.unwrap_or_else(|| i.id.clone()),
                        priority: i.priority.unwrap_or_default(),
                        status: i.status.unwrap_or_default(),
                    })
                    .collect();
                Some(ProjectHandoff {
                    project_name: entry.project_name,
                    items,
                })
            })
            .collect()
    }
}

impl Entity for CtxWindowModel {
    type Event = CtxWindowModelEvent;
}
