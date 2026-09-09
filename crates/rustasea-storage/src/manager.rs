//! StorageManager facade — named disks with read-through routing.
//!
//! Mirrors the FS-M6-02 contract: `StorageManager { disks }` with a
//! `{ primary, fallback, copy_back }` configuration. `get` reads the primary
//! and falls through to the fallback on a miss; with `copy_back` enabled the
//! fallback bytes are promoted to the primary. `path` confinement is
//! delegated to each named disk so traversal is rejected at path time
//! (NFR-Sec-03) without a filesystem probe.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use object_store::ObjectStoreExt;
use serde::Deserialize;

use crate::error::{Result, StorageError};
use crate::storage::Storage;

/// A concrete, named disk that also knows how to confine its own paths.
///
/// Implemented by both [`LocalDisk`](crate::LocalDisk) (filesystem roots) and
/// [`ObjectDisk`] (object_store virtual prefixes); object stores confine
/// logically via their path grammar, local disks via canonical containment.
#[async_trait]
pub trait ManagedDisk: Storage {
    /// Canonical, confined path for `key`, or `PathTraversal` on escape.
    ///
    /// Local disks resolve `key` under their root; object stores validate the
    /// key lexically (`..` segments are rejected by the object path grammar).
    fn path(&self, key: &str) -> Result<PathBuf>;

    /// Root label used in error metadata (`/srv/app/storage/app`, `s3://…`).
    fn label(&self) -> String;
}

/// Read-through configuration (`{ primary, fallback, copy_back }`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StorageConfig {
    /// Primary disk name (served first).
    pub primary: String,
    /// Fallback disk name (served on primary miss).
    pub fallback: String,
    /// Whether fallback hits are promoted to the primary.
    pub copy_back: bool,
}

/// Facade over named disks implementing read-through routing.
///
/// `new`/`from_config` start with an empty disk map; register disks with
/// [`StorageManager::with_disk`] before use. Unknown disk names surface as
/// [`StorageError::StoreUnavailable`].
#[derive(Clone)]
pub struct StorageManager {
    /// Named disks available for routing.
    disks: HashMap<String, Arc<dyn ManagedDisk>>,
    /// Read-through configuration.
    config: StorageConfig,
}

impl StorageManager {
    /// Create a read-through manager over `primary` + `fallback`.
    pub fn new(primary: impl Into<String>, fallback: impl Into<String>, copy_back: bool) -> Self {
        Self::from_config(StorageConfig {
            primary: primary.into(),
            fallback: fallback.into(),
            copy_back,
        })
    }

    /// Create a manager from a full configuration struct.
    pub fn from_config(config: StorageConfig) -> Self {
        Self {
            disks: HashMap::new(),
            config,
        }
    }

    /// Register a named disk (builder style).
    pub fn with_disk(mut self, name: impl Into<String>, disk: Arc<dyn ManagedDisk>) -> Self {
        self.disks.insert(name.into(), disk);
        self
    }

    /// Register a local disk under `name`.
    pub fn with_local(mut self, name: impl Into<String>, disk: crate::LocalDisk) -> Self {
        self.disks.insert(name.into(), Arc::new(disk));
        self
    }

    /// Register an object-store-backed disk under `name`.
    pub fn with_object(mut self, name: impl Into<String>, disk: ObjectDisk) -> Self {
        self.disks.insert(name.into(), Arc::new(disk));
        self
    }

    /// Look up the disk registered under `name`.
    pub fn disk(&self, name: &str) -> Result<Arc<dyn ManagedDisk>> {
        self.disks
            .get(name)
            .cloned()
            .ok_or_else(|| StorageError::UnknownDisk(name.to_string()))
    }

    /// Confine `key` under the primary disk's root.
    ///
    /// Traversal attempts (`../../etc/passwd`) yield
    /// [`StorageError::PathTraversal`] with no filesystem probe.
    pub fn path(&self, key: &str) -> Result<PathBuf> {
        self.disk(&self.config.primary)?.path(key)
    }

    /// Read `key` with read-through: primary, then fallback on miss.
    ///
    /// Fallback hits are promoted to the primary when `copy_back` is enabled;
    /// a miss on both surfaces a uniform [`StorageError::NotFound`].
    pub async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let primary = self.disk(&self.config.primary)?;
        match primary.get(key).await {
            Ok(bytes) => Ok(bytes),
            Err(StorageError::NotFound(_)) => {
                let fallback = self.disk(&self.config.fallback)?;
                let bytes = fallback.get(key).await?;
                if self.config.copy_back {
                    let _ = primary.put(key, &bytes).await;
                }
                Ok(bytes)
            }
            Err(e) => Err(e),
        }
    }

    /// Write `bytes` to `key` on the primary disk.
    pub async fn put(&self, key: &str, bytes: &[u8]) -> Result<()> {
        self.disk(&self.config.primary)?.put(key, bytes).await
    }

    /// Whether `key` exists on the primary (or fallback).
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let primary = self.disk(&self.config.primary)?;
        if primary.exists(key).await? {
            return Ok(true);
        }
        self.disk(&self.config.fallback)?.exists(key).await
    }

    /// Delete `key` from both the primary and the fallback.
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.disk(&self.config.primary)?.delete(key).await?;
        self.disk(&self.config.fallback)?.delete(key).await
    }
}

