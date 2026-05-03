---
name: godmode-tdd
description: Test-driven development workflow for Rust. Use before writing any implementation code — write a failing test first, then implement, then refactor.
---

# Test-Driven Development

**Iron Law**: No production code without a prior failing test. Delete and restart if you wrote
code before the test.

## Red-Green-Refactor Cycle

### 1. RED — Write a failing test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thing_does_x() {
        // Arrange / Act / Assert — must FAIL before implementation
    }
}
```

Run and confirm failure:

```bash
cargo nextest run -p <crate> -- <test_name>
```

Verify failure is for the right reason — not a compile error. The test must exist, compile,
and fail because the behavior isn't implemented.

### 2. GREEN — Minimum code to pass

Write the least code that makes the test pass. No extras.

```bash
cargo nextest run -p <crate>   # all tests must pass
```

### 3. REFACTOR — Clean up with tests green

```bash
cargo clippy -p <crate> -- -D warnings
cargo fmt -p <crate>
cargo nextest run -p <crate>   # still green
git commit -m "feat(<crate>): <what it does>"
```

## Rules

- Tests in `#[cfg(test)]` modules or `tests/` for integration tests.
- Use in-memory fakes over mocked HTTP/DB in unit tests.
- Domain code has zero imports from infrastructure crates.
- New external deps go behind a trait (port) — implementations in adapters.

## 3-Attempt Rule

If a test is still failing after 3 attempts: either the architecture is wrong (step back and
redesign) or the requirement is unclear (ask the user). Never brute-force past 3 failures.
