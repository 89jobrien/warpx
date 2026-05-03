---
name: godmode-brainstorm
description: Design-first workflow for new features, components, or architecture decisions. Use before writing any code — get explicit approval on a design first.
---

# Brainstorm

**Hard gate**: Do NOT write any code, scaffold any structure, or invoke any implementation
skill until you have presented a design and the user has explicitly approved it.

## Process

### Step 1: Explore context

Before asking anything, read:

- The relevant `CLAUDE.md` for the project
- The crate's `Cargo.toml` and entry points
- Any existing analogous code

### Step 2: Ask one clarifying question at a time

Never dump a list of questions. One question → user answers → next question.
Prefer multiple-choice over open-ended where possible.

### Step 3: Propose 2-3 approaches

For each approach:

- Name it
- Describe it in 2-3 sentences
- State the key trade-off vs. the alternatives

### Step 4: Present design sections iteratively

Break the design into sections. Present one section → get feedback → refine → next section.
Do not present the entire design at once.

### Step 5: Write a design document

Once approved, write to `docs/plans/YYYY-MM-DD-<feature>.md`. Include:

- Goal
- Architecture (crates affected, new traits/types, data flow)
- Tech decisions made and why
- What is explicitly out of scope

### Step 6: Write an implementation plan

The only permitted next step after brainstorming is writing an implementation plan.

## Design Principles

- **Hexagonal architecture**: external deps behind a trait (port), implementations in adapters.
- **YAGNI**: remove anything not needed for the current requirement.
- **Rust idioms**: `Result`/`Option` over panics, `impl Trait` over `Box<dyn Trait>` where
  lifetimes allow, `#[derive]` over manual impls.
- **One question at a time**: overwhelming the user with 5 questions at once is a failure mode.

## Rust Design Questions

- Which crate owns this? Existing crate or new one?
- Does this introduce a new external dependency? Can it be behind a feature flag?
- Tested with in-memory fakes or integration tests?
- Does this change the public API surface? Semver implications?
