use std::path::PathBuf;
use warpui::{Entity, ModelContext};
pub use warpx::handoff::{load_for_directory, HandoffItem};

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
    pub cwd: PathBuf,
    pub items: Vec<HandoffItem>,
    pub load_state: LoadState,
    pub project_order: Vec<String>,
}

impl HandoffModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            cwd: PathBuf::new(),
            items: Vec::new(),
            load_state: LoadState::NotLoaded,
            project_order: Vec::new(),
        }
    }

    pub fn load(&mut self, cwd: PathBuf, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();

        ctx.spawn(
            async move { load_for_directory(cwd) },
            |me, result, ctx| match result {
                Ok(loaded) => {
                    me.cwd = loaded.cwd;
                    me.items = loaded.items;
                    me.project_order = loaded.project_order;
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

impl Entity for HandoffModel {
    type Event = HandoffModelEvent;
}

#[derive(Clone, Debug)]
pub enum HandoffModelEvent {
    Loaded,
    #[allow(dead_code)]
    Error(String),
}
