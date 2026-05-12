//! Cargo state provider — last check/clippy result.
//!
//! The sidecar is written by `post-edit-cargo-check.nu` and
//! `post-edit-cargo-clippy.nu` hooks. The `generate` fallback runs
//! `cargo check` directly.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, StatusIcon, read_sidecar, write_sidecar};

const SIDECAR: &str = "cargo.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CargoState {
    check_ok: bool,
    warning_count: usize,
    clippy_ok: bool,
    error_summary: String,
}

pub struct CargoStateProvider;

impl ContextProvider for CargoStateProvider {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn label(&self) -> &str {
        "Cargo"
    }

    fn watch_paths(&self, _cwd: &Path) -> Vec<PathBuf> {
        // Written by post-edit hooks; watch the sidecar.
        vec![crate::provider::context_dir().join(SIDECAR)]
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: CargoState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        // Only run `cargo check` — clippy is too slow for a generate fallback.
        let output = command::blocking::Command::new("cargo")
            .args(["check", "--message-format=json", "--quiet"])
            .current_dir(cwd)
            .output();

        let state = match output {
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let warning_count = stderr.lines().filter(|l| l.contains("warning")).count();
                let error_summary = if o.status.success() {
                    String::new()
                } else {
                    stderr
                        .lines()
                        .filter(|l| l.starts_with("error"))
                        .take(3)
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                CargoState {
                    check_ok: o.status.success(),
                    warning_count,
                    clippy_ok: true, // not run in generate
                    error_summary,
                }
            }
            Err(e) => {
                return ContextSection::Unavailable(format!("cargo: {e}"));
            }
        };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[cargo provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &CargoState) -> ContextSection {
    if !state.check_ok {
        let text = if state.error_summary.is_empty() {
            "check failed".into()
        } else {
            format!(
                "check failed: {}",
                state.error_summary.lines().next().unwrap_or("")
            )
        };
        return ContextSection::StatusLine {
            icon: StatusIcon::Error,
            text,
        };
    }

    if state.warning_count > 0 {
        return ContextSection::StatusLine {
            icon: StatusIcon::Warn,
            text: format!(
                "check ok, {} warning{}",
                state.warning_count,
                if state.warning_count == 1 { "" } else { "s" }
            ),
        };
    }

    ContextSection::StatusLine {
        icon: StatusIcon::Ok,
        text: "check ok".into(),
    }
}
