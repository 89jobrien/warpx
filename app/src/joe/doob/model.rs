use warpui::{Entity, ModelContext};
pub use warpx::doob::DoobItem;

/// Load state for the panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded,
    Error(String),
}

pub struct DoobModel {
    pub items: Vec<DoobItem>,
    pub load_state: LoadState,
}

impl DoobModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            items: Vec::new(),
            load_state: LoadState::NotLoaded,
        }
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();

        ctx.spawn(
            async move { warpx::doob::fetch_items() },
            |me, result, ctx| match result {
                Ok(items) => {
                    me.items = items;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(DoobModelEvent::Loaded);
                    ctx.notify();
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e.clone());
                    ctx.emit(DoobModelEvent::Error(e));
                    ctx.notify();
                }
            },
        );
    }
}

impl Entity for DoobModel {
    type Event = DoobModelEvent;
}

#[derive(Clone, Debug)]
pub enum DoobModelEvent {
    Loaded,
    #[allow(dead_code)]
    Error(String),
}
