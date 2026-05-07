use std::io::Write;

use tempfile::TempDir;

use super::*;

fn tmp_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn extract_patterns_finds_glob() {
    let patterns = extract_patterns("show me **/*.toml");
    assert!(
        patterns.contains(&"**/*.toml".to_string()),
        "patterns: {patterns:?}"
    );
}

#[test]
fn extract_patterns_finds_path() {
    let patterns = extract_patterns("edit src/main.rs please");
    assert!(
        patterns.contains(&"src/main.rs".to_string()),
        "patterns: {patterns:?}"
    );
}

#[test]
fn extract_patterns_finds_camel_case_keyword() {
    let patterns = extract_patterns("explain what BackingView does");
    assert!(
        patterns.contains(&"BackingView".to_string()),
        "patterns: {patterns:?}"
    );
}

#[test]
fn extract_patterns_finds_snake_case_keyword() {
    let patterns = extract_patterns("debug the spawn_handon function");
    assert!(
        patterns.contains(&"spawn_handon".to_string()),
        "patterns: {patterns:?}"
    );
}

#[test]
fn estimate_tokens_char_heuristic() {
    let s = "a".repeat(400);
    assert_eq!(estimate_tokens(&s), 100);
}

#[test]
fn estimate_tokens_minimum_one() {
    assert_eq!(estimate_tokens("hi"), 1);
}

#[test]
fn build_jit_context_respects_budget() {
    let dir = TempDir::new().unwrap();
    // Each file: 400 chars → 100 tokens. Budget 250 → fits 2, not 3.
    let content = "x".repeat(400);
    tmp_file(&dir, "alpha.rs", &content);
    tmp_file(&dir, "beta.rs", &content);
    tmp_file(&dir, "gamma.rs", &content);

    let config = JitConfig {
        token_budget: 250,
        ..Default::default()
    };
    let result = build_jit_context(dir.path(), "*.rs", &config);

    assert_eq!(
        result.snippets.len(),
        2,
        "expected exactly 2 files within budget"
    );
    assert!(result.total_tokens <= 250);
}

#[test]
fn build_jit_context_empty_on_no_match() {
    let dir = TempDir::new().unwrap();
    tmp_file(&dir, "foo.rs", "fn main() {}");

    let config = JitConfig::default();
    let result = build_jit_context(dir.path(), "explain docker compose", &config);
    // No patterns should match foo.rs content
    assert_eq!(result.snippets.len(), 0);
}

#[test]
fn build_jit_context_matches_by_path_name() {
    let dir = TempDir::new().unwrap();
    tmp_file(&dir, "jit_context.rs", "pub fn build_jit_context() {}");

    let config = JitConfig::default();
    let result = build_jit_context(dir.path(), "look at jit_context", &config);
    assert_eq!(result.snippets.len(), 1);
    assert!(result.snippets[0].path.contains("jit_context"));
}

#[test]
fn load_config_falls_back_to_default_on_missing_file() {
    let config = load_config(std::path::Path::new("/nonexistent/jit_context.toml"));
    assert_eq!(config.token_budget, 500);
    assert_eq!(config.max_file_bytes, 32 * 1024);
}

#[test]
fn load_config_parses_toml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("jit_context.toml");
    std::fs::write(&path, "token_budget = 800\nmax_file_bytes = 65536\n").unwrap();
    let config = load_config(&path);
    assert_eq!(config.token_budget, 800);
    assert_eq!(config.max_file_bytes, 65536);
}

#[test]
fn find_candidates_excludes_target_dir() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("target")).unwrap();
    tmp_file(&dir, "target/foo.rs", "fn foo() {}");
    tmp_file(&dir, "bar.rs", "fn foo() {}");

    let config = JitConfig::default();
    let candidates = find_candidates(dir.path(), &["foo".to_string()], &config);
    assert!(
        candidates
            .iter()
            .all(|p| !p.to_string_lossy().contains("target"))
    );
}
