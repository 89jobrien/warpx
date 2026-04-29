---
name: daily-workflow
description: Morning and end-of-day rituals for development sessions. Handles orientation (handon), status checks, handoff writing, and daily note updates. Use at session start or end.
---

# Daily Workflow

Session bookends for development work.

## Morning Ritual

1. Check git status across active repos
2. Read HANDOFF.yaml for outstanding work
3. Surface overdue todos
4. Summarize what needs attention today

## End-of-Day Ritual

1. Run handoff: capture completed work, new gaps, current state
2. Update todos: mark completed items, add new discoveries
3. Write session summary to daily note
4. Verify all changes are committed and pushed

## Conventions

- HANDOFF files live in .ctx/ directory
- Use doob CLI for todo management
- Daily notes go to the Obsidian vault
