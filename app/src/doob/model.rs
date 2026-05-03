use warpui::{Entity, ModelContext};

/// A single task item returned by `doob todo list --json`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct DoobItem {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub description: Option<String>,
    pub project: Option<String>,
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

pub struct DoobModel {
    pub items: Vec<DoobItem>,
    pub load_state: LoadState,
}

pub struct LoadResult {
    pub items: Vec<DoobItem>,
}

impl DoobModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            items: Vec::new(),
            load_state: LoadState::NotLoaded,
        }
    }

    /// Shell out to `doob todo list --json` and parse the result.
    /// Runs synchronously; call from `ctx.spawn`.
    pub fn fetch() -> Result<LoadResult, String> {
        let output = command::blocking::Command::new("doob")
            .args(["todo", "list", "--json"])
            .output()
            .map_err(|e| format!("failed to run doob: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("doob exited with error: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let items: Vec<DoobItem> = serde_json::from_str(&stdout)
            .map_err(|e| format!("failed to parse doob output: {e}"))?;

        log::info!("[doob] loaded {} items", items.len());
        Ok(LoadResult { items })
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        self.load_state = LoadState::Loading;
        ctx.notify();

        ctx.spawn(
            async move { DoobModel::fetch() },
            |me, result, ctx| match result {
                Ok(loaded) => {
                    me.items = loaded.items;
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
