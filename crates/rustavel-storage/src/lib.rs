//! Rustavel Storage — read-through disk storage with confined paths.
//!
//! Sprint 07 (M6) scope: the `Storage` trait, read-through local disks
//! (primary + fallback with optional copy-back), and path confinement that
//! rejects traversal (`PathTraversal`, NFR-Sec-03).

pub mod disk;
pub mod error;
pub mod path;
pub mod storage;

pub use crate::disk::{DiskKind, LocalDisk, ReadThrough, ReadThroughDisk};
pub use crate::error::{PathError, Result, StorageError};
pub use crate::path::{confine_path, PathOutcome};
pub use crate::storage::Storage;

/// Copy-back policy for read-through storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyBack {
    /// Never copy fallback hits back to the primary.
    Never,
    /// Copy fallback hits back to the primary on read.
    OnRead,
}

/// Outcome of a read-through `get`: which disk served the object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSource {
    /// Served by the primary disk.
    Primary,
    /// Served by the fallback disk.
    Fallback,
}
