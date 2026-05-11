use serde::Deserialize;
use std::path::PathBuf;
use warpui::{Entity, ModelContext};

use crate::claude_projects;

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

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.spawn(
            async move {
                // Generate context inline (blocking git/doob calls).
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
                let generated =
                    tokio::task::spawn_blocking(move || warpx::context::generate_context(&cwd))
                        .await
                        .unwrap_or_default();

                let mut state = ContextState {
                    git: GitSection {
                        branch: generated.git.branch,
                        dirty_count: generated.git.dirty_count,
                        recent_commits: generated.git.recent_commits,
                    },
                    ai: AiSection {
                        claude_md_files: generated.ai.claude_md_files,
                        mcp_servers: generated.ai.mcp_servers,
                    },
                    handoff: HandoffSection {
                        items: generated
                            .handoff
                            .items
                            .into_iter()
                            .map(|i| HandoffItem {
                                summary: i.summary,
                                priority: i.priority,
                                status: i.status,
                            })
                            .collect(),
                    },
                    todos: TodoSection {
                        pending: generated
                            .todos
                            .pending
                            .into_iter()
                            .map(|t| TodoItem {
                                name: t.name,
                                priority: t.priority,
                                status: t.status,
                            })
                            .collect(),
                    },
                    projects: vec![],
                };

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
                let mut raw_items: Vec<warpx::handoff::HandoffItem> = Vec::new();
                warpx::handoff::scan_ctx_dir(&path.join(".ctx"), &mut raw_items);
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
