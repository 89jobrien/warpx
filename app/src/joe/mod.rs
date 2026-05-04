//! JoeMode: personal ecosystem integration.
//!
//! All fork-specific modules live here. Gated behind FeatureFlag::JoeMode
//! and related Oz flags at call sites.

pub mod claude_projects;
pub mod doob;
pub mod handoff;
pub mod hooks;
pub mod project_context;

// Re-export the pure data types from warpx for use within app.
#[allow(unused_imports)]
pub use warpx::joe::{
    read_doob_cache, read_godmode_state, refresh, DoobStatusCache, GodmodeState, JoeStatus,
};
