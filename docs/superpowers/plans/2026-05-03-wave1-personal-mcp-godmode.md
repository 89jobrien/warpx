# Wave 1 — personal-mcp godmode MCP Tools

> **Status: DONE** — Implemented on branch `feat/godmode-mcp-tools`,
> commit `8757955`. This plan documents what was built for reference.

**Goal:** Expose godmode CLI operations as MCP tools in personal-mcp so
Warp agent mode and Claude Code can invoke them directly without shelling
out manually.

**Architecture:** New `godmode` router module in personal-mcp following the
existing `ecosystem` router pattern. Each tool maps to a `godmode` CLI
subcommand. Tools return structured JSON.

**Tech Stack:** Rust, rmcp crate, serde/serde_json, std::process::Command

**Repo:** `/Users/joe/dev/personal-mcp`

**Branch:** `feat/godmode-mcp-tools` (merged)

---

## Tools Implemented

### godmode_task_list

Returns all tasks from `.ctx/GODMODE.tasks.yaml` as structured JSON.

```
Input: { "cwd": string }
Output: { "tasks": [{ "id", "title", "status", "depends_on", "crate" }] }
```

Shells out to: `godmode task list --json`

### godmode_task_next

Returns the next runnable task (all dependencies done).

```
Input: { "cwd": string }
Output: { "task": { ... } | null }
```

Shells out to: `godmode task next --json`

### godmode_task_done

Marks a task as done with an optional commit SHA.

```
Input: { "cwd": string, "task_id": string, "commit": string? }
Output: { "ok": bool }
```

Shells out to: `godmode task done <id> [--commit <sha>]`

### godmode_handon

Runs session-start triage: running tasks, next runnable, next doob todo.

```
Input: { "cwd": string }
Output: { "summary": string }
```

Shells out to: `godmode handon`

### godmode_handoff

Runs session-end closeout: warns on running tasks, writes hj handoff.

```
Input: { "cwd": string }
Output: { "summary": string }
```

Shells out to: `godmode handoff`

---

## Verification (completed 2026-05-03)

- [x] Tools appear in Warp agent mode tool list
- [x] `godmode_task_list` returns live task graph
- [x] `godmode_task_next` returns correct next task
- [x] `godmode_task_done` marks task done and persists to YAML
- [x] `godmode_handon` / `godmode_handoff` surface session context
- [x] All tools return structured JSON, never raw CLI output
- [x] Errors surfaced as `{ "error": "..." }` — never panic