/// Object-store-backed disk wrapping an `object_store` backend.
///
/// S3/GCS/Azure/LocalFileSystem/InMemory stores all implement the same
/// `object_store::ObjectStore` interface; this adapter exposes them through
/// the crate's `Storage` contract. Keys are confined by the object path
/// grammar (`..` segments are rejected → `PathTraversal`).
#[derive(Clone)]
pub struct ObjectDisk {
    /// Backend store (S3, GCS, Azure, local fs, in-memory, …).
    store: Arc<dyn object_store::ObjectStore>,
    /// Root label for error metadata.
    label: String,
}

impl ObjectDisk {
    /// Wrap an object store under a human label.
    pub fn new(store: Arc<dyn object_store::ObjectStore>, label: impl Into<String>) -> Self {
        Self {
            store,
            label: label.into(),
        }
    }

    /// Access the wrapped object store.
    pub fn store(&self) -> Arc<dyn object_store::ObjectStore> {
        Arc::clone(&self.store)
    }

    /// Parse a logical object key; `..`/empty segments are rejected.
    fn location(&self, key: &str) -> Result<object_store::path::Path> {
        object_store::path::Path::parse(key).map_err(|_| StorageError::PathTraversal(key.into()))
    }
}

#[async_trait]
impl ManagedDisk for ObjectDisk {
    fn path(&self, key: &str) -> Result<PathBuf> {
        // Logical confinement: the object path grammar rejects traversal
        // segments without touching the network or filesystem.
        self.location(key)?;
        Ok(PathBuf::from(key))
    }

    fn label(&self) -> String {
        self.label.clone()
    }
}

#[async_trait]
impl Storage for ObjectDisk {
    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let loc = self.location(key)?;
        let result = self.store.get(&loc).await.map_err(map_object_error)?;
        let bytes = result.bytes().await.map_err(map_object_error)?;
        Ok(bytes.to_vec())
    }

    async fn put(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let loc = self.location(key)?;
        self.store
            .put(&loc, bytes.to_vec().into())
            .await
            .map_err(map_object_error)?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let loc = self.location(key)?;
        match self.store.head(&loc).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(StorageError::StoreUnavailable(e.to_string())),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let loc = self.location(key)?;
        match self.store.delete(&loc).await {
            Ok(()) => Ok(()),
            // Absent objects are a no-op (parity with LocalDisk).
            Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(StorageError::StoreUnavailable(e.to_string())),
        }
    }
}

/// Map an `object_store` failure into the crate's error space.
fn map_object_error(e: object_store::Error) -> StorageError {
    match e {
        object_store::Error::NotFound { path, .. } => StorageError::NotFound(path),
        other => StorageError::StoreUnavailable(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalDisk;

    fn tmp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rustasea-manager-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    #[tokio::test]
    async fn fallback_read_returns_local_bytes_and_copy_back_promotes() {
        let root_p = tmp("rt-primary");
        let root_f = tmp("rt-fallback");
        std::fs::create_dir_all(&root_p).unwrap();
        std::fs::create_dir_all(&root_f).unwrap();

        // Primary = local-fs object store ("s3"), fallback = LocalDisk.
        let primary_store = object_store::local::LocalFileSystem::new_with_prefix(&root_p).unwrap();
        let fallback = LocalDisk::new(&root_f);
        fallback.put("a/b.txt", b"fallback-data").await.unwrap();

        let manager = StorageManager::new("s3", "local", true)
            .with_object(
                "s3",
                ObjectDisk::new(Arc::new(primary_store), root_p.display().to_string()),
            )
            .with_local("local", fallback);

        assert_eq!(
            manager.get("a/b.txt").await.unwrap(),
            b"fallback-data".to_vec()
        );

        // Copy-back: primary now serves the object.
        let probe = LocalDisk::new(&root_p);
        assert!(probe.exists("a/b.txt").await.unwrap());
    }

    #[test]
    fn path_confinement_rejects_traversal_on_local_primary() {
        let root = tmp("path-local");
        let manager = StorageManager::new("local", "missing", false)
            .with_local("local", LocalDisk::new(&root));
        let err = manager.path("../../etc/passwd").unwrap_err();
        assert!(matches!(err, StorageError::PathTraversal(_)));
    }

    #[test]
    fn path_confinement_rejects_traversal_on_object_primary() {
        let root = tmp("path-object");
        std::fs::create_dir_all(&root).unwrap();
        let store = object_store::local::LocalFileSystem::new_with_prefix(&root).unwrap();
        let manager = StorageManager::new("s3", "missing", false)
            .with_object("s3", ObjectDisk::new(Arc::new(store), "s3://bucket"));
        let err = manager.path("../../etc/passwd").unwrap_err();
        assert!(matches!(err, StorageError::PathTraversal(_)));
    }

    #[tokio::test]
    async fn both_missing_is_uniform_not_found() {
        let root_p = tmp("miss-primary");
        let root_f = tmp("miss-fallback");
        std::fs::create_dir_all(&root_p).unwrap();
        std::fs::create_dir_all(&root_f).unwrap();
        let store = object_store::local::LocalFileSystem::new_with_prefix(&root_p).unwrap();
        let manager = StorageManager::new("s3", "local", false)
            .with_object("s3", ObjectDisk::new(Arc::new(store), "s3://bucket"))
            .with_local("local", LocalDisk::new(&root_f));

        let err = manager.get("missing.bin").await.unwrap_err();
        assert!(matches!(err, StorageError::NotFound(_)));
    }

    #[test]
    fn config_round_trips_copy_back_bool() {
        let config: StorageConfig =
            serde_json::from_str(r#"{ "primary": "s3", "fallback": "local", "copy_back": true }"#)
                .unwrap();
        assert!(config.copy_back);
        assert_eq!(config.primary, "s3");
    }
}
