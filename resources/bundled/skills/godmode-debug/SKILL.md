---
name: godmode-debug
description: Systematic debugging for Rust. Use before proposing any fix — establish root cause first, then write a failing test, then fix.
---

# Systematic Debugging

**Rule zero**: No fix without a root cause. Symptom patches that mask underlying problems are
not fixes.

## Phase 1: Root Cause Investigation

Before touching any code:

1. **Parse the error exactly.** Read the full message, stack trace, and context. Do not skim.

2. **Reproduce it** consistently:

   ```bash
   RUST_BACKTRACE=1 cargo nextest run -p <crate> -- <test_name>
   ```

3. **Check environment first.** Missing env vars and unresolved `op://` refs are the most
   common root cause. Verify before investigating code.

4. **Check recent changes.**

   ```bash
   git diff HEAD~1
   git log --oneline -10
   ```

5. **For multi-component failures**: add diagnostic output at each boundary. Trace data flow
   backward from the symptom.

## Phase 2: Pattern Analysis

- Find a working analog in the codebase — similar code that does work.
- Compare working vs. broken line by line.
- Document every difference.

## Phase 3: Hypothesis and Testing

- State one specific hypothesis: "The bug is X because Y."
- Test with a minimal, isolated change.
- If wrong, discard entirely and form a new one. Do not layer fixes on failed hypotheses.

## Phase 4: Fix

Only after root cause is confirmed:

1. Write a failing test that captures the bug.
2. Implement the single fix.
3. Verify:
   ```bash
   cargo nextest run -p <crate>
   cargo clippy -p <crate> -- -D warnings
   ```

## Rust Checklist

- `RUST_BACKTRACE=full` for panics; `RUST_LOG=debug` for tracing
- `cargo check` before `cargo nextest` — catch compile errors cheaply
- Lifetime/borrow errors: read the full compiler message, not just the first line
- Async failures: check executor context (tokio runtime not entered, etc.)

## 3-Failure Rule

If 3 sequential fix attempts all fail, stop. Surface the deeper problem to the user.

## Never

- Apply a fix before identifying root cause
- Stack multiple fixes in one commit
- Suppress compiler warnings to make a test pass
- Use `unwrap()` to "fix" an error — propagate it properly
