//! Storage trait — the read/write contract implemented by disks.

use async_trait::async_trait;

use crate::error::Result;

/// Storage backend contract (Laravel `Storage` facade parity).
///
/// Object keys are forward-slash paths relative to the disk root; `get`
/// returns raw bytes, `put` writes them, `exists` probes presence, and
/// `delete` removes. Path confinement is applied by concrete disks.
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Read the object at `key`, or `NotFound` when absent.
    async fn get(&self, key: &str) -> Result<Vec<u8>>;

    /// Write `bytes` to `key`, creating parent directories as needed.
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<()>;

    /// Whether an object exists at `key`.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Delete the object at `key`; absent keys are a no-op.
    async fn delete(&self, key: &str) -> Result<()>;
}
