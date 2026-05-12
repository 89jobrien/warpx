//! Environment health provider — toolchain versions, auth status.
//!
//! Checks: rust toolchain, cargo, git, gh, doob, op (1Password),
//! direnv loaded status.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{
    ContextProvider, ContextSection, KvRow, StatusIcon, read_sidecar, write_sidecar,
};

const SIDECAR: &str = "env.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EnvState {
    checks: Vec<EnvCheck>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EnvCheck {
    name: String,
    ok: bool,
    detail: String,
}

pub struct EnvHealthProvider;

impl ContextProvider for EnvHealthProvider {
    fn id(&self) -> &'static str {
        "env_health"
    }

    fn label(&self) -> &str {
        "Environment"
    }

    fn watch_paths(&self, _cwd: &Path) -> Vec<PathBuf> {
        // Only refreshed on manual trigger or session start.
        vec![crate::provider::context_dir().join(SIDECAR)]
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: EnvState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, _cwd: &Path) -> ContextSection {
        let checks = vec![
            check_tool("rustc", &["--version"]),
            check_tool("cargo", &["--version"]),
            check_tool("git", &["--version"]),
            check_tool("gh", &["--version"]),
            check_tool("doob", &["--version"]),
            check_op_auth(),
            check_direnv(),
        ];

        let state = EnvState { checks };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[env_health provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &EnvState) -> ContextSection {
    let failed: Vec<&EnvCheck> = state.checks.iter().filter(|c| !c.ok).collect();

    if failed.is_empty() {
        return ContextSection::StatusLine {
            icon: StatusIcon::Ok,
            text: format!("{}/{} tools ok", state.checks.len(), state.checks.len()),
        };
    }

    // Show failures as KV rows so each gets its own line.
    let rows: Vec<KvRow> = state
        .checks
        .iter()
        .map(|c| KvRow {
            key: c.name.clone(),
            value: if c.ok {
                c.detail.clone()
            } else {
                format!("MISSING: {}", c.detail)
            },
        })
        .collect();

    ContextSection::KeyValue {
        rows,
        sub_items: vec![],
    }
}

fn check_tool(name: &str, args: &[&str]) -> EnvCheck {
    let output = command::blocking::Command::new(name).args(args).output();
    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            EnvCheck {
                name: name.into(),
                ok: true,
                detail: version,
            }
        }
        Ok(o) => EnvCheck {
            name: name.into(),
            ok: false,
            detail: format!("exit {}", o.status),
        },
        Err(e) => EnvCheck {
            name: name.into(),
            ok: false,
            detail: format!("{e}"),
        },
    }
}

fn check_op_auth() -> EnvCheck {
    let output = command::blocking::Command::new("op")
        .args(["account", "list", "--format=json"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let count = text
                .parse::<serde_json::Value>()
                .ok()
                .and_then(|v| v.as_array().map(|a| a.len()))
                .unwrap_or(0);
            EnvCheck {
                name: "1password".into(),
                ok: count > 0,
                detail: format!("{count} account(s)"),
            }
        }
        _ => EnvCheck {
            name: "1password".into(),
            ok: false,
            detail: "op CLI not available".into(),
        },
    }
}

fn check_direnv() -> EnvCheck {
    // Check if DIRENV_DIR is set (meaning direnv has loaded).
    let loaded = std::env::var("DIRENV_DIR").is_ok();
    EnvCheck {
        name: "direnv".into(),
        ok: loaded,
        detail: if loaded {
            "loaded".into()
        } else {
            "not loaded".into()
        },
    }
}
