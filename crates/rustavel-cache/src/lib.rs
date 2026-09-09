//! Rustavel Cache — Store/Repository traits, memory + redis stores, Lock.
//!
//! Sprint 05 (M4) scope per sprint-05.md S05-T04: driver-agnostic `Store` +
//! `Repository` (`get`/`put`/`add`/`remember`/`forever`/`forget`/`flush`/
//! `increment`/`decrement`/`pull`/`has`), the TTL-extending `touch()` with a
//! default `Ok(false)` so custom drivers keep compiling, the Laravel-style
//! context-scoped `with_context` (key prefixing per tenant/scope), a
//! moka-like in-memory store, a redis wiring stub (no real Redis required
//! this sprint), store isolation (`redis` vs `memory`), and an atomic `Lock`
//! with `get`/`block`/`release`.

pub mod error;
pub mod lock;
pub mod memory;
pub mod redis;
pub mod repository;
pub mod store;

pub use error::{CacheError, LockError, Result};
pub use lock::{Lock, LockGuard};
pub use memory::MemoryStore;
pub use redis::RedisStore;
pub use repository::{
    CacheManager, Repository, RepositoryLike, StoreRef, MEMORY_STORE, REDIS_STORE,
};
pub use store::{Store, DEFAULT_PREFIX};

/// Canonical prefix applied to every cache key (`-cache-` hardening, M3).
pub const CACHE_PREFIX: &str = "-cache-";
