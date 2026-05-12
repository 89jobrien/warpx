//! Worktrees provider — active git worktrees and their branch status.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, PriorityItem, read_sidecar, write_sidecar};

const SIDECAR: &str = "worktrees.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorktreeState {
    trees: Vec<WorktreeEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorktreeEntry {
    path: String,
    branch: String,
    /// Number of commits ahead of main/develop.
    ahead: usize,
}

pub struct WorktreesProvider;

impl ContextProvider for WorktreesProvider {
    fn id(&self) -> &'static str {
        "worktrees"
    }

    fn label(&self) -> &str {
        "Worktrees"
    }

    fn watch_paths(&self, _cwd: &Path) -> Vec<PathBuf> {
        vec![crate::provider::context_dir().join(SIDECAR)]
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: WorktreeState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        let output = command::blocking::Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(cwd)
            .output();

        let trees = match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                parse_worktree_porcelain(&text, cwd)
            }
            _ => vec![],
        };

        let state = WorktreeState { trees };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[worktrees provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &WorktreeState) -> ContextSection {
    // Filter out the main worktree (bare repo root) — only show extras.
    let extras: Vec<&WorktreeEntry> = state
        .trees
        .iter()
        .skip(1) // first entry is always the main worktree
        .collect();

    if extras.is_empty() {
        return ContextSection::Unavailable("no extra worktrees".into());
    }

    let items = extras
        .iter()
        .map(|t| {
            let ahead_label = if t.ahead > 0 {
                format!(" (+{} ahead)", t.ahead)
            } else {
                String::new()
            };
            PriorityItem {
                priority: if t.ahead > 0 {
                    "P1".into()
                } else {
                    String::new()
                },
                text: format!("{}{ahead_label}", t.branch),
                status: String::new(),
            }
        })
        .collect();

    ContextSection::PriorityList(items)
}

fn parse_worktree_porcelain(text: &str, cwd: &Path) -> Vec<WorktreeEntry> {
    let mut trees = Vec::new();
    let mut path = String::new();
    let mut branch = String::new();

    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            branch = b.to_string();
        } else if line.is_empty() && !path.is_empty() {
            let ahead = count_ahead(cwd, &branch);
            trees.push(WorktreeEntry {
                path: std::mem::take(&mut path),
                branch: std::mem::take(&mut branch),
                ahead,
            });
        }
    }
    // Handle last entry if no trailing blank line
    if !path.is_empty() {
        let ahead = count_ahead(cwd, &branch);
        trees.push(WorktreeEntry {
            path,
            branch,
            ahead,
        });
    }

    trees
}

fn count_ahead(cwd: &Path, branch: &str) -> usize {
    if branch.is_empty() {
        return 0;
    }
    // Try common main branch names
    for main_branch in &["main", "develop", "master"] {
        let range = format!("{main_branch}..{branch}");
        let output = command::blocking::Command::new("git")
            .args(["rev-list", "--count", &range])
            .current_dir(cwd)
            .output();
        if let Ok(o) = output
            && o.status.success()
        {
            let count = String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<usize>()
                .unwrap_or(0);
            return count;
        }
    }
    0
}
