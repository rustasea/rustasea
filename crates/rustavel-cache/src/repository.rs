/// Cache repository facade — typed K/V, remember/forever, store selection.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{CacheError, Result};
use crate::lock::Lock;
use crate::memory::MemoryStore;
use crate::redis::RedisStore;
use crate::store::Store;

/// Canonical store names.
pub const MEMORY_STORE: &str = "memory";
/// Canonical store names.
pub const REDIS_STORE: &str = "redis";

/// Shared reference to a store.
pub type StoreRef = Arc<dyn Store>;

/// Registry of named stores plus the typed repository facade.
///
/// Stores are isolated by name: a `put` on `redis` is not visible on
/// `memory` (US-M4-03 decision table). The manager seeds both canonical stores
/// so `store(name)` never surprises; extra stores can be registered.
#[derive(Clone)]
pub struct CacheManager {
    stores: Arc<std::sync::RwLock<HashMap<String, StoreRef>>>,
}

impl CacheManager {
    /// Create a manager pre-seeded with isolated `memory` + `redis` stores.
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert(
            MEMORY_STORE.to_string(),
            Arc::new(MemoryStore::new()) as StoreRef,
        );
        map.insert(
            REDIS_STORE.to_string(),
            Arc::new(RedisStore::new(REDIS_STORE)) as StoreRef,
        );
        Self {
            stores: Arc::new(std::sync::RwLock::new(map)),
        }
    }

    /// Register (or replace) a store under `name`.
    pub fn register(&self, name: impl Into<String>, store: StoreRef) {
        if let Ok(mut map) = self.stores.write() {
            map.insert(name.into(), store);
        }
    }

    /// Resolve the store registered under `name`.
    pub fn store(&self, name: &str) -> Result<Repository> {
        let map = self
            .stores
            .read()
            .map_err(|_| CacheError::UnknownStore("cache manager registry poisoned".into()))?;
        let store = map
            .get(name)
            .cloned()
            .ok_or_else(|| CacheError::UnknownStore(name.to_string()))?;
        Ok(Repository::new(store))
    }

    /// Access the default `memory` store repository.
    pub fn repository(&self) -> Result<Repository> {
        self.store(MEMORY_STORE)
    }

    /// Access a named store repository (same as `store`).
    pub fn cache(&self, name: &str) -> Result<Repository> {
        self.store(name)
    }

    /// Create a `Lock` over the store registered under `name`.
    pub fn lock(&self, name: &str, key: impl Into<String>, ttl: Duration) -> Result<Lock> {
        let map = self
            .stores
            .read()
            .map_err(|_| CacheError::UnknownStore("cache manager registry poisoned".into()))?;
        let store = map
            .get(name)
            .cloned()
            .ok_or_else(|| CacheError::UnknownStore(name.to_string()))?;
        Ok(Lock::new(store, key, ttl))
    }
}

impl Default for CacheManager {
    /// Create a manager with isolated default stores.
    fn default() -> Self {
        Self::new()
    }
}

/// Typed, prefix-aware view over one store.
///
/// Applies the `-cache-` prefix (M3 hardening) to every key and (de)serializes
/// values as JSON (`Repository` = typed layer; `Store` = bytes).
#[derive(Clone)]
pub struct Repository {
    store: StoreRef,
}

impl Repository {
    /// Create a repository over an existing store.
    pub fn new(store: StoreRef) -> Self {
        Self { store }
    }

    /// Store this repository wraps.
    pub fn inner(&self) -> &StoreRef {
        &self.store
    }

    /// Prefixed key helper.
    fn key(&self, key: &str) -> String {
        format!("{}{}", crate::store::DEFAULT_PREFIX, key)
    }

    /// Fetch a typed value, or `None` on miss/expiry.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.store.get(&self.key(key)).await? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| CacheError::Serialization(e.to_string())),
            None => Ok(None),
        }
    }

    /// Store a typed value for `ttl` (0 = forever).
    pub async fn put<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let bytes =
            serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))?;
        self.store.put(&self.key(key), bytes, ttl).await
    }

    /// Store `value` only when `key` is absent; `Ok(false)` if it exists.
    ///
    /// Delegates to the store's atomic `put_if_absent` (SET NX EX / map-guard),
    /// so concurrent `add` calls cannot both win a missing key.
    pub async fn add<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<bool> {
        let bytes =
            serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))?;
        self.store.put_if_absent(&self.key(key), bytes, ttl).await
    }

    /// Compute-and-cache: return the cached value or store `producer`'s output.
    pub async fn remember<T, F, Fut>(&self, key: &str, ttl: Duration, producer: F) -> Result<T>
    where
        T: DeserializeOwned + Serialize + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T>> + Send + 'static,
    {
        if let Some(hit) = self.get::<T>(key).await? {
            return Ok(hit);
        }
        let value = producer().await?;
        self.put(key, &value, ttl).await?;
        Ok(value)
    }

    /// Cache `value` with no expiry.
    pub async fn forever<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        self.put(key, value, Duration::ZERO).await
    }

    /// Fetch and remove a typed value in one step.
    pub async fn pull<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let value = self.get::<T>(key).await?;
        self.store.forget(&self.key(key)).await?;
        Ok(value)
    }

    /// Whether a live value exists under `key`.
    pub async fn has(&self, key: &str) -> Result<bool> {
        Ok(self.store.get(&self.key(key)).await?.is_some())
    }

    /// Extend the TTL of `key` without reading it (`false` when missing).
    pub async fn touch(&self, key: &str, ttl: Duration) -> Result<bool> {
        self.store.touch(&self.key(key), ttl).await
    }

    /// Remove `key`.
    pub async fn forget(&self, key: &str) -> Result<()> {
        self.store.forget(&self.key(key)).await
    }

    /// Remove every key from the underlying store.
    pub async fn flush(&self) -> Result<()> {
        self.store.flush().await
    }

    /// Atomically add `n` to the numeric value under `key`.
    pub async fn increment(&self, key: &str, n: i64) -> Result<i64> {
        self.store.increment(&self.key(key), n).await
    }

    /// Atomically subtract `n` from the numeric value under `key`.
    pub async fn decrement(&self, key: &str, n: i64) -> Result<i64> {
        self.store.decrement(&self.key(key), n).await
    }
}

/// Backwards-compatible trait alias for code written against the doc shape.
///
/// The doc-shape `Repository` facade (remember/forever/pull/has/withContext/
/// store) is satisfied by `CacheManager` + `Repository`; this marker keeps the
/// `Store`+`Repository` naming discoverable.
#[async_trait]
pub trait RepositoryLike: Send + Sync {
    /// Look up a typed value.
    async fn get_typed<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>;
}

#[async_trait]
impl RepositoryLike for Repository {
    /// Look up a typed value through the prefixed key.
    async fn get_typed<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        self.get::<T>(key).await
    }
}
