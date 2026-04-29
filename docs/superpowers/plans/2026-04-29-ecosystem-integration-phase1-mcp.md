# Ecosystem Integration Phase 1: MCP Foundation

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add doob, handoff, and env-health MCP tools to personal-mcp so
they're immediately available in both Warp agent mode and Claude Code.

**Architecture:** Each tool group gets its own module file following the
existing pattern (request struct + handler function). Tools shell out to
existing CLIs (doob, handoff-db, handoff-detect, op) and return structured
JSON. A new `ecosystem` router joins infra and integrations in
`combined_router`.

**Tech Stack:** Rust, rmcp crate, serde/serde_json, std::process::Command

**Repo:** `/Users/joe/dev/personal-mcp`

---

### Task 1: Doob Tool Module

**Files:**

- Create: `src/doob.rs`
- Modify: `src/main.rs` (add mod + imports + router wiring)

- [ ] **Step 1: Write the failing test for doob_list parsing**

Create `src/doob.rs` with a test that validates JSON parsing of doob
CLI output:

```rust
use rmcp::{ErrorData, schemars};
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::util::{STDERR_LIMIT, STDOUT_LIMIT, truncate_output};

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DoobListRequest {
    /// Filter by status: "pending", "in-progress", "done", or "all"
    pub status: Option<String>,
    /// Filter by repo name
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DoobCompleteRequest {
    /// Todo ID(s) to mark complete
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct DoobAddRequest {
    /// Todo title
    pub title: String,
    /// Priority: 1 (highest) to 5 (lowest)
    pub priority: Option<u8>,
    /// Due date in YYYY-MM-DD format
    pub due: Option<String>,
    /// Associated repo name
    pub repo: Option<String>,
}

pub fn doob_list(req: DoobListRequest) -> Result<String, ErrorData> {
    let mut cmd = Command::new("doob");
    cmd.args(["todo", "list", "--format", "json"]);

    if let Some(ref status) = req.status {
        cmd.args(["--status", status]);
    }
    if let Some(ref repo) = req.repo {
        cmd.args(["--repo", repo]);
    }

    let output = cmd.output().map_err(|e| {
        ErrorData::internal_error(format!("failed to run doob: {e}"), None)
    })?;

    if !output.status.success() {
        let stderr = truncate_output(&output.stderr, STDERR_LIMIT);
        return Err(ErrorData::internal_error(
            format!("doob failed: {stderr}"),
            None,
        ));
    }

    Ok(truncate_output(&output.stdout, STDOUT_LIMIT))
}

pub fn doob_complete(req: DoobCompleteRequest) -> Result<String, ErrorData> {
    if req.ids.is_empty() {
        return Err(ErrorData::invalid_params(
            "ids cannot be empty".to_string(),
            None,
        ));
    }

    let mut cmd = Command::new("doob");
    cmd.args(["todo", "complete"]);
    for id in &req.ids {
        cmd.arg(id);
    }

    let output = cmd.output().map_err(|e| {
        ErrorData::internal_error(format!("failed to run doob: {e}"), None)
    })?;

    if !output.status.success() {
        let stderr = truncate_output(&output.stderr, STDERR_LIMIT);
        return Err(ErrorData::internal_error(
            format!("doob complete failed: {stderr}"),
            None,
        ));
    }

    Ok(truncate_output(&output.stdout, STDOUT_LIMIT))
}

pub fn doob_add(req: DoobAddRequest) -> Result<String, ErrorData> {
    if req.title.trim().is_empty() {
        return Err(ErrorData::invalid_params(
            "title cannot be empty".to_string(),
            None,
        ));
    }

    let mut cmd = Command::new("doob");
    cmd.args(["todo", "add", &req.title]);

    if let Some(priority) = req.priority {
        cmd.args(["--priority", &priority.to_string()]);
    }
    if let Some(ref due) = req.due {
        cmd.args(["--due", due]);
    }
    if let Some(ref repo) = req.repo {
        cmd.args(["--repo", repo]);
    }

    let output = cmd.output().map_err(|e| {
        ErrorData::internal_error(format!("failed to run doob: {e}"), None)
    })?;

    if !output.status.success() {
        let stderr = truncate_output(&output.stderr, STDERR_LIMIT);
        return Err(ErrorData::internal_error(
            format!("doob add failed: {stderr}"),
            None,
        ));
    }

    Ok(truncate_output(&output.stdout, STDOUT_LIMIT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doob_add_rejects_empty_title() {
        let req = DoobAddRequest {
            title: "".to_string(),
            priority: None,
            due: None,
            repo: None,
        };
        let result = doob_add(req);
        assert!(result.is_err());
    }

    #[test]
    fn doob_complete_rejects_empty_ids() {
        let req = DoobCompleteRequest { ids: vec![] };
        let result = doob_complete(req);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it compiles and validation tests pass**

Run: `cargo test -p personal-mcp doob`
Expected: 2 tests pass (empty title, empty ids)

- [ ] **Step 3: Wire doob module into main.rs**

Add to the top of `main.rs` after the existing mod declarations:

```rust
mod doob;
```

Add imports after existing integration imports:

```rust
use doob::{DoobAddRequest, DoobCompleteRequest, DoobListRequest};
```

Add tools to the `integrations_router` impl block (before the closing
`}`):

```rust
    #[tool(description = "List doob todos. Filter by status (pending/in-progress/done/all) and repo.")]
    fn doob_list(
        &self,
        Parameters(req): Parameters<DoobListRequest>,
    ) -> Result<String, ErrorData> {
        doob::doob_list(req)
    }

    #[tool(description = "Mark doob todo(s) as complete by ID.")]
    fn doob_complete(
        &self,
        Parameters(req): Parameters<DoobCompleteRequest>,
    ) -> Result<String, ErrorData> {
        doob::doob_complete(req)
    }

    #[tool(description = "Add a new doob todo with optional priority, due date, and repo.")]
    fn doob_add(
        &self,
        Parameters(req): Parameters<DoobAddRequest>,
    ) -> Result<String, ErrorData> {
        doob::doob_add(req)
    }
