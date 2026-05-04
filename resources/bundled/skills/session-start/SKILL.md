---
name: session-start
description: Orient to the current repo at session start — handoff items, overdue todos, and next godmode task.
---

Run these in order:

1. `hj detect $PWD` — show any handoff items for this repo
2. `doob todo list --status pending --project $(basename $PWD) --json` — show overdue/pending todos
3. `godmode handon` — show task graph state and next runnable task

Summarize findings as:

- Handoff: N items (highest priority listed)
- Todos: N pending, N overdue
- Next task: <title> (or "no tasks")
