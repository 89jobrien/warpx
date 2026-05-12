//! Todos provider — pending items from doob.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, PriorityItem, read_sidecar, write_sidecar};

const SIDECAR: &str = "todos.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TodoState {
    pending: Vec<TodoEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TodoEntry {
    name: String,
    priority: String,
    status: String,
}

pub struct TodosProvider;

impl ContextProvider for TodosProvider {
    fn id(&self) -> &'static str {
        "todos"
    }

    fn label(&self) -> &str {
        "Todos"
    }

    fn watch_paths(&self, _cwd: &Path) -> Vec<PathBuf> {
        // Doob state is a DB, not a file we can watch reliably.
        // Hooks write the sidecar; we watch that.
        vec![crate::provider::context_dir().join(SIDECAR)]
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: TodoState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        let items = match crate::doob::fetch_items() {
            Ok(all) => {
                let cwd_str = cwd.display().to_string();
                all.into_iter()
                    .filter(|i| {
                        i.status.as_deref() == Some("pending")
                            || i.status.as_deref() == Some("in_progress")
                    })
                    .filter(|i| {
                        i.project
                            .as_ref()
                            .map(|p| cwd_str.contains(p.as_str()))
                            .unwrap_or(true)
                    })
                    .take(15)
                    .map(|i| TodoEntry {
                        name: i.title.unwrap_or_else(|| "?".into()),
                        priority: i.priority.unwrap_or_else(|| "?".into()),
                        status: i.status.unwrap_or_else(|| "pending".into()),
                    })
                    .collect()
            }
            Err(e) => {
                log::warn!("[todos provider] doob fetch failed: {e}");
                return ContextSection::Unavailable(format!("doob: {e}"));
            }
        };

        let state = TodoState { pending: items };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[todos provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &TodoState) -> ContextSection {
    if state.pending.is_empty() {
        return ContextSection::PriorityList(vec![PriorityItem {
            priority: String::new(),
            text: "No pending items".into(),
            status: String::new(),
        }]);
    }

    let items = state
        .pending
        .iter()
        .take(10)
        .map(|i| PriorityItem {
            priority: i.priority.clone(),
            text: i.name.clone(),
            status: i.status.clone(),
        })
        .collect();

    ContextSection::PriorityList(items)
}
