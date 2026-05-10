//! warpx — personal fork extensions for warpx.
//!
//! Pure data and IO layer. No warpui dependency.
//! WarpUI views live in `app/src/joe/`.

pub mod claude_projects;
pub mod context;
pub mod doob;
pub mod handoff;
pub mod hooks;
pub mod jit_context;
pub mod joe;

#[cfg(test)]
pub(crate) mod test_env {
    use std::sync::Mutex;

    pub static ENV_LOCK: Mutex<()> = Mutex::new(());
}
