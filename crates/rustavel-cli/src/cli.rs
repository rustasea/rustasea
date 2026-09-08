//! Clap derive surface: `cargo rustavel <command> [args] [--json]`.
//!
//! The command name is positional; all remaining arguments are forwarded
//! verbatim to the registered command's `run(args)`, which keeps the clap
//! layer a thin shim over the process-wide [`crate::Artisan`] registry —
//! `list`/`Artisan::call` never diverge from what `main` dispatches.

use clap::{Parser, Subcommand};

/// Rustavel command-line interface (Artisan-parity DX).
#[derive(Debug, Parser)]
#[command(
    name = "rustavel",
    about = "Rustavel framework console",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Command to execute, e.g. `make:controller`.
    #[command(subcommand)]
    pub command: CliCommand,

    /// Forwarded verbatim to the selected command.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
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
    /// Invoke a registered command by name with raw arguments.
    #[command(external_subcommand)]
    Run(Vec<String>),
}
