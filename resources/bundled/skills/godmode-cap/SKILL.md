---
name: godmode-cap
description: Commit-and-push workflow with validation. Use when asked to "cap", "commit and push", or "ship it".
---

# Cap — Commit and Push

Run the full validation gate, commit, and push in one pass.

## Step 1: Validate

```bash
cargo check --workspace
cargo nextest run --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```

- If `cargo check` or tests fail — **stop**. Report failures. Do not commit broken code.
- If `cargo fmt --check` fails — run `cargo fmt --all` to fix, then continue.
- If clippy warnings exist — fix them before committing.

## Step 2: Stage

```bash
git add -A
git diff --cached --stat
```

Review staged files. If unrelated changes are staged, report and ask before proceeding.

## Step 3: Write commit message

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <summary>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`
Scope: crate name or module. Derive message from the staged diff — do not ask the user.

## Step 4: Verify branch and commit

```bash
git branch --show-current
```

If output is `main` and user did not explicitly ask to commit to main — stop and report.

```bash
git commit -m "<message>"
```

**Hook failure recovery**: read the hook output, identify the exact rule. False positive →
add minimum exclusion and retry once. Real issue → fix, re-stage, retry once. Still failing
after one retry → stop and report. Never use `--no-verify`.

## Step 5: Push

```bash
git push
```

If push is rejected (non-fast-forward): report to user. Do not force-push without explicit
instruction.

## Step 6: Sync task state (if applicable)

If a godmode task is currently `running`:

```bash
godmode task done <task-id> --commit <sha>
```

## Guardrails

- Never use `--no-verify`.
- Never force-push without explicit user instruction.
- Never commit to `main` directly — verify with `git branch --show-current` first.
