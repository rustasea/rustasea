//! `cargo rustavel` binary shim.
//!
//! Loads the built-in command registry and forwards clap-parsed arguments to
//! the selected command — or runs `list` natively when that subcommand is
//! used. Errors map onto stable exit codes (see [`rustavel_cli::exit_code`]).
//!
//! The binary runs on a multi-threaded tokio runtime so async built-ins
//! (`queue:retry`, `schedule:run`, …) can await the M4 facades directly.

use clap::Parser;
use rustavel_cli::{cli, load_default_commands, CliCommand, Command};

/// Program entry point.
fn main() {
    let cli = cli::Cli::parse();
    load_default_commands();
    std::process::exit(entrypoint(cli));
}

/// Async entrypoint run under a tokio runtime; returns the process exit code.
fn entrypoint(cli: cli::Cli) -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("rustavel: failed to start runtime: {err}");
            return 1;
        }
    };
    runtime.block_on(async move {
        let result = match cli.command {
            CliCommand::List { json, all } => {
                let mut io = rustavel_cli::CommandOutput::default();
                let mut args = Vec::new();
                if json {
                    args.push("--json".to_string());
                }
                if all {
                    args.push("--all".to_string());
                }
                // The list command reads --json/--all from its raw args.
                let outcome = rustavel_cli::commands::builtins::List
                    .run(args, &mut io)
                    .await;
                outcome.map(|()| io)
            }
            CliCommand::Run(args) => {
                // External subcommand: first arg is the signature, rest are raw args.
                let Some((signature, forwarded)) = args.split_first() else {
                    eprintln!("rustavel: missing command name");
                    return 2;
                };
                let forwarded = forwarded.to_vec();
                rustavel_cli::Artisan::call(signature, forwarded).await
            }
        };

        match result {
            Ok(output) => {
                print!("{}", output.stdout);
                if !output.stderr.is_empty() {
                    eprint!("{}", output.stderr);
                }
                output.exit_code
            }
            Err(err) => {
                let code = rustavel_cli::exit_code(&err);
                eprintln!("rustavel: {err}");
                code
            }
        }
    })
}
