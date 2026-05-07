//! JIT context injection — app-layer wrapper.
//!
//! Gated by `FeatureFlag::OzJitContext`. Called from the agent prompt dispatch
//! path before the query is sent to the AI backend.

use std::path::Path;

use crate::features::FeatureFlag;
use warpx::jit_context::{build_jit_context, load_config, JitConfig};

/// Returns a context string to prepend to the agent prompt, or `None` if:
/// - `OzJitContext` flag is disabled
/// - No relevant files found in `cwd`
/// - Token budget is zero after scan
pub fn prepare_jit_context(prompt: &str, cwd: &Path) -> Option<String> {
    if !FeatureFlag::OzJitContext.is_enabled() {
        return None;
    }

    let config_path = dirs::home_dir()?.join(".warp/jit_context.toml");
    let config: JitConfig = load_config(&config_path);

    let result = build_jit_context(cwd, prompt, &config);
    if result.snippets.is_empty() {
        return None;
    }

    let mut context = String::new();
    for snippet in &result.snippets {
        context.push_str(&format!("// {}\n{}\n\n", snippet.path, snippet.content));
    }
    Some(context)
}
