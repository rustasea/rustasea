//! `cargo rustasea` — the RustaSea application scaffolder binary.
//!
//! Cargo discovers this binary through the `cargo-rustasea` name and invokes it
//! as `cargo rustasea <args>`, passing the literal `rustasea` token as the first
//! argument. [`normalize_args`] strips that token — and the `artisan` alias —
//! before clap parses, so all of these forms work:
//!
//! ```text
//! cargo rustasea new my-app --variant blade
//! cargo artisan new my-app --variant blade   # legacy alias
//! cargo-rustasea new my-app --variant blade  # direct binary invocation
//! ```
//!
//! Process concerns live here; all template logic lives in `rustasea-scaffold`.

use std::path::Path;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use rustasea_scaffold::{resolve_target, Scaffold, StarterKitVariant};

/// Top-level cargo subcommand surface.
#[derive(Debug, Parser)]
#[command(
    name = "cargo-rustasea",
    bin_name = "cargo rustasea",
    version,
    about = "RustaSea application scaffolder",
    propagate_version = true
)]
struct Cli {
    /// Selected subcommand.
    #[command(subcommand)]
    command: Command,
}

/// Supported subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new RustaSea application.
    New(NewArgs),
}

/// Arguments for `cargo rustasea new`.
#[derive(Debug, Args)]
struct NewArgs {
    /// Application name or target directory (for example `my-app`).
    app: String,

    /// Presentation variant to scaffold.
    #[arg(short, long, value_enum)]
    variant: Variant,

    /// Overwrite existing files instead of failing.
    #[arg(long)]
    force: bool,

    /// Skip `git init` in the generated application directory.
    #[arg(long)]
    no_git: bool,
}

/// CLI-facing mirror of [`StarterKitVariant`] so clap can validate the value.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Variant {
    /// Server-rendered askama views.
    Blade,
    /// Dioxus WASM + Inertia.
    React,
    /// Leptos WASM + Inertia.
    Vue,
    /// askama + HTMX + broadcast.
    Livewire,
}

impl From<Variant> for StarterKitVariant {
    /// Map the clap enum onto the library variant.
    fn from(value: Variant) -> Self {
        match value {
            Variant::Blade => StarterKitVariant::Blade,
            Variant::React => StarterKitVariant::React,
            Variant::Vue => StarterKitVariant::Vue,
            Variant::Livewire => StarterKitVariant::Livewire,
        }
    }
}

/// Program entry point.
fn main() -> ExitCode {
    let args = normalize_args(std::env::args().collect());
    let cli = Cli::parse_from(args);
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Drop cargo's injected subcommand token (`rustasea`) or the legacy `artisan`
/// alias so both `cargo rustasea new` and `cargo artisan new` parse identically.
fn normalize_args(mut args: Vec<String>) -> Vec<String> {
    if args
        .get(1)
        .is_some_and(|token| token == "rustasea" || token == "artisan")
    {
        args.remove(1);
    }
    args
}

/// Dispatch the parsed command.
fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::New(args) => new_app(args),
    }
}

/// Scaffold a new application from parsed arguments.
fn new_app(args: NewArgs) -> Result<(), String> {
    let (target, app_name) = resolve_target(&args.app).map_err(|err| err.to_string())?;
    let scaffold = Scaffold::new(app_name, args.variant.into()).with_force(args.force);
    let generated = scaffold.generate(&target).map_err(|err| err.to_string())?;

    println!("Created {} files in {}", generated.len(), target.display());

    if !args.no_git {
        init_git(&target);
    }
    Ok(())
}

/// Best-effort `git init`; a missing git binary is a warning, not a failure.
fn init_git(target: &Path) {
    let status = std::process::Command::new("git")
        .arg("init")
        .arg("--quiet")
        .current_dir(target)
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("warning: `git init` exited with {status}"),
        Err(error) => eprintln!("warning: could not run `git init`: {error}"),
    }
}
