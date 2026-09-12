//! Error surface for the starter-kit scaffolder.
//!
//! Every failure carries the offending value or path so the `cargo-rustasea`
//! binary can print an actionable message and choose a non-zero exit code.

/// Errors produced while parsing a request or writing a generated tree.
#[derive(Debug, thiserror::Error)]
pub enum ScaffoldError {
    /// The requested application name cannot form a Rust crate / directory name.
    #[error("`{name}` is not a valid application name: expected a letter followed by letters, digits, `-`, or `_`")]
    InvalidAppName {
        /// The rejected name.
        name: String,
    },

    /// The requested presentation variant is not one of the four supported kits.
    #[error("unknown variant `{variant}`: expected one of {supported}")]
    UnknownVariant {
        /// The rejected variant token.
        variant: String,
        /// Comma-separated list of supported variants.
        supported: String,
    },

    /// A generated file already exists and `--force` was not supplied.
    #[error("{path} already exists. Pass --force to overwrite.")]
    AlreadyExists {
        /// Application-relative path of the conflicting file.
        path: String,
    },

    /// The target path exists but is not a directory.
    #[error("target `{path}` exists and is not a directory")]
    InvalidTarget {
        /// The rejected target path.
        path: String,
    },

    /// The filesystem rejected a directory creation or file write.
    #[error("failed to write {path}: {source}")]
    Io {
        /// Application-relative path being written.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

/// Convenience result alias used across the scaffolder.
pub type ScaffoldResult<T> = Result<T, ScaffoldError>;
