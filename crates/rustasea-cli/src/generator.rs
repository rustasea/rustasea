//! Generator core — file writing, naming, and typed errors.
//!
//! Every `make:*` generator resolves an output path relative to the project
//! root, refuses to overwrite without `--force` ([`GeneratorError::AlreadyExists`]),
//! and returns the written path for the CLI's confirmation line (FR-501).

use std::path::{Path, PathBuf};

use crate::error::CliResult;

/// Errors produced by scaffolding operations.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// The output file already exists and `--force` was not passed.
    #[error("{path} already exists. Pass --force to overwrite.")]
    AlreadyExists {
        /// Existing output path.
        path: String,
    },

    /// The scaffold name is not a valid Rust identifier.
    #[error("`{name}` is not a valid {kind} name: expected PascalCase alphanumeric identifier")]
    InvalidName {
        /// Generator kind.
        kind: String,
        /// Supplied name.
        name: String,
    },

    /// The filesystem rejected the write.
    #[error("failed to write {path}: {source}")]
    Write {
        /// Target path.
        path: String,
        /// Underlying io error.
        source: std::io::Error,
    },

    /// Parent directory creation failed.
    #[error("failed to create directory for {path}: {source}")]
    Mkdir {
        /// Target path.
        path: String,
        /// Underlying io error.
        source: std::io::Error,
    },
}

impl From<GeneratorError> for crate::CliError {
    /// Lift a generator error into the CLI error surface.
    fn from(e: GeneratorError) -> Self {
        match e {
            GeneratorError::AlreadyExists { path } => crate::CliError::AlreadyExists { path },
            GeneratorError::InvalidName { kind, name } => crate::CliError::GenerationFailed {
                kind,
                name,
                detail: "name is not a valid PascalCase identifier".into(),
            },
            GeneratorError::Write { path, source } => crate::CliError::GenerationFailed {
                kind: "file".into(),
                name: path,
                detail: source.to_string(),
            },
            GeneratorError::Mkdir { path, source } => crate::CliError::GenerationFailed {
                kind: "directory".into(),
                name: path,
                detail: source.to_string(),
            },
        }
    }
}

/// A rendered file awaiting persistence.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Workspace-relative output path (`app/models/post.rs`).
    pub relative_path: String,
    /// File contents (always rustfmt-clean by construction).
    pub source: String,
}

/// Result of one successful generation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Generated {
    /// Written path, e.g. `app/http/controllers/user_controller.rs`.
    pub path: String,
    /// Whether an existing file was overwritten (`--force`).
    pub overwritten: bool,
}

/// Common scaffolding behaviour shared by every generator.
pub struct Generator;

impl Generator {
    /// Validate a PascalCase Rust type name.
    ///
    /// Accepts ASCII alphanumeric identifiers starting with an uppercase
    /// letter; anything else is rejected before any file is touched.
    pub fn validate_name(kind: &str, name: &str) -> Result<(), GeneratorError> {
        let mut chars = name.chars();
        let valid = matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
            && chars.all(|c| c.is_ascii_alphanumeric());
        if valid && !name.is_empty() {
            Ok(())
        } else {
            Err(GeneratorError::InvalidName {
                kind: kind.to_string(),
                name: name.to_string(),
            })
        }
    }

    /// Convert a PascalCase name to snake_case (`UserController` → `user_controller`).
    pub fn snake(name: &str) -> String {
        let mut out = String::with_capacity(name.len() + 4);
        for (idx, ch) in name.chars().enumerate() {
            if ch.is_ascii_uppercase() {
                if idx > 0 {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// Convert a PascalCase name to a plural snake table name
    /// (`Post` → `posts`, `Person` → `people` is out of scope — naive `s`).
    pub fn table(name: &str) -> String {
        let snake = Self::snake(name);
        if snake.ends_with('s') || snake.ends_with('x') || snake.ends_with('z') {
            format!("{snake}es")
        } else {
            format!("{snake}s")
        }
    }

    /// Resolve an absolute target path for a workspace-relative `rel`.
    pub fn resolve(root: &Path, rel: &str) -> PathBuf {
        root.join(rel)
    }

    /// Write a rendered file, honouring `--force`.
    pub fn write(
        root: &Path,
        file: &GeneratedFile,
        force: bool,
    ) -> Result<Generated, GeneratorError> {
        let absolute = Self::resolve(root, &file.relative_path);
        if absolute.exists() && !force {
            return Err(GeneratorError::AlreadyExists {
                path: file.relative_path.clone(),
            });
        }
        if let Some(parent) = absolute.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|source| GeneratorError::Mkdir {
                    path: file.relative_path.clone(),
                    source,
                })?;
            }
        }
        std::fs::write(&absolute, file.source.as_bytes()).map_err(|source| {
            GeneratorError::Write {
                path: file.relative_path.clone(),
                source,
            }
        })?;
        Ok(Generated {
            path: file.relative_path.clone(),
            overwritten: absolute.exists() && force,
        })
    }

    /// Render a file without writing it (dry-run / test helper).
    pub fn dry(file: GeneratedFile) -> GeneratedFile {
        file
    }
}

/// Convenience alias mirroring the crate-level generator result.
pub type GeneratorResult<T> = CliResult<T>;

/// Write a single file from parts, lifting errors into the CLI surface.
pub fn scaffold(
    root: &Path,
    relative_path: &str,
    source: String,
    force: bool,
) -> CliResult<Generated> {
    Generator::write(
        root,
        &GeneratedFile {
            relative_path: relative_path.to_string(),
            source,
        },
        force,
    )
    .map_err(Into::into)
}
