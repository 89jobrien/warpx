//! Handoff provider — items from `.ctx/HANDOFF.*.yaml` files.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, PriorityItem, read_sidecar, write_sidecar};

const SIDECAR: &str = "handoff.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HandoffState {
    items: Vec<HandoffEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HandoffEntry {
    summary: String,
    priority: String,
    status: String,
}

pub struct HandoffProvider;

impl ContextProvider for HandoffProvider {
    fn id(&self) -> &'static str {
        "handoff"
    }

    fn label(&self) -> &str {
        "Handoff"
    }

    fn watch_paths(&self, cwd: &Path) -> Vec<PathBuf> {
        let mut paths = vec![crate::provider::context_dir().join(SIDECAR)];
        let ctx_dir = cwd.join(".ctx");
        if ctx_dir.is_dir() {
            // Watch the directory itself — any HANDOFF.*.yaml change triggers.
            paths.push(ctx_dir);
        }
        // Also watch root HANDOFF.yaml if present.
        let root = cwd.join("HANDOFF.yaml");
        if root.exists() {
            paths.push(root);
        }
        paths
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: HandoffState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        let mut raw_items: Vec<crate::handoff::HandoffItem> = Vec::new();
        let ctx_dir = cwd.join(".ctx");
        if ctx_dir.is_dir() {
            crate::handoff::scan_ctx_dir(&ctx_dir, &mut raw_items);
        }

        let entries: Vec<HandoffEntry> = raw_items
            .into_iter()
            .map(|i| HandoffEntry {
                summary: i.title.unwrap_or_else(|| i.id.clone()),
                priority: i.priority.unwrap_or_default(),
                status: i.status.unwrap_or_default(),
            })
            .collect();

        let state = HandoffState { items: entries };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[handoff provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &HandoffState) -> ContextSection {
    if state.items.is_empty() {
        return ContextSection::PriorityList(vec![PriorityItem {
            priority: String::new(),
            text: "No handoff items".into(),
            status: String::new(),
        }]);
    }

    let items = state
        .items
        .iter()
        .filter(|i| i.status != "done")
        .map(|i| PriorityItem {
            priority: i.priority.clone(),
            text: i.summary.clone(),
            status: i.status.clone(),
        })
        .collect();

    ContextSection::PriorityList(items)
}
