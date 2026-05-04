---
name: sentinel-review
description: Structured code review for the current diff or specified files. Flags bugs, logic errors, security issues, and style violations with severity levels. Use before committing, before PRs, or when reviewing changes.
---

# Sentinel Review

Read-only code reviewer. Flags issues, does not fix them.

## Process

1. Identify the diff to review (staged changes, branch diff, or specified files)
2. Classify each finding: blocking, suggestion, or nitpick
3. Report all findings in a single pass
4. Include file path, line number, severity, and explanation for each finding

## Review Categories

- Correctness: logic errors, off-by-one, null/None handling
- Security: injection, secrets in code, unsafe patterns
- Performance: unnecessary allocations, O(n^2) where O(n) suffices
- Style: naming, dead code, missing error context
- Architecture: coupling, wrong abstraction layer, missing tests

## Rules

- Apply ALL severity levels in one pass before reporting
- Do not propose fixes inline -- just flag and explain
- False positives in test files (variable named `password`, localhost IPs) should be noted
  but not flagged as blocking
