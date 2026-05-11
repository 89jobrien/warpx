//! Shared macOS app bundle pipeline for WarpX.
//!
//! Handles: cargo bundle, rpath, plist, bundled resources, icon, codesigning.

use anyhow::{bail, Context, Result};
use command::blocking::Command;
use std::path::{Path, PathBuf};

const BIN_NAME: &str = "warpx";
const CHANNEL: &str = "local";
const APP_NAME: &str = "WarpX.app";
const FEATURES: &str = "gui";

pub struct BundleResult {
    pub app_path: PathBuf,
}

/// Build and prepare a WarpX.app bundle.
pub fn build_bundle(root: &Path, release: bool) -> Result<BundleResult> {
    let profile = if release { "release" } else { "debug" };
    let app_path = root
        .join("target")
        .join(profile)
        .join("bundle/osx")
        .join(APP_NAME);

    // Clean previous bundle
    if app_path.exists() {
        std::fs::remove_dir_all(&app_path)?;
    }

    // cargo bundle
    eprintln!("Bundling WarpX ({profile})...");
    let mut cmd = Command::new("cargo");
    cmd.arg("bundle")
        .arg("--bin")
        .arg(BIN_NAME)
        .arg("--features")
        .arg(FEATURES)
        .current_dir(root.join("app"))
        .env("FRAMEWORK_OVERRIDE", "dev");
    if release {
        cmd.arg("--release");
    }
    run(&mut cmd)?;

    // rpath
    eprintln!("Adding rpath for mac frameworks...");
    run(Command::new("install_name_tool")
        .arg("-add_rpath")
        .arg("@executable_path/../Frameworks")
        .arg(app_path.join("Contents/MacOS").join(BIN_NAME)))?;

    // plist
    run(Command::new(root.join("script/update_plist"))
        .env("WARP_SCHEME_NAME", "warpx")
        .env("WARP_PLIST_PATH", app_path.join("Contents/Info.plist"))
        .current_dir(root))?;

    // bundled resources
    eprintln!("Preparing bundled resources...");
    run(Command::new(root.join("script/prepare_bundled_resources"))
        .arg(app_path.join("Contents/Resources"))
        .arg(CHANNEL)
        .env("SKIP_SETTINGS_SCHEMA", "1")
        .env("NO_LICENSES", "1")
        .env("FRAMEWORK_OVERRIDE", "dev")
        .current_dir(root))?;

    // icon
    run(Command::new(root.join("script/compile_icon"))
        .arg(CHANNEL)
        .arg(&app_path)
        .current_dir(root))?;

    // codesign
    eprintln!("Codesigning...");
    let cert = find_signing_cert();
    let entitlements = if release {
        "script/Entitlements.plist"
    } else {
        "script/Debug-Entitlements.plist"
    };
    run(Command::new("codesign")
        .args(["--force", "--deep", "--options", "runtime", "--sign"])
        .arg(cert.as_deref().unwrap_or("-"))
        .arg(&app_path)
        .arg("--entitlements")
        .arg(root.join(entitlements)))?;

    Ok(BundleResult { app_path })
}

/// Launch the bundled app.
pub fn launch(bundle: &BundleResult, extra_args: &[String]) -> Result<()> {
    eprintln!("Launching {}...", bundle.app_path.display());
    let bin = bundle.app_path.join("Contents/MacOS").join(BIN_NAME);
    let status = Command::new(bin)
        .args(extra_args)
        .status()
        .context("failed to launch WarpX")?;
    if !status.success() {
        bail!("WarpX exited with {status}");
    }
    Ok(())
}

/// Install the bundle to /Applications.
pub fn install(bundle: &BundleResult) -> Result<()> {
    let dest = PathBuf::from("/Applications").join(APP_NAME);
    eprintln!("Installing to {}...", dest.display());
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    copy_dir_all(&bundle.app_path, &dest)?;
    eprintln!("WarpX installed to {}", dest.display());
    Ok(())
}

fn find_signing_cert() -> Option<String> {
    let output = Command::new("security")
        .args(["find-identity", "-p", "codesigning", "-v"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find(|l| l.contains("Apple Development"))
        .and_then(|l| l.split_whitespace().nth(1))
        .map(|s| s.to_string())
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().context("failed to run command")?;
    if !status.success() {
        bail!("command failed with {status}");
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else if ty.is_symlink() {
            let target = std::fs::read_link(entry.path())?;
            std::os::unix::fs::symlink(target, dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
