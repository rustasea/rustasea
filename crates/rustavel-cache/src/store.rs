/// Driver-agnostic cache store trait with a default `touch` implementation.
use std::time::Duration;

use async_trait::async_trait;

use crate::error::Result;

/// Prefix applied to every key passing through the repository layer.
pub const DEFAULT_PREFIX: &str = "-cache-";

/// A cache backend storing opaque byte values with a TTL.
///
/// Implementors are the `memory` (moka-like) and `redis` stores. The trait is
/// deliberately byte-oriented: typed (de)serialization lives in the
/// `Repository` facade so stores stay driver-agnostic (FS-M4-04).
#[async_trait]
pub trait Store: Send + Sync {
    /// Fetch the raw value for `key`, or `None` on a miss/expiry.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Store `value` under `key` for `ttl`.
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()>;

    /// Store `value` under `key` only when `key` is absent.
    ///
    /// Atomic compare-and-set backing `Lock` acquisition (`SET NX EX` on Redis,
    /// insert-if-absent under one map guard in memory). Returns `Ok(true)` when
    /// the value was stored, `Ok(false)` when `key` already exists. The default
    /// falls back to a non-atomic check-then-put so custom drivers written
    /// before this method existed keep compiling (additive contract); the
    /// bundled memory/redis stores override it with the true atomic form.
    async fn put_if_absent(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<bool> {
        if self.get(key).await?.is_some() {
            return Ok(false);
        }
        self.put(key, value, ttl).await?;
        Ok(true)
    }

    /// Remove `key` only when its current value equals `expected`.
    ///
    /// Atomic compare-and-delete backing `LockGuard::release`, so a lease that
    /// expired and was re-acquired by another owner is never deleted by the
    /// stale holder (Redis Lua `if get == expected then del`, in-memory under
    /// one map guard). Returns `Ok(true)` when the matching value was removed,
    /// `Ok(false)` when the key is absent or holds a different value. The
    /// default falls back to a non-atomic read-then-delete for drivers written
    /// before this method existed (additive contract).
    async fn compare_and_delete(&self, key: &str, expected: &[u8]) -> Result<bool> {
        match self.get(key).await? {
            Some(current) if current == expected => {
                self.forget(key).await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Extend the TTL of `key` to `ttl` without reading the value.
    ///
    /// Returns `Ok(false)` when the key is missing (not an error) and
    /// `Ok(true)` after a successful extension. Store-level failures surface
    /// as `CacheError::TouchFailed`. The default implementation reports
    /// `Ok(false)` so custom drivers added before `touch` existed keep
    /// compiling (additive contract, FSD §4.1).
    async fn touch(&self, key: &str, ttl: Duration) -> Result<bool> {
        let _ = (key, ttl);
        Ok(false)
    }

    /// Remove `key` from the store.
    async fn forget(&self, key: &str) -> Result<()>;

    /// Remove every key from the store.
    async fn flush(&self) -> Result<()>;

    /// Atomically add `n` to the numeric value under `key`, returning the new
    /// value. A missing key is treated as zero.
    async fn increment(&self, key: &str, n: i64) -> Result<i64>;

    /// Atomically subtract `n` from the numeric value under `key`, returning
    /// the new value.
    async fn decrement(&self, key: &str, n: i64) -> Result<i64> {
        self.increment(key, -n).await
    }
}
