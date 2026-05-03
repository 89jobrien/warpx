---
name: hj-handon
description: Session-start handoff reader. Use at the start of a session to orient to outstanding work — scans for HANDOFF.yaml, triages items by priority, and acts accordingly.
---

# hj-handon — Session Start

Scan for handoff files, parse items by priority, and act.

## Steps

### 1. Find handoff file

```bash
handoff-detect   # exit 0 = file found; exit 2 = missing; exit 1 = not in git repo
```

If exit 2 and `HANDOFF.md` exists at repo root — read it as freeform. Do not convert unless
asked.

### 2. Sync from doob

```bash
doob handoff sync --file <path-to-HANDOFF.yaml>
```

Skip if `doob` is not on PATH. Continue with local file as-is.

### 3. Review on wake

Before triaging, surface any `type: human-edit` entries with no `reviewed` field. Present
these first. Wait for user acknowledgement before proceeding.

### 4. Triage by priority

| Priority | Action                                                                       |
| -------- | ---------------------------------------------------------------------------- |
| P0       | Validate current state. Report to user. Ask before touching anything.        |
| P1       | Execute autonomously. Stop if scope expands or something unexpected happens. |
| P2       | Delegate to subagents. Cap at 5 concurrent.                                  |

Do not proceed to P1/P2 until all P0s are acknowledged.

### 5. Report

```
## Handoff Triage — <repo>

P0: [id] "[title]" → <state found> → <action / question>
P1: [id] "[title]" → done | blocked: <reason>
P2: [id] "[title]" → delegated | skipped: <reason>
```

Then update HANDOFF.yaml: mark done items, prepend log entry, run `doob handoff sync`.

## Edge Cases

- No handoff file: offer to create via `/handoff`.
- All items done or parked: report clean state, no action needed.
- Blocked item: report the blocker verbatim. Do not attempt.
