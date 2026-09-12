//! Redis-backed cache store.
//!
//! Compiled in two shapes. With the `redis` feature the store drives a real
//! `deadpool-redis` pool using atomic RESP commands — `SET NX PX` for
//! compare-and-set and a Lua compare-and-delete for lock release. Without the
//! feature the store is inert: construction never contacts a server and every
//! operation fails with a typed `StoreUnavailable` rather than a fake success.
//! Keys are namespaced as `{prefix}{key}` so `flush` only removes keys owned by
//! this store.
use std::time::Duration;

use async_trait::async_trait;

use crate::error::{CacheError, Result};
use crate::store::Store;

/// Default key prefix applied to every key this store touches.
pub const DEFAULT_REDIS_PREFIX: &str = "rustasea:cache:";

#[cfg(feature = "redis")]
use deadpool_redis::redis::{cmd, AsyncCommands, ExistenceCheck, Script, SetExpiry, SetOptions};
#[cfg(feature = "redis")]
use deadpool_redis::{Config, Pool, Runtime};

/// Lua compare-and-delete: removes `KEYS[1]` only when it holds `ARGV[1]`.
#[cfg(feature = "redis")]
const COMPARE_AND_DELETE_LUA: &str = r"
if redis.call('GET', KEYS[1]) == ARGV[1] then
    return redis.call('DEL', KEYS[1])
end
return 0
";

/// Lua persist-and-report: drops `KEYS[1]`'s timeout and reports existence.
///
/// Runs as one atomic script so a key that expires between the existence check
/// and `PERSIST` can never be reported as touched (FIX-009).
#[cfg(feature = "redis")]
const PERSIST_IF_EXISTS_LUA: &str = r"
if redis.call('EXISTS', KEYS[1]) == 1 then
    redis.call('PERSIST', KEYS[1])
    return 1
end
return 0
";

/// Keys removed per `UNLINK`/`DEL` batch during `flush`.
#[cfg(feature = "redis")]
const FLUSH_BATCH_SIZE: usize = 500;

/// `SCAN` `COUNT` hint used while draining the store's keyspace.
#[cfg(feature = "redis")]
const FLUSH_SCAN_COUNT: usize = 500;

/// Redis-backed cache store.
///
/// With the `redis` feature a store owns a lazily-connected `deadpool-redis`
/// pool; each command borrows a pooled connection and maps every pool or
/// command failure to a typed [`CacheError::StoreUnavailable`]. Without the
/// feature the store is inert — all operations return `StoreUnavailable`.
#[derive(Debug, Clone)]
pub struct RedisStore {
    /// Connection name this store is registered under (diagnostics).
    name: String,
    /// Key prefix that scopes this store's keys and `flush`.
    prefix: String,
    /// Pooled Redis connections; `None` when the store is inert.
    #[cfg(feature = "redis")]
    pool: Option<Pool>,
}

