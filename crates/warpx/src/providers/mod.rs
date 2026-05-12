//! Concrete [`ContextProvider`](crate::provider::ContextProvider) implementations.
//!
//! Each module exports a single struct implementing the provider trait.
//! The [`all_providers`] function returns the default set in display order.

pub mod ai_context;
pub mod cargo_state;
pub mod ci;
pub mod env_health;
pub mod git;
pub mod handoff;
pub mod todos;
pub mod worktrees;

use std::path::Path;

use crate::provider::ContextProvider;

/// Returns all built-in providers in the order they should appear in the panel.
pub fn all_providers(cwd: &Path) -> Vec<Box<dyn ContextProvider>> {
    vec![
        Box::new(git::GitProvider::new(cwd)),
        Box::new(ai_context::AiContextProvider),
        Box::new(handoff::HandoffProvider),
        Box::new(todos::TodosProvider),
        Box::new(ci::CiProvider),
        Box::new(cargo_state::CargoStateProvider),
        Box::new(worktrees::WorktreesProvider),
        Box::new(env_health::EnvHealthProvider),
    ]
}
