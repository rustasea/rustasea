//! Local disk + read-through implementations of the `Storage` trait.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::{Result, StorageError};
use crate::path::confine_path;
use crate::{CopyBack, ReadSource, Storage};

/// Disk role inside a read-through pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskKind {
    /// Served first; fallback hits may copy back to it.
    Primary,
    /// Served when the primary misses.
    Fallback,
}

/// Name a concrete disk in a read-through configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadThrough {
    /// Primary disk name (`"s3"`, `"local"`, …).
    pub primary: String,
    /// Fallback disk name (`"local"`, …).
    pub fallback: String,
    /// Whether fallback hits are copied back to the primary.
    pub copy_back: CopyBack,
}

impl ReadThrough {
    /// Create a read-through configuration with `OnRead` copy-back default.
    pub fn new(primary: impl Into<String>, fallback: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            fallback: fallback.into(),
            copy_back: CopyBack::OnRead,
        }
    }

    /// Disable copy-back (fallback reads never touch the primary).
    pub fn without_copy_back(mut self) -> Self {
        self.copy_back = CopyBack::Never;
        self
    }
}

/// Filesystem-backed `Storage` rooted at a directory.
///
/// The disk root is created lazily on first write; every operation confines
/// its key via [`confine_path`] so `../../` escapes surface as
/// [`StorageError::PathTraversal`].
#[derive(Debug, Clone)]
pub struct LocalDisk {
    root: PathBuf,
}

impl LocalDisk {
    /// Create a disk rooted at `dir`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { root: dir.into() }
    }

    /// Absolute, canonicalized root of this disk.
    pub fn root(&self) -> PathBuf {
        self.root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone())
    }

    fn resolve(&self, key: &str) -> Result<PathBuf> {
        let rel = Path::new(key);
        if rel.is_absolute() {
            return Err(StorageError::PathTraversal(format!(
                "absolute key not allowed: {key}"
            )));
        }
        let root = self.root();
        std::fs::create_dir_all(&root).map_err(|e| StorageError::Io {
            path: root.display().to_string(),
            source: e,
        })?;
        match confine_path(&root, &root.join(rel))? {
            crate::path::PathOutcome::Confined(path) => Ok(path),
            crate::path::PathOutcome::Traversal => {
                Err(StorageError::PathTraversal(key.to_string()))
            }
        }
    }
}

#[async_trait]
impl Storage for LocalDisk {
    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.resolve(key)?;
        tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(key.to_string())
            } else {
                StorageError::Io {
                    path: path.display().to_string(),
                    source: e,
                }
            }
        })
    }

    async fn put(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let path = self.resolve(key)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io {
                    path: parent.display().to_string(),
                    source: e,
                })?;
        }
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| StorageError::Io {
                path: path.display().to_string(),
                source: e,
            })
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.resolve(key)?;
        Ok(path.exists())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.resolve(key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(StorageError::Io {
                path: path.display().to_string(),
                source: e,
            }),
        }
    }
}

/// Read-through orchestrator: primary first, fallback on miss.
///
/// Implements the FS-M6-03 contract — `get` returns the fallback object when
/// the primary misses and (optionally) copies it back to the primary so the
/// next read hits the primary.
pub struct ReadThroughDisk {
    /// Named disks available for routing.
    disks: HashMap<String, LocalDisk>,
    /// Primary + fallback routing.
    config: ReadThrough,
}

impl ReadThroughDisk {
    /// Create a read-through disk over named local disks.
    pub fn new(config: ReadThrough, disks: HashMap<String, LocalDisk>) -> Self {
        Self { disks, config }
    }

    /// Look up the disk registered under `name`.
    fn disk(&self, name: &str) -> Result<&LocalDisk> {
        self.disks
            .get(name)
            .ok_or_else(|| StorageError::UnknownDisk(name.to_string()))
    }
}

#[async_trait]
impl Storage for ReadThroughDisk {
    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let primary = self.disk(&self.config.primary)?;
        match primary.get(key).await {
            Ok(bytes) => Ok(bytes),
            Err(StorageError::NotFound(_)) => {
                let fallback = self.disk(&self.config.fallback)?;
                let bytes = fallback.get(key).await?;
                if matches!(self.config.copy_back, CopyBack::OnRead) {
                    let _ = primary.put(key, &bytes).await;
                }
                Ok(bytes)
            }
            Err(e) => Err(e),
        }
    }

    async fn put(&self, key: &str, bytes: &[u8]) -> Result<()> {
        self.disk(&self.config.primary)?.put(key, bytes).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let primary = self.disk(&self.config.primary)?;
        if primary.exists(key).await? {
            return Ok(true);
        }
        self.disk(&self.config.fallback)?.exists(key).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let primary = self.disk(&self.config.primary)?;
        let fallback = self.disk(&self.config.fallback)?;
        primary.delete(key).await?;
        fallback.delete(key).await
    }
}

/// Track which disk served a read — public helper for observability.
pub fn track_source(source: ReadSource) -> ReadSource {
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustavel-storage-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    #[tokio::test]
    async fn traversal_is_rejected() {
        let disk = LocalDisk::new(tmp("trav"));
        let err = disk.get("../../etc/passwd").await.unwrap_err();
        assert!(matches!(err, StorageError::PathTraversal(_)));
    }

    #[tokio::test]
    async fn read_through_falls_back_and_copies_back() {
        let root_p = tmp("rt-primary");
        let root_f = tmp("rt-fallback");
        let fallback = LocalDisk::new(&root_f);
        fallback.put("a/b.txt", b"fallback-data").await.unwrap();

        let mut disks = HashMap::new();
        disks.insert("s3".to_string(), LocalDisk::new(&root_p));
        disks.insert("local".to_string(), fallback);

        let rt = ReadThroughDisk::new(ReadThrough::new("s3", "local"), disks);

        assert_eq!(rt.get("a/b.txt").await.unwrap(), b"fallback-data".to_vec());

        // Copy-back (default OnRead) → primary now serves the object.
        let primary = LocalDisk::new(&root_p);
        assert!(primary.exists("a/b.txt").await.unwrap());
        assert_eq!(rt.get("a/b.txt").await.unwrap(), b"fallback-data".to_vec());
    }
}
