//! CLI entrypoint for generating `~/.warp/context.json`.
//!
//! Usage: `cargo run -p warpx --example context_gen`

fn main() {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let state = warpx::context::generate_context(&cwd);
    let json = serde_json::to_string_pretty(&state).expect("serialize context");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let out_path = std::path::PathBuf::from(home).join(".warp/context.json");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&out_path, &json).expect("write context.json");
    eprintln!("wrote {}", out_path.display());
}
