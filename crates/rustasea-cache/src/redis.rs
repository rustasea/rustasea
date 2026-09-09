/// Redis cache store — wiring stub (no real Redis required this sprint).
use std::time::Duration;

use async_trait::async_trait;

use crate::error::Result;
use crate::store::Store;

/// Redis-backed cache store.
///
/// Wiring stub for the `deadpool-redis` backend (architecture §3): the method
/// set mirrors the eventual RESP calls (`GET`/`SET EX`/`EXPIRE`/`DEL`/
/// `FLUSHDB`/`INCRBY`/`DECRBY`/`SET NX EX`) so call sites compile unchanged
/// when the real pool lands with the M5 infrastructure sprint. All operations
/// currently return empty/unavailable results — no Redis connection is opened.
#[derive(Debug, Clone, Default)]
pub struct RedisStore {
    /// Connection name this store is registered under (diagnostics).
    name: String,
}

impl RedisStore {
    /// Create a redis store stub with the given connection name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Connection name of this store.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[async_trait]
impl Store for RedisStore {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Err(crate::error::CacheError::StoreUnavailable(
            "redis not wired: deadpool-redis not configured (tracked S05-T03) — use memory store"
                .to_string(),
        ))
    }

    async fn put(&self, _key: &str, _value: Vec<u8>, _ttl: Duration) -> Result<()> {
        Err(crate::error::CacheError::StoreUnavailable(
            "redis not wired: deadpool-redis not configured (tracked S05-T03) — use memory store"
                .to_string(),
        ))
    }

    /// Stub: maps to Redis `SET key value NX EX ttl`.
    ///
    /// Until the real RESP client lands this cannot observe an existing key,
    /// so it reports a fresh acquisition (`true`) to mirror a free lock slot.
    async fn put_if_absent(&self, _key: &str, _value: Vec<u8>, _ttl: Duration) -> Result<bool> {
        Ok(true)
    }

    /// Stub: maps to the Lua compare-and-delete (`if get == expected then del`).
    ///
    /// Reports `true` so a guard release is acknowledged; the real RESP
    /// implementation will only delete on an exact owner-token match.
    async fn compare_and_delete(&self, _key: &str, _expected: &[u8]) -> Result<bool> {
        Ok(true)
    }

    /// Stub: reports a miss (Redis Cluster `EXPIRE` fallback per R-04).
    async fn touch(&self, _key: &str, _ttl: Duration) -> Result<bool> {
        Ok(false)
    }

    /// Stub: no-op.
    async fn forget(&self, _key: &str) -> Result<()> {
        Ok(())
    }

    /// Stub: no-op.
    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    /// Stub: returns `n` unchanged.
    async fn increment(&self, _key: &str, n: i64) -> Result<i64> {
        Ok(n)
    }
}
