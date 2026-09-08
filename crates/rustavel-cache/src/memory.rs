/// Moka-like in-memory cache store with per-entry TTL.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::error::{CacheError, Result};
use crate::store::Store;

/// Single in-memory entry with an absolute expiry instant.
#[derive(Debug, Clone)]
struct Entry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}

impl Entry {
    /// Whether the entry is still inside its TTL window.
    fn alive(&self, now: Instant) -> bool {
        match self.expires_at {
            Some(at) => now < at,
            None => true,
        }
    }
}

/// TTL-aware concurrent in-memory store.
///
/// Mirrors the `moka` feature choice (architecture §6) with a lock-guarded map
/// — no external dependency while keeping the same semantics: per-entry TTL,
/// atomic increment/decrement, `touch` extending TTL in place, and full
/// `flush`. Intended for tests and single-node deployments.
#[derive(Debug, Default)]
pub struct MemoryStore {
    inner: Mutex<HashMap<String, Entry>>,
}

impl MemoryStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sweep expired entries and return the live map guard (internal).
    fn live(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        map.retain(|_, e| e.alive(now));
        map
    }
}

#[async_trait]
impl Store for MemoryStore {
    /// Fetch a live value, or `None` on miss/expiry.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let map = self.live();
        Ok(map.get(key).map(|e| e.value.clone()))
    }

    /// Store `value` under `key` for `ttl` (0 = forever).
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let expires_at = (!ttl.is_zero()).then(|| Instant::now() + ttl);
        map.insert(key.to_string(), Entry { value, expires_at });
        Ok(())
    }

    /// Insert `value` under `key` only when `key` is absent (map-atomic).
    async fn put_if_absent(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<bool> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();
        if map.get(key).is_some_and(|e| e.alive(now)) {
            return Ok(false);
        }
        let expires_at = (!ttl.is_zero()).then(|| now + ttl);
        map.insert(key.to_string(), Entry { value, expires_at });
        Ok(true)
    }

    /// Delete `key` only when it holds exactly `expected` (map-atomic).
    ///
    /// Guards against a stale `LockGuard` deleting a lease that expired and
    /// was re-acquired by a different owner (compare-and-delete).
    async fn compare_and_delete(&self, key: &str, expected: &[u8]) -> Result<bool> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        match map.get(key) {
            Some(entry) if entry.alive(Instant::now()) && entry.value == expected => {
                map.remove(key);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Extend the TTL of `key` in place without reading the value.
    async fn touch(&self, key: &str, ttl: Duration) -> Result<bool> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();
        match map.get_mut(key) {
            Some(entry) if entry.alive(now) => {
                entry.expires_at = (!ttl.is_zero()).then(|| now + ttl);
                Ok(true)
            }
            Some(_) | None => Ok(false),
        }
    }

    /// Remove `key`.
    async fn forget(&self, key: &str) -> Result<()> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        map.remove(key);
        Ok(())
    }

    /// Remove every entry.
    async fn flush(&self) -> Result<()> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        map.clear();
        Ok(())
    }

    /// Atomically add `n` to the numeric value under `key`.
    ///
    /// A missing key starts at zero; non-numeric values return
    /// `StoreUnavailable`.
    async fn increment(&self, key: &str, n: i64) -> Result<i64> {
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();
        let current = match map.get(key) {
            Some(e) if e.alive(now) => {
                let raw = String::from_utf8_lossy(&e.value);
                raw.parse::<i64>().map_err(|_| {
                    CacheError::StoreUnavailable(format!("key {key} holds a non-integer value"))
                })?
            }
            _ => 0,
        };
        let next = current + n;
        map.insert(
            key.to_string(),
            Entry {
                value: next.to_string().into_bytes(),
                expires_at: None,
            },
        );
        Ok(next)
    }
}