```

- [ ] **Step 4: Run tests and cargo check**

Run: `cargo check -p personal-mcp && cargo test -p personal-mcp doob`
Expected: compiles clean, 2 tests pass

- [ ] **Step 5: Commit**

```bash
git -C /Users/joe/dev/personal-mcp add src/doob.rs src/main.rs
git -C /Users/joe/dev/personal-mcp commit -m "feat: add doob MCP tools (list, complete, add)"
```

---

### Task 2: Handoff Tool Module

**Files:**

- Create: `src/handoff.rs`
- Modify: `src/main.rs` (add mod + imports + router wiring)

- [ ] **Step 1: Create handoff.rs with scan, read, and write handlers**

```rust
use rmcp::{ErrorData, schemars};
use serde::Deserialize;
use std::process::Command;

use crate::util::{STDERR_LIMIT, STDOUT_LIMIT, truncate_output};

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct HandoffScanRequest {
    /// Directory to scan for HANDOFF files (defaults to cwd)
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct HandoffReadRequest {
    /// Path to a specific HANDOFF file
    pub file: String,
}

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct HandoffWriteRequest {
    /// Repo path to write handoff for
    pub repo_path: String,
    /// JSON string of handoff entries to write
    pub entries_json: String,
}

pub fn handoff_scan(req: HandoffScanRequest) -> Result<String, ErrorData> {
    let mut cmd = Command::new("handoff-detect");
    if let Some(ref path) = req.path {
        cmd.arg(path);
    }

    let output = cmd.output().map_err(|e| {
        ErrorData::internal_error(
            format!("failed to run handoff-detect: {e}"),
            None,
        )
    })?;

    if !output.status.success() {
        let stderr = truncate_output(&output.stderr, STDERR_LIMIT);
        return Err(ErrorData::internal_error(
            format!("handoff-detect failed: {stderr}"),
            None,
        ));
    }

    Ok(truncate_output(&output.stdout, STDOUT_LIMIT))
}

pub fn handoff_read(req: HandoffReadRequest) -> Result<String, ErrorData> {
    if req.file.trim().is_empty() {
        return Err(ErrorData::invalid_params(
            "file path cannot be empty".to_string(),
            None,
        ));
    }

    let path = std::path::Path::new(&req.file);
    if !path.exists() {
        return Err(ErrorData::invalid_params(
            format!("file not found: {}", req.file),
            None,
        ));
    }

    std::fs::read_to_string(path).map_err(|e| {
        ErrorData::internal_error(format!("failed to read {}: {e}", req.file), None)
    })
}

pub fn handoff_write(req: HandoffWriteRequest) -> Result<String, ErrorData> {
    if req.repo_path.trim().is_empty() {
        return Err(ErrorData::invalid_params(
            "repo_path cannot be empty".to_string(),
            None,
        ));
    }

    // Validate entries_json is valid JSON
    serde_json::from_str::<serde_json::Value>(&req.entries_json).map_err(|e| {
        ErrorData::invalid_params(format!("invalid entries_json: {e}"), None)
    })?;

    let mut cmd = Command::new("handoff-db");
    cmd.args(["write", "--repo", &req.repo_path, "--entries", &req.entries_json]);

    let output = cmd.output().map_err(|e| {
        ErrorData::internal_error(
            format!("failed to run handoff-db: {e}"),
            None,
        )
    })?;

    if !output.status.success() {
        let stderr = truncate_output(&output.stderr, STDERR_LIMIT);
        return Err(ErrorData::internal_error(
            format!("handoff-db write failed: {stderr}"),
            None,
        ));
    }

    Ok(truncate_output(&output.stdout, STDOUT_LIMIT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_read_rejects_empty_path() {
        let req = HandoffReadRequest {
            file: "".to_string(),
        };
        assert!(handoff_read(req).is_err());
    }

    #[test]
    fn handoff_read_rejects_missing_file() {
        let req = HandoffReadRequest {
            file: "/nonexistent/HANDOFF.yaml".to_string(),
        };
        assert!(handoff_read(req).is_err());
    }

    #[test]
    fn handoff_write_rejects_empty_repo() {
        let req = HandoffWriteRequest {
            repo_path: "".to_string(),
            entries_json: "[]".to_string(),
        };
        assert!(handoff_write(req).is_err());
    }

    #[test]
    fn handoff_write_rejects_invalid_json() {
        let req = HandoffWriteRequest {
            repo_path: "/tmp/test".to_string(),
            entries_json: "not json".to_string(),
        };
        assert!(handoff_write(req).is_err());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p personal-mcp handoff`
Expected: 4 tests pass

- [ ] **Step 3: Wire handoff module into main.rs**

Add `mod handoff;` after `mod doob;`.

Add imports:

```rust
use handoff::{HandoffReadRequest, HandoffScanRequest, HandoffWriteRequest};
```

Add tools to `integrations_router`:

```rust
    #[tool(description = "Scan a directory tree for HANDOFF files and return their locations and priority items.")]
    fn handoff_scan(
        &self,
        Parameters(req): Parameters<HandoffScanRequest>,
    ) -> Result<String, ErrorData> {
        handoff::handoff_scan(req)
    }

    #[tool(description = "Read a specific HANDOFF file and return its contents.")]
    fn handoff_read(
        &self,
        Parameters(req): Parameters<HandoffReadRequest>,
    ) -> Result<String, ErrorData> {
        handoff::handoff_read(req)
    }

    #[tool(description = "Write handoff entries to handoff-db for a repo.")]
    fn handoff_write(
        &self,
        Parameters(req): Parameters<HandoffWriteRequest>,
    ) -> Result<String, ErrorData> {
        handoff::handoff_write(req)
    }
```

- [ ] **Step 4: Cargo check + test**

Run: `cargo check -p personal-mcp && cargo test -p personal-mcp handoff`
Expected: compiles, 4 tests pass

- [ ] **Step 5: Commit**

```bash
git -C /Users/joe/dev/personal-mcp add src/handoff.rs src/main.rs
git -C /Users/joe/dev/personal-mcp commit -m "feat: add handoff MCP tools (scan, read, write)"
```

---

### Task 3: Env Health Tool Module

**Files:**

- Create: `src/env_health.rs`
- Modify: `src/main.rs` (add mod + imports + router wiring)

- [ ] **Step 1: Create env_health.rs**

```rust
use rmcp::{ErrorData, schemars};
use serde::Deserialize;
use serde_json::json;
use std::process::Command;

use crate::util::{STDERR_LIMIT, truncate_output};

#[derive(Debug, Deserialize, rmcp::schemars::JsonSchema)]
pub struct EnvHealthRequest {
    /// Optional: check a specific project path (defaults to ~/dev)
    pub path: Option<String>,
}

pub fn env_health(req: EnvHealthRequest) -> Result<String, ErrorData> {
    let dev_path = req
        .path
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| format!("{h}/dev"))
                .unwrap_or_else(|_| "/Users/joe/dev".to_string())
        });

    // Check 1Password auth
    let op_ok = Command::new("op")
        .args(["account", "list", "--format", "json"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Check which secrets are loaded
    let expected_keys = [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "GH_AUTH_TOKEN",
    ];

    let loaded: Vec<&str> = expected_keys
        .iter()
        .filter(|k| std::env::var(k).is_ok())
        .copied()
        .collect();

    let missing: Vec<&str> = expected_keys
        .iter()
        .filter(|k| std::env::var(k).is_err())
        .copied()
        .collect();

    // Check nuenv allowlist
    let nuenv_allowed = Command::new("nu")
        .args(["-c", &format!(
            "if ('{dev_path}/.env.nu' | path exists) {{ 'present' }} else {{ 'absent' }}"
        )])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let status = if !op_ok {
        "red"
    } else if !missing.is_empty() {
        "amber"
    } else {
        "green"
    };

    let result = json!({
        "status": status,
        "op_authenticated": op_ok,
        "secrets_loaded": loaded.len(),
        "secrets_expected": expected_keys.len(),
        "loaded": loaded,
        "missing": missing,
        "nuenv_file": nuenv_allowed,
        "path": dev_path,
    });

    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_health_returns_json() {
        // This test runs against real env -- just verify it returns
        // valid JSON without panicking
        let req = EnvHealthRequest {
            path: Some("/tmp".to_string()),
        };
        let result = env_health(req);
        assert!(result.is_ok());
        let json: serde_json::Value =
            serde_json::from_str(&result.unwrap()).unwrap();
        assert!(json.get("status").is_some());
        assert!(json.get("op_authenticated").is_some());
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p personal-mcp env_health`
Expected: 1 test passes

- [ ] **Step 3: Wire into main.rs**

Add `mod env_health;` after `mod handoff;`.

Add import:

```rust
use env_health::EnvHealthRequest;
```

Add tool to `integrations_router`:

```rust
    #[tool(description = "Check environment health: 1Password auth, loaded secrets, nuenv status. Returns JSON with status (green/amber/red).")]
    fn env_health(
        &self,
        Parameters(req): Parameters<EnvHealthRequest>,
    ) -> Result<String, ErrorData> {
        env_health::env_health(req)
    }
```

- [ ] **Step 4: Cargo check + test**

Run: `cargo check -p personal-mcp && cargo test -p personal-mcp env_health`
Expected: compiles, 1 test passes

- [ ] **Step 5: Commit**

```bash
git -C /Users/joe/dev/personal-mcp add src/env_health.rs src/main.rs
git -C /Users/joe/dev/personal-mcp commit -m "feat: add env_health MCP tool"
```

---

### Task 4: Update ServerInfo and Integration Test

**Files:**

- Modify: `src/main.rs` (update ServerInfo instructions string)

- [ ] **Step 1: Update the instructions string in ServerInfo**

In the `get_info` method, the instructions string already lists doob,
minibox, etc. Verify the doob and handoff entries match the actual tool
names we added. Update if needed to reflect the exact tool names:

```
- Doob: doob_list, doob_complete, doob_add\n\
- Handoff: handoff_scan, handoff_read, handoff_write\n\
- Environment: env_health\n\
```

- [ ] **Step 2: Run full test suite**

Run: `cargo test -p personal-mcp`
Expected: all tests pass (doob, handoff, env_health, plus existing)

- [ ] **Step 3: Build release and verify MCP server starts**

Run: `cargo build -p personal-mcp && echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' | timeout 5 cargo run -p personal-mcp 2>/dev/null | head -5`
Expected: JSON response with server capabilities listing all tools

- [ ] **Step 4: Commit**

```bash
git -C /Users/joe/dev/personal-mcp add src/main.rs
git -C /Users/joe/dev/personal-mcp commit -m "feat: update ServerInfo with ecosystem tool descriptions"
```

---

## Phase 2-4 Plans

Phase 2 (Terminal-Native), Phase 3 (Warpx Panels), and Phase 4 (Warp
Skills) are separate plans to be written after Phase 1 is complete and
the MCP tools are verified working. Phase 3 is a candidate for
orca-strait orchestration.

**Dependency chain:**

- Phase 2 depends on: Phase 1 (MCP tools) + nuenv (already installed)
- Phase 3 depends on: Phase 1 (MCP tools to call from panels)
- Phase 4 depends on: Phase 1 + Phase 3 (skills reference panels)
