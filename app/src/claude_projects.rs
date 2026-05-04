// Thin re-export — all logic now lives in the warpx crate.
pub use warpx::claude_projects::read_project_entries;
#[allow(unused_imports)]
pub use warpx::claude_projects::{claude_projects_dir, read_entries_from_dir, ProjectEntry};
