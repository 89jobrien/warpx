//! xtask — warpx workspace dev-tool binary.
//!
//! | Command   | Description                                       |
//! |-----------|---------------------------------------------------|
//! | `bump`    | Bump warpx crate version (patch/minor/major)      |
//! | `run`     | Build and launch WarpX.app locally                 |
//! | `install` | Build and install WarpX.app to /Applications       |

use anyhow::{bail, Result};
use std::{env, path::Path};

mod bump;
mod bundle;

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let args: Vec<String> = env::args().collect();
    let task = args.get(1).map(|s| s.as_str());

    match task {
        Some("bump") => {
            let level = args.get(2).map(|s| s.as_str()).unwrap_or("patch");
            bump::bump(root, level)
        }
        Some("run") => {
            let release = args.iter().any(|a| a == "--release");
            let extra: Vec<String> = args
                .iter()
                .position(|a| a == "--")
                .map(|i| args[i + 1..].to_vec())
                .unwrap_or_default();
            let result = bundle::build_bundle(root, release)?;
            bundle::launch(&result, &extra)
        }
        Some("install") => {
            let release = args.iter().any(|a| a == "--release");
            let result = bundle::build_bundle(root, release)?;
            bundle::install(&result)
        }
        Some(other) => bail!("unknown task: {other}"),
        None => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  bump [patch|minor|major]    bump warpx crate version");
            eprintln!("  run [--release] [-- args]   build and launch WarpX.app");
            eprintln!("  install [--release]          build and install to /Applications");
            Ok(())
        }
    }
}
