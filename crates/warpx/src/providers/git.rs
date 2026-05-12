//! Git provider — branch, dirty count, recent commits.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, KvRow, read_sidecar, write_sidecar};

const SIDECAR: &str = "git.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GitState {
    branch: String,
    dirty_count: usize,
    recent_commits: Vec<String>,
}

pub struct GitProvider;

impl GitProvider {
    pub fn new(_cwd: &Path) -> Self {
        Self
    }
}

impl ContextProvider for GitProvider {
    fn id(&self) -> &'static str {
        "git"
    }

    fn label(&self) -> &str {
        "Git"
    }

    fn watch_paths(&self, cwd: &Path) -> Vec<PathBuf> {
        let mut paths = vec![crate::provider::context_dir().join(SIDECAR)];
        let git_dir = cwd.join(".git");
        if git_dir.is_dir() {
            paths.push(git_dir.join("HEAD"));
            paths.push(git_dir.join("index"));
        }
        paths
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: GitState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        let branch =
            run_git(cwd, &["branch", "--show-current"]).unwrap_or_else(|| "unknown".into());
        let status = run_git(cwd, &["status", "--porcelain"]).unwrap_or_default();
        let dirty_count = status.lines().filter(|l| !l.is_empty()).count();
        let log = run_git(cwd, &["log", "--oneline", "-5"]).unwrap_or_default();
        let recent_commits: Vec<String> = log.lines().map(|l| l.to_string()).collect();

        let state = GitState {
            branch,
            dirty_count,
            recent_commits,
        };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[git provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &GitState) -> ContextSection {
    let dirty_label = if state.dirty_count > 0 {
        format!("{} dirty", state.dirty_count)
    } else {
        "clean".into()
    };

    ContextSection::KeyValue {
        rows: vec![
            KvRow {
                key: "branch".into(),
                value: state.branch.clone(),
            },
            KvRow {
                key: "tree".into(),
                value: dirty_label,
            },
        ],
        sub_items: state.recent_commits.clone(),
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Option<String> {
    command::blocking::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
