# Warpx Ecosystem Integration Design

**Date:** 2026-04-29
**Status:** approved
**Scope:** warpx, personal-mcp, dotfiles

## Overview

Integrate warpx (personal Warp terminal fork) as the control plane and
ecosystem-aware terminal for Joe's software ecosystem: doob, handoff-db,
devloop, minibox, braid, sanctum, and supporting tools.

Three integration layers:

1. **Dedicated panels** for high-interaction systems (Doob, Handoff)
2. **Aggregated dashboard** for glanceable health (env, devloop, minibox,
   agents)
3. **Terminal-native hooks** for ambient awareness (prompt badges, session
   lifecycle, directory-change triggers)

All data flows through personal-mcp as the universal bridge, so every tool
works in both Warp agent mode and Claude Code.

## Data Layer: personal-mcp as Universal Bridge

New MCP tool groups added to personal-mcp. Each returns structured JSON.

| Group       | Tools                                                   | Data source               |
| ----------- | ------------------------------------------------------- | ------------------------- |
| `doob_*`    | `doob_list`, `doob_complete`, `doob_add`, `doob_triage` | doob CLI / SQLite         |
| `handoff_*` | `handoff_read`, `handoff_write`, `handoff_scan`         | handoff-db / YAML         |
| `env_*`     | `env_health`, `env_resolve`, `env_trace`                | op CLI, direnv, nuenv     |
| `devloop_*` | `devloop_status`, `devloop_analyze`                     | devloop CLI / JSON        |
| `minibox_*` | `minibox_status`, `minibox_ssh`                         | minibox CLI / Tailscale   |
| `agent_*`   | `agent_list`, `agent_status`                            | braid CLI / process table |

Existing tools (`host_info`, `read_notes`, `run_cmd`, `append_note`) stay
as-is.

## Dedicated Panels

### Doob Panel (left sidebar)

- List view with filtering: pending/in-progress/done, by repo, by priority
- Inline actions: complete, set due date, change priority
- Overdue items highlighted at top
- Reads from `doob_list`, writes via `doob_complete`/`doob_add`
- Refreshes on sidebar open and on directory change
- Location: `app/src/left_sidebar/doob/`

### Handoff Panel (already exists, enhancements)

- Priority color coding (P0 red, P1 amber, P2 default)
- "Start session" button runs handon logic via `handoff_scan`
- "End session" button triggers handoff write via `handoff_write`
- Cross-link with doob tasks
- Location: existing `app/src/left_sidebar/` handoff code

Both panels follow WarpUI Entity-Component-Handle pattern. Data flows
through personal-mcp -- panels never shell out directly.

## Ecosystem Dashboard

Single "Ecosystem" panel in the left sidebar (below Doob, below Handoff).
Aggregated health at a glance with expandable rows.

| Row           | MCP tool         | Display                                          |
| ------------- | ---------------- | ------------------------------------------------ |
| Env Health    | `env_health`     | Green/amber/red dot + loaded vs expected secrets |
| Devloop       | `devloop_status` | Last analysis timestamp + P0/P1/P2 counts        |
| Minibox       | `minibox_status` | VM name + up/down + Tailscale IP                 |
| Active Agents | `agent_list`     | Count of running braid agents + names            |

Each row clickable to expand detail or open a terminal command. Polls on
configurable interval (default 60s) via MCP.

## Terminal-Native Integration

### Session Lifecycle Hooks

- **Shell open**: auto-runs handon -- reads handoff-db, surfaces top
  priority items as styled terminal banner via `handoff_scan`
- **Shell close / tab close**: triggers handoff write -- snapshots current
  state via `handoff_write`
- **Directory change**: nuenv loads `.env.nu`, fires `doob_list` filtered
  to current repo, shows overdue count if > 0

### Starship Prompt Integration

- Custom starship module: `[doob: 3 overdue]` when count > 0
- Reads from `~/.cache/doob/status.json` (doob CLI writes on every
  mutation)
- Amber when 1-3 overdue, red when > 3

### Warp Agent Mode

- All personal-mcp tools available automatically (registered in
  `~/.warp/.mcp.json`)
- Full ecosystem access: "show my overdue tasks", "what's the minibox
  status", "run devloop analysis"
- Bundled Warp skills can reference MCP tools (e.g. `daily-triage` skill)

## Implementation Phases

### Phase 1: MCP Foundation (personal-mcp)

- `doob_list`, `doob_complete`, `doob_add` tools
- `handoff_scan`, `handoff_read` tools
- `env_health` tool
- Immediately usable in Warp agent mode and Claude Code

### Phase 2: Terminal-Native

- `doob status.json` cache file
- Starship custom module
- Nuenv directory-change hook for doob overdue count
- Session open/close hooks for handon/handoff

### Phase 3: Doob Panel

- Left sidebar panel, Entity-Component-Handle pattern
- List/filter/complete/add via MCP tools
- Priority sorting, overdue highlighting

### Phase 4: Ecosystem Dashboard

- `devloop_status`, `minibox_status`, `agent_list` MCP tools
- Dashboard panel with aggregated rows
- 60s polling interval

### Phase 5: Handoff Panel Enhancements

- Priority color coding
- Start/end session buttons wired to MCP
- Cross-link with doob tasks

### Phase 6: Warp Skills

- `daily-triage` bundled skill
- `env-debug` skill for Warp agent mode
- Port key atelier skills as Warp-native skills

Each phase is independently shippable.

## Upstream Merge Strategy

Custom code lives in isolated locations to minimize conflicts:

| Custom code          | Location                          | Conflict risk            |
| -------------------- | --------------------------------- | ------------------------ |
| JoeMode / JoeHarness | `app/src/joe/`                    | None -- new directory    |
| Doob panel           | `app/src/left_sidebar/doob/`      | Low -- new directory     |
| Dashboard panel      | `app/src/left_sidebar/ecosystem/` | Low -- new directory     |
| Feature flags        | `crates/warp_features/src/lib.rs` | Medium -- append         |
| Sidebar wiring       | `app/src/left_sidebar/mod.rs`     | Medium -- adding entries |
| MCP tools            | `personal-mcp/` (separate repo)   | None                     |
| Starship module      | `dotfiles/` (separate repo)       | None                     |
| Nu hooks             | `dotfiles/` (separate repo)       | None                     |

Strategy: `main` tracks upstream, rebase `joe/main` periodically. Feature
flags gate all custom panels so they compile cleanly even if wiring
conflicts arise.
