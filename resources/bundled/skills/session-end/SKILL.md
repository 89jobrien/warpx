---
name: session-end
description: Close the session — write handoff state and summarize what was accomplished.
---

Run these in order:

1. `git log --oneline -5` — show recent commits
2. `godmode task list --json` — show task graph final state
3. `godmode handoff` — validate session state and trigger handoff write

Summarize:

- Commits made this session
- Tasks completed vs remaining
- Any blocked tasks and why
