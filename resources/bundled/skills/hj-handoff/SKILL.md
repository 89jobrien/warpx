---
name: hj-handoff
description: Session-end handoff writer. Use at end of a session to update HANDOFF.yaml with completed work, new gaps, and current project state.
---

# hj-handoff — Session End

Write HANDOFF.yaml with completed work, new gaps discovered, and current project state.

## File Layout

| File                            | Location  | Committed | Purpose                                  |
| ------------------------------- | --------- | --------- | ---------------------------------------- |
| `HANDOFF.<project>.<base>.yaml` | repo root | yes       | Tasks, items, log — doob source of truth |
| `.ctx/HANDOFF.state.yaml`       | `.ctx/`   | no        | Build/tests/branch snapshot              |
| `.ctx/HANDOFF.md`               | `.ctx/`   | no        | Generated reference doc                  |

`.ctx/` must be in `.gitignore`. Never commit anything under it.

## Steps

### 1. Get current state

```bash
git branch --show-current
git log --oneline -5
cargo check 2>&1 | tail -3
cargo nextest run 2>&1 | tail -5
```

### 2. Resolve HANDOFF.yaml path

```bash
handoff-detect              # path if exists; expected path + exit 2 if not
handoff-detect --name       # expected filename
```

Naming: `HANDOFF.<project>.<cwd-basename>.yaml`

### 3. Update HANDOFF.yaml

- New gap → append with new `id` (no `doob_uuid` yet)
- Completed → set `status: done`, add `completed: <today>`
- Blocked → set `status: blocked`, append `extra` entry with `type: blocker`
- Do NOT edit `title`, `description`, `priority`, or `files` on existing items
- `log` — prepend a new entry (newest first), one line, past tense, with commit hashes
- `updated` — set to today

### 4. Write .ctx/HANDOFF.state.yaml

```yaml
updated: <YYYY-MM-DD>
branch: <git branch>
build: clean | failing | unknown
tests: "<N passing>" | "failing: N" | "unknown"
notes: <one-line or null>
```

### 5. Sync to doob

```bash
doob handoff sync --file <path-to-HANDOFF.yaml>
```

### 6. Commit

```bash
git add HANDOFF.yaml
git commit -m "docs: update handoff"
```

Stage only `HANDOFF.yaml`. Never stage anything under `.ctx/`.

## Priority Guide

| Priority | Meaning                                 |
| -------- | --------------------------------------- |
| P0       | Broken, blocked, security, data loss    |
| P1       | Known fix, clear scope, safe to execute |
| P2       | Safe to delegate, well-understood       |
