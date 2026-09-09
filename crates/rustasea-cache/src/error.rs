/// Typed errors for the cache layer.
use thiserror::Error;

/// Alias for results produced by cache operations.
pub type Result<T> = std::result::Result<T, CacheError>;

/// Top-level cache error type (api-cache.md §4 catalogue).
#[derive(Debug, Error)]
pub enum CacheError {
    /// The backing store (Redis/Memory) is unreachable.
    #[error("cache store unavailable: {0}")]
    StoreUnavailable(String),

    /// A TTL-extension (`touch`) failed at the store level.
    #[error("cache touch failed: {0}")]
    TouchFailed(String),

    /// A value could not be serialized/deserialized.
    #[error("cache serialization failed: {0}")]
    Serialization(String),

    /// No store is registered under the requested name.
    #[error("unknown cache store: {0}")]
    UnknownStore(String),
}

/// Lock-specific error surfaced by `Lock::block` timeouts.
#[derive(Debug, Error)]
pub enum LockError {
    /// The lock is still held when the wait window expires.
    #[error("lock contention: {key} already held after {waited:?}")]
    AlreadyHeld {
        /// Lock key that stayed held.
        key: String,
        /// Total time waited before giving up.
        waited: std::time::Duration,
    },

    /// The underlying store failed while acquiring/releasing the lock.
    #[error("lock store unavailable: {0}")]
    StoreUnavailable(String),

    /// The lease expired while blocked; the holder may still be running.
    #[error("lock lease expired for {0}")]
    LeaseExpired(String),
}

impl From<LockError> for CacheError {
    /// Promote a lock failure into the cache error space.
    fn from(e: LockError) -> Self {
        match e {
            LockError::AlreadyHeld { key, waited } => {
                CacheError::TouchFailed(format!("lock {key} held after {waited:?}"))
            }
            LockError::StoreUnavailable(m) => CacheError::StoreUnavailable(m),
            LockError::LeaseExpired(k) => CacheError::TouchFailed(format!("lease expired: {k}")),
        }
    }
}
