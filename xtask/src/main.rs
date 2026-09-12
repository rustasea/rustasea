//! RustaSea `cargo xtask` entrypoint.
//!
//! Provides the CI-facing task surface documented for M5: `cargo xtask ci`
//! gates the workspace on rustfmt + clippy (C-04), `cargo xtask check-cycles`
//! validates the crate DAG stays acyclic (architecture §3), and `cargo xtask
//! migrate` runs the framework's registered migrations. Toolchain tasks shell
//! out to `cargo`; graph analysis and migration execution live in submodules.

mod cycles;
mod migrate;

use std::process::Command;

/// Exit code returned on task failure.
pub(crate) const FAILURE: i32 = 1;

/// Program entry point: dispatch the first CLI argument as the task name.
fn main() {
    let mut args = std::env::args().skip(1);
    let task = args.next().unwrap_or_else(|| "ci".to_string());
    let rest: Vec<String> = args.collect();
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
        "check-cycles" => cycles::run(),
        "migrate" => migrate::run(&rest),
        other => {
            eprintln!(
                "xtask: unknown task `{other}` (expected ci|fmt|clippy|check-cycles|migrate)"
            );
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
    cycles::run()
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
