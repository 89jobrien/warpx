//! JIT context injection — scans cwd for files matching patterns extracted from the
//! user's agent prompt and returns token-budgeted snippets to prepend as context.
//!
//! All operations are synchronous but fast: max depth 3, max 20 candidate files.
//! App wrapper with feature-flag gating lives in `app/src/joe/jit_context.rs`.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Runtime configuration for JIT context injection.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct JitConfig {
    /// Max tokens to inject across all files. Default: 500.
    pub token_budget: usize,
    /// Max file size in bytes to read. Default: 32 KiB.
    pub max_file_bytes: usize,
    /// Additional glob-style path segments to exclude (matched against each path component).
    pub exclude: Vec<String>,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            token_budget: 500,
            max_file_bytes: 32 * 1024,
            exclude: Vec::new(),
        }
    }
}

/// Load config from `path`. Falls back to `JitConfig::default()` on any error.
pub fn load_config(path: &Path) -> JitConfig {
    let Ok(text) = std::fs::read_to_string(path) else {
        return JitConfig::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single file snippet selected for injection.
#[derive(Debug, Clone)]
pub struct JitSnippet {
    pub path: String,
    pub content: String,
    pub tokens: usize,
}

/// Result of a JIT context scan.
#[derive(Debug, Default)]
pub struct JitResult {
    pub snippets: Vec<JitSnippet>,
    pub total_tokens: usize,
}

// ---------------------------------------------------------------------------
// Pattern extraction
// ---------------------------------------------------------------------------

/// Extract candidate file patterns from a user prompt.
///
/// Returns three kinds of patterns:
/// - Glob tokens: any whitespace-delimited word containing `*`, `?`, or `{`/`}`
/// - Explicit paths: tokens that look like file paths (`foo/bar.rs`, `./src/lib.rs`)
/// - Keywords: CamelCase or snake_case identifiers longer than 4 chars
pub fn extract_patterns(prompt: &str) -> Vec<String> {
    let mut patterns: Vec<String> = Vec::new();

    for word in prompt.split_whitespace() {
        // Strip common punctuation wrappers
        let w = word.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ',' | '.' | ':'));
        if w.is_empty() {
            continue;
        }

        // Glob token
        if w.contains('*') || w.contains('?') || w.contains('{') {
            patterns.push(w.to_string());
            continue;
        }

        // Explicit path: contains a `/` or starts with `./` or ends with a dotted extension
        if w.contains('/') || looks_like_file(w) {
            patterns.push(w.to_string());
            continue;
        }

        // Keyword: CamelCase (has uppercase after lowercase) or snake_case with underscore
        if w.len() > 4
            && (is_camel_case(w)
                || (w.contains('_') && w.chars().all(|c| c.is_alphanumeric() || c == '_')))
        {
            patterns.push(w.to_string());
        }
    }

    patterns.dedup();
    patterns
}

fn looks_like_file(s: &str) -> bool {
    // Has an extension: last component after `.` is 1–6 alphanumeric chars
    if let Some(dot) = s.rfind('.') {
        let ext = &s[dot + 1..];
        return !ext.is_empty() && ext.len() <= 6 && ext.chars().all(|c| c.is_alphanumeric());
    }
    false
}

fn is_camel_case(s: &str) -> bool {
    let mut saw_lower = false;
    for c in s.chars() {
        if c.is_uppercase() && saw_lower {
            return true;
        }
        if c.is_lowercase() {
            saw_lower = true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Rough token estimate: characters / 4. No external dependency.
pub fn estimate_tokens(content: &str) -> usize {
    (content.len() / 4).max(1)
}

// ---------------------------------------------------------------------------
// Candidate search
// ---------------------------------------------------------------------------

/// Default directory names to always exclude from file walking.
const ALWAYS_EXCLUDE: &[&str] = &[".git", "target", "node_modules", ".ctx", ".build", "dist"];

/// Walk `cwd` up to `depth` levels deep and return files whose path (or first
/// 512 bytes of content) contain any of the given patterns.
/// Hard cap: 20 files returned.
pub fn find_candidates(cwd: &Path, patterns: &[String], config: &JitConfig) -> Vec<PathBuf> {
    if patterns.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    walk_dir(cwd, patterns, config, 0, 3, &mut results);
    results.truncate(20);
    results
}

fn walk_dir(
    dir: &Path,
    patterns: &[String],
    config: &JitConfig,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
) {
    if out.len() >= 20 || depth > max_depth {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        if out.len() >= 20 {
            break;
        }
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip hidden dirs and always-excluded dirs
        if name.starts_with('.') && path.is_dir() {
            continue;
        }
        if ALWAYS_EXCLUDE.contains(&name) {
            continue;
        }
        // Skip user-configured exclusions (simple substring match on path component)
        if config.exclude.iter().any(|ex| name.contains(ex.as_str())) {
            continue;
        }

        if path.is_dir() {
            walk_dir(&path, patterns, config, depth + 1, max_depth, out);
        } else if path.is_file() && file_matches(&path, patterns, config) {
            out.push(path);
        }
    }
}

fn file_matches(path: &Path, patterns: &[String], _config: &JitConfig) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    for p in patterns {
        let p_lower = p.to_lowercase();
        // Match against full path string
        if path_str.contains(p_lower.as_str()) {
            return true;
        }
        // Match against first 512 bytes of file content
        if let Ok(mut f) = std::fs::File::open(path) {
            use std::io::Read;
            let mut buf = [0u8; 512];
            if let Ok(n) = f.read(&mut buf) {
                let preview = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                if preview.contains(p_lower.as_str()) {
                    return true;
                }
            }
        }
        // Glob-style: if pattern ends with extension, check file extension
        if let Some(ext) = p.strip_prefix("*.")
            && path.extension().and_then(|e| e.to_str()) == Some(ext)
        {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Scan `cwd` for files relevant to `prompt` and return token-budgeted snippets.
pub fn build_jit_context(cwd: &Path, prompt: &str, config: &JitConfig) -> JitResult {
    let patterns = extract_patterns(prompt);
    if patterns.is_empty() {
        return JitResult::default();
    }

    let candidates = find_candidates(cwd, &patterns, config);
    let mut result = JitResult::default();

    for path in candidates {
        if result.total_tokens >= config.token_budget {
            break;
        }

        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if meta.len() as usize > config.max_file_bytes {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let tokens = estimate_tokens(&content);

        if result.total_tokens + tokens > config.token_budget {
            continue;
        }

        let path_str = path
            .strip_prefix(cwd)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        result.total_tokens += tokens;
        result.snippets.push(JitSnippet {
            path: path_str,
            content,
            tokens,
        });
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "jit_context_tests.rs"]
mod tests;
