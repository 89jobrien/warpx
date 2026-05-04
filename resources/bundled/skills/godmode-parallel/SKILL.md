---
name: godmode-parallel
description: Dispatch parallel subagents for independent tasks. Use when 2+ tasks have no shared state or sequential dependencies.
---

# Parallel Agents

Dispatch one agent per independent domain. Let them work concurrently.

## When to Use

Use when:

- Multiple task chains in `.ctx/GODMODE.tasks.yaml` have no shared `depends_on`
- Multiple crates need implementation and they don't share types
- Multiple test failures originate from different subsystems

Do NOT use when:

- Tasks share state or cross-crate types
- One task produces types consumed by another
- You're still in exploratory debugging (unclear scope)

## Dispatch Protocol

### Step 1: Identify independent units

Find all tasks where `depends_on` is empty or all dependencies are `done`. Group into chains.
Chains with no shared nodes are independent.

### Step 2: Prepare focused prompts

Each agent prompt must be self-contained, scoped to one crate or domain, explicit about
exact file paths and test names, and constrained: "Do NOT modify files outside `crates/<crate>/`."

### Step 3: Initialize wave state

Write `.ctx/wave-status.json` before dispatching:

```json
{
  "wave": 1,
  "agents": {
    "<crate>": { "status": "pending", "branch": "", "commits": [] }
  }
}
```

### Step 4: Dispatch concurrently

Cap at **5 concurrent agents**. Always pass `allowedTools` explicitly — agents do NOT inherit
Bash permissions from parent.

### Step 5: Integrate results

After all agents report:

1. Verify no agent has `status: "pending"` or empty `commits` list.
2. Run full workspace suite:
   ```bash
   cargo nextest run --workspace
   cargo clippy --workspace -- -D warnings
   ```
3. Merge each branch sequentially with `--no-ff` (never octopus-merge).
4. Update `.ctx/GODMODE.tasks.yaml`: mark completed tasks `done`.
5. Archive: `mv .ctx/wave-status.json .ctx/wave-<N>-complete.json`

## Guardrails

- Crate isolation is the unit of parallelism. Never dispatch two agents for the same crate.
- 3 failed attempts → write `BLOCKED.md` → stop. Do not retry with identical parameters.
- Each agent must verify `git branch --show-current` before every commit. If `main` — stop.
- Never octopus-merge. Cherry-pick sequentially if branches diverge.
