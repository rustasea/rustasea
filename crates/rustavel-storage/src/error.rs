/// Storage error types for the filesystem layer.
use std::io;

use thiserror::Error;

/// Alias for results produced by storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

/// Top-level storage error type.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The requested object does not exist on any configured disk.
    #[error("object not found: {0}")]
    NotFound(String),

    /// The path escapes the configured disk root (NFR-Sec-03).
    #[error("path traversal detected: {0}")]
    PathTraversal(String),

    /// The underlying filesystem call failed.
    #[error("io error for {path}: {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// No disk is registered under the requested name.
    #[error("unknown disk: {0}")]
    UnknownDisk(String),
}

/// Error produced by [`crate::confine_path`] on a traversal attempt.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("path traversal detected: {0}")]
pub struct PathError(pub String);

impl From<PathError> for StorageError {
    /// Promote a confinement failure into the storage error space.
    fn from(e: PathError) -> Self {
        StorageError::PathTraversal(e.0)
    }
}
