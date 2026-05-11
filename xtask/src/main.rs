//! xtask — warpx workspace dev-tool binary.
//!
//! | Command | Description                                       |
//! |---------|---------------------------------------------------|
//! | `bump`  | Bump warpx crate version (patch/minor/major)      |

use anyhow::{bail, Result};
use std::{env, path::Path};

mod bump;

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let task = env::args().nth(1);

    match task.as_deref() {
        Some("bump") => {
            let level = env::args().nth(2).unwrap_or_else(|| "patch".to_string());
            bump::bump(root, &level)
        }
        Some(other) => bail!("unknown task: {other}"),
        None => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  bump [patch|minor|major]  bump warpx crate version");
            Ok(())
        }
    }
}
