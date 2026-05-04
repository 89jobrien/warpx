---
name: task-triage
description: Show the current task graph and next runnable task for this repo using godmode.
---

Run `godmode handon` to surface the session state, then `godmode task next` to show what to work on next.

If `.ctx/GODMODE.tasks.yaml` doesn't exist, run `godmode task list` and report "no tasks".

Show:

- Running tasks (if any)
- Next runnable task(s)
- Blocked tasks with reasons
- Total done/pending counts
