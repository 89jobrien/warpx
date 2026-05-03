---
name: writing-plans
description: >
  Write an implementation plan as a markdown file, then immediately serialize it to a
  timestamped YAML task file in .ctx/tasks/. Use before touching any code when given a
  spec or approved design.
---

# Writing Plans

A plan is a complete implementation guide a fresh agent with no prior context could execute.
If it requires context not in the document, it is incomplete.

## Step 1: Write the plan markdown

Save to: `docs/superpowers/plans/YYYY-MM-DD-<feature-name>.md`

Structure:

```markdown
# <Title>

> **For agentic workers:** Run `/godmode:tackle-issues` to dispatch parallel
> subagents per task group. Use `/godmode:test-driven-development` for each
> implementation step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One sentence.

**Architecture:** Crates/files affected, data flow.

**Tech Stack:** Rust edition, key crates.

**Repo:** `/path/to/repo`

**GitHub issue:** #N (if applicable)

---

## Task 1: <name>

**Files:**

- `path/to/file.rs`

- [ ] **Step 1:** ...
- [ ] **Step 2:** ...
- [ ] **Commit:** `git commit -m "feat: ..."`

---

## Verification Checklist

- [ ] All tasks complete
- [ ] cargo check passes
- [ ] cargo clippy -- -D warnings clean
```

## Step 2: Serialize to .ctx/tasks/ (do this immediately after saving the markdown)

Create `.ctx/tasks/<YYYYMMDD>-<feature-name>.yaml` in the repo root.

Schema:

```yaml
id: <feature-name> # kebab-case, no date prefix
title: "<plan title>"
plan_file: docs/superpowers/plans/YYYY-MM-DD-<feature-name>.md
date: YYYY-MM-DD
repo: <repo dirname>
github_issue: N # null if none
status: pending # pending | in_progress | done
tasks:
  - id: t1
    title: "<task 1 title>"
    status: pending # pending | in_progress | done | blocked
    depends_on: []
    files:
      - path/to/file.rs
  - id: t2
    title: "<task 2 title>"
    status: pending
    depends_on: [t1]
    files: []
commits: []
```

Rules:

- One YAML task entry per `## Task N:` section in the markdown
- `depends_on` mirrors the sequential order of tasks unless explicitly independent
- `status` starts as `pending` for all tasks
- File path is the timestamped filename: `YYYYMMDD-<feature-name>.yaml`
  where `YYYYMMDD` is today's date

## Quality Rules

- No placeholders — every path and code block must be complete
- Each task ends with a commit step
- YAML must be valid — check with `python3 -c "import yaml; yaml.safe_load(open('.ctx/tasks/<file>'))"` if unsure
- Write the YAML immediately after the markdown — do not defer
