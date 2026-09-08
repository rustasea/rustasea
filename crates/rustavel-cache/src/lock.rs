/// Atomic distributed lock with `get`/`block`/`release`.
use std::sync::Arc;
use std::time::Duration;

use crate::error::{CacheError, LockError, Result};
use crate::store::Store;

/// A named mutual-exclusion lease over a cache store.
///
/// Acquisition is atomic (`put_if_absent` — `SET NX EX` on Redis, an
/// insert-if-absent entry under one guard in memory); the TTL acts as a lease
/// so a crashed holder cannot deadlock the section (NFR-Rel-03). `get` is
/// non-blocking, `block` polls until the wait window elapses. The guard stores
/// the owner token it acquired, so `release` is a compare-and-delete — a
/// holder whose lease expired and was re-acquired elsewhere can never delete
/// the new owner's lock.
pub struct Lock {
    /// Store the lock lives on.
    store: Arc<dyn Store>,
    /// Lock key.
    key: String,
    /// Lease duration.
    ttl: Duration,
}

impl Lock {
    /// Create a lock over `store` for `key` with lease `ttl`.
    pub fn new(store: Arc<dyn Store>, key: impl Into<String>, ttl: Duration) -> Self {
        Self {
            store,
            key: key.into(),
            ttl,
        }
    }

    /// Key this lock guards.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Lease duration of this lock.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Attempt one atomic acquisition, returning a guard when it succeeds.
    pub async fn get(&self) -> Result<Option<LockGuard>> {
        let token = owner_token();
        match self
            .store
            .put_if_absent(&self.key, token.as_bytes().to_vec(), self.ttl)
            .await
        {
            Ok(true) => Ok(Some(LockGuard::new(
                self.store.clone(),
                self.key.clone(),
                token,
            ))),
            Ok(false) => Ok(None),
            Err(CacheError::StoreUnavailable(m)) => Err(LockError::StoreUnavailable(m).into()),
            Err(e) => Err(e),
        }
    }

    /// Block until acquired or `wait` elapses (`LockError::AlreadyHeld`).
    ///
    /// Polls every 50ms. A store failure aborts immediately rather than
    /// spinning for the full window (Chaos: Redis down returns typed error).
    pub async fn block(&self, wait: Duration) -> Result<LockGuard> {
        let started = std::time::Instant::now();
        loop {
            if let Some(guard) = self.get().await? {
                return Ok(guard);
            }
            if started.elapsed() >= wait {
                return Err(LockError::AlreadyHeld {
                    key: self.key.clone(),
                    waited: wait,
                }
                .into());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

/// Mint a fresh random owner token for one lock acquisition.
fn owner_token() -> String {
    format!("owner-{}", uuid::Uuid::new_v4())
}

/// RAII lock guard — releases the lease on drop.
///
/// Stores the owner token it acquired; release is a compare-and-delete against
/// that token, so a guard whose lease expired (and was re-acquired by another
/// owner) releases nothing instead of deleting the new owner's lock.
pub struct LockGuard {
    store: Arc<dyn Store>,
    key: String,
    token: Vec<u8>,
    released: bool,
}

impl LockGuard {
    /// Create a guard that will release `key` on drop.
    fn new(store: Arc<dyn Store>, key: String, token: String) -> Self {
        Self {
            store,
            key,
            token: token.into_bytes(),
            released: false,
        }
    }

    /// Key this guard holds.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Explicitly release the lease (idempotent).
    ///
    /// Removes the lock only when it still holds this guard's owner token;
    /// a lease that expired and was re-acquired stays untouched.
    pub async fn release(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }
        self.released = true;
        let _ = self
            .store
            .compare_and_delete(&self.key, &self.token)
            .await?;
        Ok(())
    }
}

impl Drop for LockGuard {
    /// Best-effort asynchronous compare-and-delete release (fire-and-forget).
    fn drop(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        let store = self.store.clone();
        let key = self.key.clone();
        let token = self.token.clone();
        std::mem::drop(tokio::task::spawn(async move {
            let _ = store.compare_and_delete(&key, &token).await;
        }));
    }
}

impl std::fmt::Debug for Lock {
    /// Debug representation without the erased store.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lock")
            .field("key", &self.key)
            .field("ttl", &self.ttl)
            .finish()
    }
}

impl std::fmt::Debug for LockGuard {
    /// Debug representation without the erased store.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockGuard")
            .field("key", &self.key)
            .field("released", &self.released)
            .finish()
    }
}
