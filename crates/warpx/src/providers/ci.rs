//! CI provider — last GitHub Actions run status for the current branch.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, StatusIcon, read_sidecar, write_sidecar};

const SIDECAR: &str = "ci.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CiState {
    status: String,
    conclusion: String,
    branch: String,
    workflow: String,
    url: String,
    elapsed: String,
}

pub struct CiProvider;

impl ContextProvider for CiProvider {
    fn id(&self) -> &'static str {
        "ci"
    }

    fn label(&self) -> &str {
        "CI"
    }

    fn watch_paths(&self, _cwd: &Path) -> Vec<PathBuf> {
        // Written by post-push-ci-watch hook; we watch the sidecar.
        vec![crate::provider::context_dir().join(SIDECAR)]
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: CiState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        let branch = run_git_branch(cwd).unwrap_or_else(|| "unknown".into());
        let output = command::blocking::Command::new("gh")
            .args([
                "run",
                "list",
                "--branch",
                &branch,
                "--limit",
                "1",
                "--json",
                "status,conclusion,name,url,updatedAt",
            ])
            .current_dir(cwd)
            .output();

        let state = match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                parse_gh_run(&text, &branch)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                log::warn!("[ci provider] gh run list failed: {stderr}");
                None
            }
            Err(e) => {
                log::warn!("[ci provider] gh not available: {e}");
                None
            }
        };

        let state = match state {
            Some(s) => s,
            None => {
                return ContextSection::Unavailable("no CI runs found".into());
            }
        };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[ci provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &CiState) -> ContextSection {
    let icon = match state.conclusion.as_str() {
        "success" => StatusIcon::Ok,
        "failure" | "timed_out" => StatusIcon::Error,
        "" if state.status == "in_progress" || state.status == "queued" => StatusIcon::Warn,
        _ => StatusIcon::Unknown,
    };

    let text = if state.conclusion.is_empty() {
        format!("{} ({}) {}", state.workflow, state.status, state.branch)
    } else {
        format!(
            "{} ({}) {} {}",
            state.workflow, state.conclusion, state.branch, state.elapsed
        )
    };

    ContextSection::StatusLine { icon, text }
}

fn run_git_branch(cwd: &Path) -> Option<String> {
    command::blocking::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn parse_gh_run(json_text: &str, branch: &str) -> Option<CiState> {
    let val: serde_json::Value = serde_json::from_str(json_text).ok()?;
    let arr = val.as_array()?;
    let run = arr.first()?;

    Some(CiState {
        status: run.get("status")?.as_str()?.to_string(),
        conclusion: run
            .get("conclusion")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        branch: branch.to_string(),
        workflow: run
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("CI")
            .to_string(),
        url: run
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        elapsed: run
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}
