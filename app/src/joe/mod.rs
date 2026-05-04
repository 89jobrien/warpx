//! JoeMode: personal ecosystem integration.
//!
//! Reads doob status cache and godmode task graph for ambient awareness.
//! Gated behind FeatureFlag::JoeMode — no-op when disabled.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// ── Doob status cache ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DoobStatusCache {
    pub updated_at: String,
    pub pending_total: usize,
    pub overdue_total: usize,
    pub overdue_by_repo: HashMap<String, usize>,
}

pub fn read_doob_cache() -> Option<DoobStatusCache> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home).join(".cache/doob/status.json");
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

// ── Godmode task graph ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct GodmodeState {
    pub pending: usize,
    pub running: usize,
    pub done: usize,
    pub blocked: usize,
    pub next_task: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawTask {
    status: String,
    title: Option<String>,
}

pub fn read_godmode_state(repo_path: &Path) -> Option<GodmodeState> {
    let yaml_path = repo_path.join(".ctx/GODMODE.tasks.yaml");
    let text = std::fs::read_to_string(&yaml_path).ok()?;
    let tasks: Vec<RawTask> = serde_yaml::from_str(&text).ok()?;

    let mut state = GodmodeState::default();
    for task in &tasks {
        match task.status.as_str() {
            "pending" => state.pending += 1,
            "running" => {
                state.running += 1;
                if state.next_task.is_none() {
                    state.next_task = task.title.clone();
                }
            }
            "done" => state.done += 1,
            "blocked" => state.blocked += 1,
            _ => {}
        }
    }
    // If nothing running, show first pending as next
    if state.next_task.is_none() {
        state.next_task = tasks
            .iter()
            .find(|t| t.status == "pending")
            .and_then(|t| t.title.clone());
    }
    Some(state)
}

// ── Combined refresh ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct JoeStatus {
    pub doob: Option<DoobStatusCache>,
    pub godmode: Option<GodmodeState>,
}

pub fn refresh(repo_path: &Path) -> JoeStatus {
    JoeStatus {
        doob: read_doob_cache(),
        godmode: read_godmode_state(repo_path),
    }
}
