//! Rustavel CLI — typed errors for commands, generators and the Artisan API.
//!
//! Every failure the CLI surfaces is a typed [`CliError`] with a stable
//! `code`, so the `main` shim can map it onto a process exit code and the
//! `list --json` contract stays machine-readable (FR-500..505).

use std::path::{Path, PathBuf};

/// CLI error kind — printed as `{code}: {message}` by the bin shim.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// The requested command is not registered.
    #[error("unknown command `{0}`")]
    UnknownCommand(String),

    /// The requested command is registered but hidden and not runnable
    /// directly (enumeration only via `list --all`).
    #[error("command `{0}` is hidden")]
    HiddenCommand(String),

    /// Command argument parsing failed inside `Artisan::call`.
    #[error("invalid arguments for `{command}`: {detail}")]
    InvalidArguments {
        /// Command signature.
        command: String,
        /// Parser detail.
        detail: String,
    },

    /// A generator refused to overwrite an existing file without `--force`.
    #[error("{path} already exists. Pass --force to overwrite.")]
    AlreadyExists {
        /// Existing output path.
        path: String,
    },

    /// A generator cannot derive the output path (no Cargo manifest).
    #[error("not a Rustavel project: {detail}")]
    NotAProject {
        /// Missing artifact detail.
        detail: String,
    },

    /// A generator produced invalid scaffolding.
    #[error("generation failed for {kind} `{name}`: {detail}")]
    GenerationFailed {
        /// Generator kind.
        kind: String,
        /// Scaffold name.
        name: String,
        /// Failure detail.
        detail: String,
    },

    /// The workspace filesystem returned an unexpected error.
    #[error("io error: {0}")]
    Io(std::io::Error),

    /// JSON serialization of `list --json` failed.
    #[error("json serialization failed: {0}")]
    Json(serde_json::Error),

    /// A graceful-shutdown drain exceeded the configured timeout.
    #[error("shutdown drain timed out after {0:?}")]
    ShutdownTimeout(std::time::Duration),

    /// A schedule/queue operation surfaced a downstream error.
    #[error("{0}")]
    Domain(String),
}

/// Convenience alias for CLI results.
pub type CliResult<T> = std::result::Result<T, CliError>;

impl From<std::io::Error> for CliError {
    /// Convert an io error into the typed CLI error.
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

impl From<serde_json::Error> for CliError {
    /// Convert a JSON error into the typed CLI error.
    fn from(e: serde_json::Error) -> Self {
        CliError::Json(e)
    }
}

/// Canonical exit-code mapping for CLI errors (contract `cli.errors`).
pub fn exit_code(err: &CliError) -> i32 {
    match err {
        CliError::UnknownCommand(_) | CliError::HiddenCommand(_) => 2,
        CliError::InvalidArguments { .. } => 3,
        CliError::AlreadyExists { .. } => 4,
        CliError::ShutdownTimeout(_) => 5,
        CliError::NotAProject { .. }
        | CliError::GenerationFailed { .. }
        | CliError::Io(_)
        | CliError::Json(_)
        | CliError::Domain(_) => 1,
    }
}

/// Locate the project root by walking up from `start`.
///
/// The root is the nearest ancestor containing a `Cargo.toml`. All generated
/// files are written relative to it (contract: generators only write inside a
/// Rustavel workspace/app).
pub fn project_root(start: &Path) -> CliResult<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join("Cargo.toml").is_file() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Err(CliError::NotAProject {
        detail: format!("no Cargo.toml found above {}", start.display()),
    })
}
