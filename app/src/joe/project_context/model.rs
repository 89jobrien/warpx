use std::path::PathBuf;
use warpui::{Entity, ModelContext};
#[allow(unused_imports)]
pub use warpx::context::{AiSection, GitSection, HandoffItem, TodoItem};
pub use warpx::context::{HandoffSection, ProjectContextState, TodoSection};

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
        }
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let path = warpx::context::resolve_context_path(&cwd);
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
