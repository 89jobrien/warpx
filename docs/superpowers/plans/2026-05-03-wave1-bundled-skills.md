# Wave 1 — Bundled Skills for godmode and hj

> **For agentic workers:** Run `/godmode:tackle-issues` to dispatch parallel
> subagents per task group. Use `/godmode:test-driven-development` for each
> implementation step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bundle godmode and hj session lifecycle skills into warpx so they
appear in Warp agent mode without the user manually installing them.

**Architecture:** Each skill is a directory under
`resources/bundled/skills/<name>/SKILL.md` — YAML front-matter + markdown
body. Warp picks them up automatically at startup. Skills are read-only;
user skills in `~/.warp/skills/` always take precedence.

**Tech Stack:** SKILL.md format (YAML + Markdown), no Rust changes needed.

**Repo:** `/Users/joe/dev/warpx`

**GitHub issue:** #3

**Source skills:**

- godmode: `~/.claude/plugins/cache/bazaar/godmode/1.1.0/skills/`
- hj/hand: `~/.claude/plugins/local-marketplace/plugins/hand/`

---

## Task 1: Audit existing bundled skills

**Files:**

- `resources/bundled/skills/`

- [ ] **Step 1: Read existing skill structure**

Read one existing skill (`session-start/SKILL.md`) to confirm exact YAML
front-matter fields required: `name`, `description`.

- [ ] **Step 2: List godmode skills to bundle**

Read skill directories from
`~/.claude/plugins/cache/bazaar/godmode/1.1.0/skills/`. Identify the core
subset to bundle (avoid internal/meta skills):

Priority skills to bundle:

- `test-driven-development`
- `systematic-debugging`
- `cap`
- `parallel-agents`
- `verification-before-completion`
- `brainstorm`
- `task-management`

- [ ] **Step 3: List hj skills to bundle**

Read skills from the `hand` plugin. Identify `handon` and `handoff` skills.

---

## Task 2: Bundle godmode skills

For each skill in the priority list:

- [ ] **godmode-tdd** — `resources/bundled/skills/godmode-tdd/SKILL.md`

```yaml
---
name: godmode-tdd
description: >
  Red-green-refactor TDD workflow for Rust. Write a failing test first,
  implement minimum code to pass, run cargo nextest, fix clippy, commit.
---
```

Body: concise summary of the TDD loop. Reference
`~/.claude/plugins/cache/bazaar/godmode/1.1.0/skills/test-driven-development/`
for content — do not copy verbatim if it's large; distill to key steps.

- [ ] **godmode-debug** — `resources/bundled/skills/godmode-debug/SKILL.md`

Distilled systematic debugging loop: hypothesis → minimal repro →
instrument → verify → fix → regression test.

- [ ] **godmode-cap** — `resources/bundled/skills/godmode-cap/SKILL.md`

Commit-and-push workflow: cargo check → clippy → nextest → git add → commit
(signed) → push. Never `--no-verify`.

- [ ] **godmode-parallel** — `resources/bundled/skills/godmode-parallel/SKILL.md`

When to use parallel agents, how to write self-contained prompts, wave
state protocol, integration steps.

- [ ] **godmode-verify** — `resources/bundled/skills/godmode-verify/SKILL.md`

Pre-completion checklist: tests green, clippy clean, no TODOs left, commit
SHA recorded.

- [ ] **godmode-brainstorm** — `resources/bundled/skills/godmode-brainstorm/SKILL.md`

Structured brainstorm: enumerate options, score trade-offs, recommend,
confirm before acting.

- [ ] **godmode-tasks** — `resources/bundled/skills/godmode-tasks/SKILL.md`

Task graph lifecycle: `godmode task list`, `task start`, `task done`,
`task block`, `task next`.

---

## Task 3: Bundle hj session skills

- [ ] **hj-handon** — `resources/bundled/skills/hj-handon/SKILL.md`

```yaml
---
name: hj-handon
description: >
  Orient at session start — triage HANDOFF items, pending doob todos,
  and next godmode task for the current repo.
---
```

Steps: run `handoff-detect $PWD`, `doob todo list --status pending --json`,
`godmode task next`. Summarize as: Handoff N items / Todos N pending /
Next task.

- [ ] **hj-handoff** — `resources/bundled/skills/hj-handoff/SKILL.md`

```yaml
---
name: hj-handoff
description: >
  Close a dev session — warn on running tasks, write HANDOFF entry,
  update doob todo statuses.
---
```

Steps: `godmode handoff`, verify no running tasks left, confirm HANDOFF
YAML updated with session summary and commit SHAs.

---

## Task 4: Verify skills appear in agent mode

- [ ] **Step 1: Build and run**

```bash
cargo run --manifest-path /Users/joe/dev/warpx/Cargo.toml --bin warpx
```

- [ ] **Step 2: Open agent mode**

Open Warp agent panel. Verify bundled skills appear in the skill list
(search for "godmode" and "hj").

- [ ] **Step 3: Smoke test one skill**

Invoke `godmode-cap` skill via agent mode. Confirm the agent receives the
correct skill body and acts on it.

- [ ] **Step 4: Commit**

```bash
git -C /Users/joe/dev/warpx add resources/bundled/skills/
git -C /Users/joe/dev/warpx commit -m \
  "feat: bundle godmode and hj skills into warpx closes #3"
```

---

## Verification Checklist

- [ ] All 9 skill directories created under `resources/bundled/skills/`
- [ ] Each `SKILL.md` has valid YAML front-matter (`name`, `description`)
- [ ] Skills visible in warpx agent mode skill list
- [ ] `godmode-cap` and `hj-handon` smoke-tested via agent
- [ ] No Rust changes — `cargo check` still passes
