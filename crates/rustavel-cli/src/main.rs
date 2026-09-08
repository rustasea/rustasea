//! `cargo artisan` binary shim.
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
    // Cargo injects the subcommand name (`artisan`) as argv[1] when the
    // binary is invoked via `cargo artisan …`; drop it before clap parses so
    // `bin_name` help rendering stays correct. Direct binary runs are
    // unaffected: they carry no such token.
    let args: Vec<String> = std::env::args().collect();
    let args = if args.get(1).is_some_and(|a| a == "artisan") {
        &args[1..]
    } else {
        &args
    };
    let cli = cli::Cli::parse_from(args);
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
            eprintln!("artisan: failed to start runtime: {err}");
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
                    eprintln!("artisan: missing command name");
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
                eprintln!("artisan: {err}");
                code
            }
        }
    })
}
