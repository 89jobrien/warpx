---
name: godmode-tasks
description: Task graph management for a session. Use to create tasks, track progress, execute the next unblocked task, or manage dependencies between work items.
---

# Task Management

Godmode maintains a task graph at `.ctx/GODMODE.tasks.yaml` via the `godmode` CLI. Tasks
persist across sessions, encode causal dependencies, and drive sequential or parallel
execution. Never edit the YAML directly — always use the CLI.

## Session Start

```bash
godmode handon           # running, next runnable, blocked, next doob todo
godmode task next        # next runnable task(s)
godmode task next --json # machine-readable — exit 1 if empty
```

## Session End

```bash
godmode handoff          # warns on running tasks, calls hj handoff
```

## Task Operations

```bash
# Ingest from a plan doc
godmode plan ingest docs/plans/YYYY-MM-DD-<feature>.md

# Add manually
godmode task add <id> "<title>" [--depends-on t1,t2] [--crate-name <crate>]

# Lifecycle
godmode task start <id>
godmode task done <id> [--commit <sha>] [--notes "<text>"]
godmode task block <id> "<reason>"
godmode task unblock <id>

# List
godmode task list           # human table
godmode task list --json    # full JSON — exit 1 if empty
```

## Rules

- A task is **runnable** when all `depends_on` entries have `status: done`.
- A task is **blocked** when a dependency is blocked — do not continue past it.
- Only one task per causal chain runs at a time.
- Independent chains can run in parallel via `godmode-parallel`.

## Parallel Dispatch

```bash
godmode dispatch [--max 5] --json
```

Emits independent chains shaped for parallel agents. Each chain targets one crate.

## Example Workflow

```
godmode plan ingest docs/plans/2026-05-01-my-feature.md
godmode handon
godmode task start t1
# implement...
godmode task done t1 --commit abc1234
godmode task next    # → t2 is now runnable
godmode task start t2
# ...
godmode handoff
```
