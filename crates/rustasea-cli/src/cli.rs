//! Clap derive surface: `cargo artisan <command> [args] [--json]`.
//!
//! The command name is positional; all remaining arguments are forwarded
//! verbatim to the registered command's `run(args)`, which keeps the clap
//! layer a thin shim over the process-wide [`crate::Artisan`] registry —
//! `list`/`Artisan::call` never diverge from what `main` dispatches.

use clap::{Parser, Subcommand};

/// RustaSea command-line interface (Artisan-parity DX).
#[derive(Debug, Parser)]
#[command(
    name = "cargo-artisan",
    bin_name = "cargo artisan",
    about = "RustaSea framework console (Artisan-parity CLI)",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Command to execute, e.g. `make:controller`.
    #[command(subcommand)]
    pub command: CliCommand,
}

/// Top-level subcommands resolved by clap.
#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// List available commands (`--json` machine-readable, `--all` includes hidden).
    List {
        /// Emit JSON instead of a human table.
        #[arg(long)]
        json: bool,
        /// Include hidden commands.
        #[arg(long)]
        all: bool,
    },
    /// Invoke a registered command by name with raw arguments
    /// (e.g. `make:controller UserController --resource`).
    #[command(external_subcommand)]
    Run(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The named `list` subcommand parses with its flags.
    #[test]
    fn list_parses_with_flags() {
        let cli = Cli::parse_from(["cargo artisan", "list", "--json", "--all"]);
        match cli.command {
            CliCommand::List { json, all } => {
                assert!(json);
                assert!(all);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    /// Unknown commands fall through to the external subcommand with raw args,
    /// including hyphen-prefixed flags (e.g. `--force`).
    #[test]
    fn external_subcommand_captures_raw_args() {
        let cli = Cli::parse_from([
            "cargo artisan",
            "make:controller",
            "UserController",
            "--resource",
            "--force",
        ]);
        match cli.command {
            CliCommand::Run(args) => {
                assert_eq!(
                    args,
                    vec!["make:controller", "UserController", "--resource", "--force"]
                );
            }
            other => panic!("expected Run, got {other:?}"),
        }
    }
}
