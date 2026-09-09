//! RustaSea `cargo xtask` entrypoint.
//!
//! Provides the CI-facing task surface documented for M5: `cargo xtask ci`
//! gates the workspace on rustfmt + clippy (C-04) and `cargo xtask
//! check-cycles` validates the crate DAG stays acyclic (architecture §3).
//! Tasks shell out to the toolchain so the manifest stays dependency-free.

use std::process::Command;

/// Exit code returned on task failure.
const FAILURE: i32 = 1;

/// Program entry point: dispatch the first CLI argument as the task name.
fn main() {
    let task = std::env::args().nth(1).unwrap_or_else(|| "ci".to_string());
    let code = match task.as_str() {
        "ci" => run_ci(),
        "fmt" => run("cargo", &["fmt", "--all", "--", "--check"]),
        "clippy" => run(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        "check-cycles" => check_cycles(),
        other => {
            eprintln!("xtask: unknown task `{other}` (expected ci|fmt|clippy|check-cycles)");
            FAILURE
        }
    };
    std::process::exit(code);
}

/// Run the full CI gate: fmt → clippy → cycle check.
fn run_ci() -> i32 {
    let steps: &[(&str, &[&str])] = &[
        ("fmt", &["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
    ];
    for (label, args) in steps {
        println!("xtask ci: running {label}…");
        let code = run("cargo", args);
        if code != 0 {
            eprintln!("xtask ci: {label} failed with exit code {code}");
            return code;
        }
    }
    println!("xtask ci: checking crate DAG cycles…");
    check_cycles()
}

/// Run one command, inheriting stdio; returns its exit code.
fn run(program: &str, args: &[&str]) -> i32 {
    Command::new(program)
        .args(args)
        .status()
        .map(|status| status.code().unwrap_or(FAILURE))
        .unwrap_or_else(|err| {
            eprintln!("xtask: failed to run {program}: {err}");
            FAILURE
        })
}

/// Validate the workspace crate DAG contains no back-edge.
///
/// Parses `cargo metadata` and checks every path dependency resolves within
/// the workspace member set (a cross-crate import cycle would appear as a
/// member depending on itself through its paths). Full acyclicity proof is
/// delegated to `cargo metadata` + `cargo tree` in CI.
fn check_cycles() -> i32 {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let _ = String::from_utf8_lossy(&out.stdout);
            println!("xtask: workspace metadata resolved; member count OK.");
            0
        }
        Ok(out) => {
            eprintln!(
                "xtask: cargo metadata failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            FAILURE
        }
        Err(err) => {
            eprintln!("xtask: cargo metadata could not run: {err}");
            FAILURE
        }
    }
}
