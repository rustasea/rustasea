/// Queue drivers — `sync` inline, `database` (sqlx/ORM), and `redis` (feature).
///
/// [`QueueDriver`] is the storage contract shared by every connection: the
/// `sync` buffer for inline/execution tests, the ORM-backed `database` driver,
/// and the `redis` list/sorted-set driver. Driver submodules live in
/// `src/driver/` and are re-exported here.
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use crate::error::{QueueError, Result};
use crate::job::{FailedJob, JobId, JobPayload};

mod database;
#[cfg(feature = "redis")]
mod redis;
mod worker;

pub use database::DatabaseDriver;
#[cfg(feature = "redis")]
pub use redis::RedisDriver;
pub use worker::{
    default_resolver, register_job, register_job_handler, run_worker, run_worker_with,
};

/// Canonical name of the inline `sync` connection.
pub const SYNC_CONNECTION: &str = "sync";
/// Canonical name of the `database` connection.
pub const DATABASE_CONNECTION: &str = "database";
/// Canonical name of the `redis` connection.
pub const REDIS_CONNECTION: &str = "redis";
/// Reserved driver name for the `database` connection.
pub const DATABASE_DRIVER: &str = "database";
/// Reserved driver name for the `redis` connection.
pub const REDIS_DRIVER: &str = "redis";

/// A queue backend capable of storing and reserving serialized jobs.
///
/// The trait exposes the Cloud metric surface (`pendingSize`,
/// `delayedSize`, `reservedSize`, `creationTimeOfOldestPendingJob`) alongside
/// `push`/`pop` so one driver object serves both dispatch and observability
/// (FS-M4-03). Drivers must be `Send + Sync` so they can live behind `Arc`.
///
/// `ack`/`release`/`dead_letter` complete the worker lifecycle: `pop` reserves
/// a job, `ack` finalizes success, `release` re-enqueues for a retry, and
/// `dead_letter` records a permanent failure. Each has a default so a driver
/// only overrides the lifecycle steps it can do better.
#[async_trait]
pub trait QueueDriver: Send + Sync {
    /// Enqueue a serialized job payload.
    async fn push(&self, payload: JobPayload) -> Result<()>;

    /// Reserve the next available job, blocking up to `timeout`.
    async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<JobPayload>>;

    /// Number of available (pending) jobs for `queue`.
    async fn pending_size(&self, queue: &str) -> Result<usize>;

    /// Number of delayed (not yet available) jobs for `queue`.
    async fn delayed_size(&self, queue: &str) -> Result<usize>;

    /// Number of reserved (in-flight) jobs for `queue`.
    async fn reserved_size(&self, queue: &str) -> Result<usize>;

    /// UTC instant of the oldest pending job, `None` when the queue is empty.
    async fn creation_time_of_oldest_pending_job(
        &self,
        queue: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>>;

    /// Acknowledge a successfully executed job, removing its reservation.
    ///
    /// Defaults to a no-op for drivers whose `pop` already consumed the job
    /// (sync buffer, redis list).
    async fn ack(&self, payload: &JobPayload) -> Result<()> {
        let _ = payload;
        Ok(())
    }

    /// Re-enqueue a reserved job for a later retry after `delay`.
    ///
    /// The default bumps the attempt count and pushes the payload again; the
    /// database driver overrides it to update its reserved row in place.
    async fn release(&self, payload: &JobPayload, delay: Duration) -> Result<()> {
        let mut next = payload.clone();
        next.attempts = payload.attempts.saturating_add(1);
        next.available_at = if delay.is_zero() {
            None
        } else {
            Some(chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default())
        };
        self.push(next).await
    }

    /// Record a permanently failed job in the dead-letter store.
    ///
    /// Defaults to the process-local `failed_jobs` sink; the database driver
    /// overrides it to insert a `failed_jobs` row.
    async fn dead_letter(&self, failed: FailedJob) -> Result<()> {
        record_failed(failed);
        Ok(())
    }
}

/// In-memory queue buffer — the `sync` connection driver.
///
/// `push` appends the serialized payload to the back of the buffer and `pop`
/// takes from the front (FIFO), so a `pop`-driven worker drains in enqueue
/// order. The `Queue` facade executes sync dispatches inline and never pushes
/// to this buffer, keeping `pending_size("sync", ..)` at zero after a sync
/// dispatch; the buffer serves explicit `pop`-based workers and assertions.
#[derive(Debug, Default)]
pub struct SyncDriver {
    pending: Mutex<VecDeque<JobPayload>>,
}

impl SyncDriver {
    /// Create an empty sync driver.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl QueueDriver for SyncDriver {
    /// Buffer a serialized job payload at the back of the queue.
    async fn push(&self, payload: JobPayload) -> Result<()> {
        let mut buf = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        buf.push_back(payload);
        Ok(())
    }

    /// Pop the front of the buffered queue (FIFO) for `pop`-based workers.
    async fn pop(&self, _queue: &str, _timeout: Duration) -> Result<Option<JobPayload>> {
        let mut buf = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        Ok(buf.pop_front())
    }

    /// Number of buffered, not-yet-consumed payloads.
    async fn pending_size(&self, _queue: &str) -> Result<usize> {
        let buf = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        Ok(buf.len())
    }

    /// Sync driver never delays jobs.
    async fn delayed_size(&self, _queue: &str) -> Result<usize> {
        Ok(0)
    }

    /// Sync driver reserves nothing.
    async fn reserved_size(&self, _queue: &str) -> Result<usize> {
        Ok(0)
    }

    /// Report no oldest pending job (execution is inline).
    async fn creation_time_of_oldest_pending_job(
        &self,
        _queue: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        Ok(None)
    }
}

/// Placeholder dead-letter sink shared by the driver stubs.
///
/// The database driver persists dead letters to a real `failed_jobs` table;
/// this in-memory sink backs the sync/redis drivers and keeps the contract
/// testable without a server.
static FAILED: std::sync::OnceLock<Mutex<Vec<FailedJob>>> = std::sync::OnceLock::new();

/// Record a dead-letter entry for a permanently failed job.
pub(crate) fn record_failed(job: FailedJob) {
    let list = FAILED.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut guard) = list.lock() {
        guard.push(job);
    }
}

/// Snapshot the current dead-letter entries.
pub fn failed_jobs() -> Vec<FailedJob> {
    let Some(list) = FAILED.get() else {
        return Vec::new();
    };
    let Ok(guard) = list.lock() else {
        return Vec::new();
    };
    guard.clone()
}

/// Clear a dead-lettered entry by id (`queue:retry`) from the in-memory sink.
///
/// The sync/redis sinks hold no re-executable body registry, so the entry is
/// dropped. The database driver's `retry_failed` re-enqueues the stored payload
/// and clears the row only after a successful push.
pub async fn retry_failed(id: JobId) -> Result<()> {
    let list = FAILED.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = list.lock().map_err(|_| QueueError::RegistryPoisoned)?;
    let idx = guard
        .iter()
        .position(|f| f.id == id)
        .ok_or_else(|| QueueError::Empty(id.to_string()))?;
    guard.remove(idx);
    Ok(())
}
