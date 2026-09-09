//! RustaSea CLI — Artisan-like command surface (FS-M5-01..03, FR-500..505).
//!
//! `cargo artisan <command>` is backed by a process-wide command registry
//! that both the clap shim ([`crate::cli`]) and the programmatic
//! [`Artisan::call`] facade read, so `list`, dispatch and in-process
//! invocation never diverge. Long-running workers cooperate with graceful
//! shutdown through [`crate::shutdown`].
//!
//! Feature `cli` gates the interactive/pretty surface (clap, dialoguer,
//! indicatif, comfy-table); the core registry + `Artisan::call` compile
//! without it so test crates can invoke commands with a thin dependency tree.

pub mod artisan;
pub mod error;
pub mod registry;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub mod commands;
#[cfg(feature = "cli")]
pub mod generator;
#[cfg(feature = "cli")]
pub mod generators;
#[cfg(feature = "cli")]
pub mod output;
#[cfg(feature = "cli")]
pub mod prompt;
#[cfg(feature = "cli")]
pub mod shutdown;

pub use artisan::{Artisan, Command, CommandMeta, CommandOutput, Io};
pub use error::{exit_code, CliError, CliResult};
pub use registry::{CommandRegistry, Registered};

/// Re-export for generated commands — app crates depend on the `rustasea`
/// umbrella only, so `make:command` output imports `async_trait` from here.
pub use async_trait;

#[cfg(feature = "cli")]
pub use generator::{Generator, GeneratorError};
#[cfg(feature = "cli")]
pub use shutdown::Shutdownable;

#[cfg(feature = "cli")]
pub use cli::{Cli, CliCommand};

/// Fetch the process-wide command registry (OnceLock singleton).
pub fn registry() -> &'static std::sync::RwLock<registry::CommandRegistry> {
    static REGISTRY: std::sync::OnceLock<std::sync::RwLock<registry::CommandRegistry>> =
        std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| std::sync::RwLock::new(registry::CommandRegistry::new()))
}

/// Register every built-in command into the global registry.
///
/// Invoked by `main` and available to tests that want the full surface
/// without subprocesses (`Artisan::call("migrate", vec![])`).
#[cfg(feature = "cli")]
pub fn load_default_commands() {
    crate::commands::register_all();
}

/// Register a command into the process-wide registry.
pub fn register_command(command: impl Command + 'static) {
    let registry = registry();
    let Ok(mut reg) = registry.write() else {
        panic!("global command registry lock poisoned");
    };
    reg.register(command);
}

/// Convenience: run a built-in command in-process.
///
/// Thin wrapper over [`Artisan::call`] after ensuring default registration.
#[cfg(feature = "cli")]
pub async fn call(signature: &str, args: Vec<String>) -> error::CliResult<artisan::CommandOutput> {
    Artisan::call(signature, args).await
}
