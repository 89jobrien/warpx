use serde::Deserialize;
use std::path::PathBuf;
use warpui::{Entity, ModelContext};

/// A single sentiment analysis entry.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct SentimentEntry {
    pub text: String,
    pub sentiment: String,
    pub severity: String,
    pub mood: String,
    pub confidence: f64,
    #[serde(default)]
    pub timestamp: u64,
}

/// State file written by looprs sessions at `~/.looprs/sentiment.json`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct SentimentState {
    pub status: String,
    #[serde(default)]
    pub current: Option<SentimentEntry>,
    #[serde(default)]
    pub history: Vec<SentimentEntry>,
}

/// Load state for the panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LoadState {
    #[default]
    NotLoaded,
    Loaded,
    Error(String),
}

pub struct SentimentModel {
    pub state: SentimentState,
    pub load_state: LoadState,
}

#[derive(Clone, Debug)]
pub enum SentimentModelEvent {
    Updated,
    Error(String),
}

impl SentimentModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            state: SentimentState::default(),
            load_state: LoadState::NotLoaded,
        }
    }

    fn state_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".looprs").join("sentiment.json")
    }

    pub fn load(&mut self, ctx: &mut ModelContext<Self>) {
        let path = Self::state_path();
        ctx.spawn(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("read {}: {e}", path.display()))?;
                serde_json::from_str::<SentimentState>(&content).map_err(|e| format!("parse: {e}"))
            },
            |me, result, ctx| match result {
                Ok(state) => {
                    me.state = state;
                    me.load_state = LoadState::Loaded;
                    ctx.emit(SentimentModelEvent::Updated);
                }
                Err(e) => {
                    me.load_state = LoadState::Error(e.clone());
                    ctx.emit(SentimentModelEvent::Error(e));
                }
            },
        );
    }
}

impl Entity for SentimentModel {
    type Event = SentimentModelEvent;
}