impl RedisStore {
    /// Create an inert store that never contacts a Redis server.
    ///
    /// Use [`RedisStore::from_url`] (feature `redis`) to build a live store.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            prefix: DEFAULT_REDIS_PREFIX.to_string(),
            #[cfg(feature = "redis")]
            pool: None,
        }
    }

    /// Build a live store for `url` (feature `redis`).
    ///
    /// A blank URL yields an inert store. The pool is created eagerly but does
    /// not connect until the first command, so construction succeeds against an
    /// absent server and the first call surfaces a typed error instead.
    #[cfg(feature = "redis")]
    pub fn from_url(name: impl Into<String>, url: impl AsRef<str>) -> Result<Self> {
        match url.as_ref().trim() {
            "" => Ok(Self::new(name)),
            url => {
                let pool = Config::from_url(url)
                    .create_pool(Some(Runtime::Tokio1))
                    .map_err(|e| CacheError::StoreUnavailable(e.to_string()))?;
                Ok(Self::from_pool(name, pool))
            }
        }
    }

    /// Build a live store over an existing pool (feature `redis`).
    #[cfg(feature = "redis")]
    pub fn from_pool(name: impl Into<String>, pool: Pool) -> Self {
        Self {
            name: name.into(),
            prefix: DEFAULT_REDIS_PREFIX.to_string(),
            pool: Some(pool),
        }
    }

    /// Override the key prefix (defaults to [`DEFAULT_REDIS_PREFIX`]).
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Connection name of this store.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether this store has a configured Redis pool.
    #[cfg(feature = "redis")]
    pub fn is_enabled(&self) -> bool {
        self.pool.is_some()
    }

    /// Whether this store has a configured Redis pool (always `false` without the feature).
    #[cfg(not(feature = "redis"))]
    pub fn is_enabled(&self) -> bool {
        false
    }

    /// Fully-qualified Redis key for `key`.
    fn qualified(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key)
    }

    /// Acquire a pooled connection, or a typed error when the store is inert.
    #[cfg(feature = "redis")]
    async fn connection(&self) -> Result<deadpool_redis::Connection> {
        let pool = self.pool.as_ref().ok_or_else(|| {
            CacheError::StoreUnavailable("redis connection not configured".to_string())
        })?;
        pool.get()
            .await
            .map_err(|e| CacheError::StoreUnavailable(e.to_string()))
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl Store for RedisStore {
    /// Fetch the raw value for `key`, or `None` on a miss/expiry.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.connection().await?;
        conn.get(self.qualified(key)).await.map_err(redis_error)
    }

    /// Store `value` under `key` for `ttl` (`PSETEX`, or `SET` for no expiry).
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        let mut conn = self.connection().await?;
        let key = self.qualified(key);
        if ttl.is_zero() {
            let _: () = conn.set(key, value).await.map_err(redis_error)?;
        } else {
            let _: () = conn
                .pset_ex(key, value, ttl_millis(ttl))
                .await
                .map_err(redis_error)?;
        }
        Ok(())
    }

    /// Insert `value` under `key` only when absent (`SET NX [PX]`).
    async fn put_if_absent(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<bool> {
        let mut conn = self.connection().await?;
        let mut options = SetOptions::default().conditional_set(ExistenceCheck::NX);
        if !ttl.is_zero() {
            options = options.with_expiration(SetExpiry::PX(ttl_millis(ttl)));
        }
        let stored: Option<String> = conn
            .set_options(self.qualified(key), value, options)
            .await
            .map_err(redis_error)?;
        Ok(stored.is_some())
    }

    /// Delete `key` only when it holds `expected` (atomic Lua compare-and-delete).
    async fn compare_and_delete(&self, key: &str, expected: &[u8]) -> Result<bool> {
        let mut conn = self.connection().await?;
        let deleted: i64 = Script::new(COMPARE_AND_DELETE_LUA)
            .key(self.qualified(key))
            .arg(expected)
            .invoke_async(&mut conn)
            .await
            .map_err(redis_error)?;
        Ok(deleted > 0)
    }

    /// Extend the TTL of `key` to `ttl` without reading the value.
    ///
    /// `ttl == 0` atomically removes the expiry (`PERSIST`) and reports whether
    /// the key existed, so a key expiring mid-call can never yield a false
    /// success (FIX-009).
    async fn touch(&self, key: &str, ttl: Duration) -> Result<bool> {
        let mut conn = self.connection().await?;
        let key = self.qualified(key);
        if ttl.is_zero() {
            let existed: i64 = Script::new(PERSIST_IF_EXISTS_LUA)
                .key(key)
                .invoke_async(&mut conn)
                .await
                .map_err(touch_error)?;
            Ok(existed > 0)
        } else {
            let millis = i64::try_from(ttl_millis(ttl)).unwrap_or(i64::MAX);
            let extended: bool = conn.pexpire(&key, millis).await.map_err(touch_error)?;
            Ok(extended)
        }
    }

    /// Remove `key`.
    async fn forget(&self, key: &str) -> Result<()> {
        let mut conn = self.connection().await?;
        let _: () = conn.del(self.qualified(key)).await.map_err(redis_error)?;
        Ok(())
    }

    /// Remove every key owned by this store.
    ///
    /// Drains the keyspace with cursor-based `SCAN` and deletes each bounded
    /// batch via non-blocking `UNLINK` (falling back to `DEL`), so a large
    /// keyspace never builds an unbounded key vector or blocks Redis with one
    /// giant deletion (FIX-008).
    async fn flush(&self) -> Result<()> {
        let mut conn = self.connection().await?;
        let pattern = format!("{}*", self.prefix);
        let mut cursor: u64 = 0;
        loop {
            let (next, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(FLUSH_SCAN_COUNT)
                .query_async(&mut conn)
                .await
                .map_err(redis_error)?;
            for batch in keys.chunks(FLUSH_BATCH_SIZE) {
                delete_batch(&mut conn, batch).await?;
            }
            cursor = next;
            if cursor == 0 {
                break;
            }
        }
        Ok(())
    }

    /// Atomically add `n` to the numeric value under `key` (`INCRBY`).
    async fn increment(&self, key: &str, n: i64) -> Result<i64> {
        let mut conn = self.connection().await?;
        conn.incr(self.qualified(key), n).await.map_err(redis_error)
    }

    /// Atomically subtract `n` from the numeric value under `key` (`DECRBY`).
    async fn decrement(&self, key: &str, n: i64) -> Result<i64> {
        let mut conn = self.connection().await?;
        conn.decr(self.qualified(key), n).await.map_err(redis_error)
    }
}

#[cfg(not(feature = "redis"))]
#[async_trait]
impl Store for RedisStore {
    /// Always `StoreUnavailable` — the `redis` feature is not compiled in.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — the `redis` feature is not compiled in.
    async fn put(&self, key: &str, _value: Vec<u8>, _ttl: Duration) -> Result<()> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — never a fake acquisition.
    async fn put_if_absent(&self, key: &str, _value: Vec<u8>, _ttl: Duration) -> Result<bool> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — never a fake deletion.
    async fn compare_and_delete(&self, key: &str, _expected: &[u8]) -> Result<bool> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — never a fake TTL extension.
    async fn touch(&self, key: &str, _ttl: Duration) -> Result<bool> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — the `redis` feature is not compiled in.
    async fn forget(&self, key: &str) -> Result<()> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — the `redis` feature is not compiled in.
    async fn flush(&self) -> Result<()> {
        Err(self.unavailable(""))
    }

    /// Always `StoreUnavailable` — never a fake increment.
    async fn increment(&self, key: &str, _n: i64) -> Result<i64> {
        Err(self.unavailable(key))
    }

    /// Always `StoreUnavailable` — never a fake decrement.
    async fn decrement(&self, key: &str, _n: i64) -> Result<i64> {
        Err(self.unavailable(key))
    }
}

#[cfg(not(feature = "redis"))]
impl RedisStore {
    /// Typed error for an inert store (feature `redis` not compiled in).
    fn unavailable(&self, key: &str) -> CacheError {
        CacheError::StoreUnavailable(format!(
            "redis store disabled for '{}': build with the `redis` feature",
            self.qualified(key)
        ))
    }
}

/// Convert a duration to a positive Redis millisecond TTL.
#[cfg(feature = "redis")]
fn ttl_millis(ttl: Duration) -> u64 {
    let ms = ttl.as_millis();
    if ms == 0 {
        1
    } else {
        ms.min(u128::from(u64::MAX)) as u64
    }
}

/// Map a Redis command error onto the cache store error.
#[cfg(feature = "redis")]
fn redis_error(error: deadpool_redis::redis::RedisError) -> CacheError {
    CacheError::StoreUnavailable(error.to_string())
}

/// Map a Redis `touch` command error onto the TTL-extension error.
#[cfg(feature = "redis")]
fn touch_error(error: deadpool_redis::redis::RedisError) -> CacheError {
    CacheError::TouchFailed(error.to_string())
}

/// Delete a bounded batch of keys, preferring non-blocking `UNLINK`.
///
/// Falls back to `DEL` when the server does not support `UNLINK`; the fallback
/// error is mapped through the store error so failures stay typed.
#[cfg(feature = "redis")]
async fn delete_batch(conn: &mut deadpool_redis::Connection, keys: &[String]) -> Result<()> {
    match conn.unlink::<_, ()>(keys).await {
        Ok(()) => Ok(()),
        Err(_) => {
            let _: () = conn.del(keys).await.map_err(redis_error)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
