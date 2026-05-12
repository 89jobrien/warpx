//! AI context provider — CLAUDE.md files and MCP server names.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provider::{ContextProvider, ContextSection, KvRow, read_sidecar, write_sidecar};

const SIDECAR: &str = "ai.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AiState {
    claude_md_files: Vec<String>,
    mcp_servers: Vec<String>,
}

pub struct AiContextProvider;

impl ContextProvider for AiContextProvider {
    fn id(&self) -> &'static str {
        "ai_context"
    }

    fn label(&self) -> &str {
        "AI Context"
    }

    fn watch_paths(&self, cwd: &Path) -> Vec<PathBuf> {
        let home = home_dir();
        let mut paths = vec![
            crate::provider::context_dir().join(SIDECAR),
            cwd.join("CLAUDE.md"),
            home.join(".claude/CLAUDE.md"),
            home.join(".warp/.mcp.json"),
        ];
        // Walk up from cwd looking for CLAUDE.md files
        let mut dir = cwd.parent();
        while let Some(d) = dir {
            let candidate = d.join("CLAUDE.md");
            if candidate.exists() {
                paths.push(candidate);
            }
            dir = d.parent();
        }
        paths.dedup();
        paths
    }

    fn collect(&self, _cwd: &Path) -> Option<ContextSection> {
        let state: AiState = read_sidecar(SIDECAR)?;
        Some(to_section(&state))
    }

    fn generate(&self, cwd: &Path) -> ContextSection {
        let home = home_dir();
        let mut claude_files = Vec::new();

        let global = home.join(".claude/CLAUDE.md");
        if global.exists() {
            claude_files.push(global.display().to_string());
        }

        let mut dir: Option<&Path> = Some(cwd);
        while let Some(d) = dir {
            let candidate = d.join("CLAUDE.md");
            if candidate.exists() {
                claude_files.push(candidate.display().to_string());
            }
            dir = d.parent();
        }
        claude_files.dedup();

        let mcp_servers = read_mcp_server_names(&home.join(".warp/.mcp.json"));

        let state = AiState {
            claude_md_files: claude_files,
            mcp_servers,
        };

        if let Err(e) = write_sidecar(SIDECAR, &state) {
            log::warn!("[ai_context provider] sidecar write failed: {e}");
        }
        to_section(&state)
    }
}

fn to_section(state: &AiState) -> ContextSection {
    let home_str = std::env::var("HOME").unwrap_or_default();
    let mut rows = Vec::new();
    let mut sub_items = Vec::new();

    if !state.claude_md_files.is_empty() {
        rows.push(KvRow {
            key: "rules".into(),
            value: format!("{} files", state.claude_md_files.len()),
        });
        for path in &state.claude_md_files {
            let display = path
                .strip_prefix(&home_str)
                .map(|p| format!("~{p}"))
                .unwrap_or_else(|| path.clone());
            sub_items.push(display);
        }
    }

    if !state.mcp_servers.is_empty() {
        rows.push(KvRow {
            key: "MCP".into(),
            value: state.mcp_servers.join(", "),
        });
    }

    ContextSection::KeyValue { rows, sub_items }
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

fn read_mcp_server_names(path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let val: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    val.get("mcpServers")
        .or_else(|| val.get("servers"))
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}
