#!/usr/bin/env rust-script
//! Local macOS CI runner — mirrors .ctx/ci_macos.yml
//!
//! Usage: rust-script scripts/ci-macos.rs

use std::process::{Command, ExitCode};

fn run(label: &str, args: &[&str], shell_path: Option<&str>) -> bool {
    println!("\n==> {label}");
    let mut c = Command::new("cargo");
    c.args(args)
        .env("CARGO_TERM_COLOR", "always")
        .env("NEXTEST_PROFILE", "ci")
        .env("RUSTFLAGS", "-C debuginfo=line-tables-only --cfg=web_sys_unstable_apis");
    if let Some(p) = shell_path {
        c.env("WARP_SHELL_PATH", p);
    }
    let status = c.status().expect("failed to spawn cargo");
    if !status.success() {
        eprintln!("FAILED: {label} (exit {})", status.code().unwrap_or(-1));
    }
    status.success()
}

fn which(cmd: &str) -> String {
    String::from_utf8(
        Command::new("bash")
            .args(["-c", cmd])
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default(),
    )
    .unwrap_or_default()
    .trim()
    .to_string()
}

fn main() -> ExitCode {
    const W: &[&str] = &["--workspace", "--locked", "--exclude", "command-signatures-v2"];

    let bash = which("command -pv bash");
    let zsh  = which("which zsh 2>/dev/null || echo /bin/zsh");
    let fish = which("which fish 2>/dev/null");

    // Ensure cargo-nextest installed
    if !Command::new("cargo-nextest").arg("--version").status().map(|s| s.success()).unwrap_or(false) {
        Command::new("cargo").args(["install", "cargo-nextest", "--locked"]).status().ok();
    }

    macro_rules! nextest {
        ($($e:expr),*) => {{
            let mut v = vec!["nextest", "run"];
            v.extend_from_slice(W);
            $(v.push($e);)*
            v
        }};
    }

    // Compile — hard gate
    if !run("Compile tests", &{ let mut a = nextest!(); a.push("--no-run"); a }, None) {
        eprintln!("Compile step failed — aborting.");
        return ExitCode::FAILURE;
    }

    let mut ok = true;

    ok &= run("Unit tests",
        &nextest!("-E", "not package(integration)"),
        Some(&zsh));

    ok &= run("Shell-agnostic integration tests",
        &nextest!("-E", "package(integration) and not test(shell_integration_tests) and not test(/_ssh_/)"),
        Some(&zsh));

    ok &= run("Shell integration tests (bash)",
        &nextest!("-E", "package(integration) and test(shell_integration_tests)"),
        Some(&bash));

    ok &= run("Shell integration tests (zsh)",
        &nextest!("-E", "package(integration) and test(shell_integration_tests)"),
        Some(&zsh));

    if !fish.is_empty() {
        ok &= run("Shell integration tests (fish)",
            &nextest!("-E", "package(integration) and test(shell_integration_tests)"),
            Some(&fish));
    }

    // Doc tests (cargo test, not nextest)
    {
        let mut args = vec!["test"];
        args.extend_from_slice(W);
        args.push("--doc");
        ok &= run("Doc tests", &args, None);
    }

    ok &= run("Completions-on-js tests",
        &["nextest", "run", "--locked", "-p", "warp_completer", "--features", "v2"],
        None);

    if ok {
        println!("\nAll jobs completed successfully!");
        ExitCode::SUCCESS
    } else {
        eprintln!("\nOne or more jobs failed.");
        ExitCode::FAILURE
    }
}
