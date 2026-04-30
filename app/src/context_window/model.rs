use serde::Deserialize;
use std::path::PathBuf;
use warpui::{Entity, ModelContext};

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

/// Context state read from `~/.warp/context.json`.
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
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("read {}: {e}", path.display()))?;
                serde_json::from_str::<ContextState>(&content).map_err(|e| format!("parse: {e}"))
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
}

impl Entity for CtxWindowModel {
    type Event = CtxWindowModelEvent;
}
