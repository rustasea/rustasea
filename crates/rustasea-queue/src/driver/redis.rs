//! `redis` queue driver — Redis lists (immediate) + sorted sets (delayed).
//!
//! Compiled only behind the `redis` feature. Immediate jobs live on a Redis
//! list (`LPUSH`/`BRPOP`, FIFO); delayed jobs live on a sorted set scored by
//! availability epoch and are promoted to the list when due; reserved jobs live
//! on a second sorted set so `ack`/`release`/`dead_letter` can address them.
//! When no URL is configured the driver is *disabled*: `push` reports
//! `StoreUnavailable` and `pop` yields `None`, so callers skip it gracefully.

use std::time::Duration;

use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config, Pool, Runtime};

use crate::driver::QueueDriver;
use crate::error::{QueueError, Result};
use crate::job::{FailedJob, JobPayload};

/// Default key prefix for queue keys.
pub const DEFAULT_PREFIX: &str = "rustasea:queue:";
/// How long a popped job stays on the reserved set before it is abandoned.
const RESERVATION_TTL: Duration = Duration::from_secs(90);

/// Redis-backed queue driver (the `redis` connection).
///
/// A disabled instance (no configured URL) is inert: it never contacts a server
/// and lets `pop` return `None` so a worker skips the connection.
#[derive(Debug, Clone)]
pub struct RedisDriver {
    pool: Option<Pool>,
    prefix: String,
}

impl RedisDriver {
    /// Build a driver for `url`, or a disabled driver when `url` is empty.
    ///
    /// A pool is created eagerly; the first command surfaces a connection error
    /// as `StoreUnavailable`, so construction never fails on an absent server.
    pub fn from_url(url: Option<&str>) -> Result<Self> {
        match url.map(str::trim).filter(|u| !u.is_empty()) {
            Some(url) => {
                let pool = Config::from_url(url)
                    .create_pool(Some(Runtime::Tokio1))
                    .map_err(|e| QueueError::StoreUnavailable(e.to_string()))?;
                Ok(Self {
                    pool: Some(pool),
                    prefix: DEFAULT_PREFIX.to_string(),
                })
            }
            None => Ok(Self::disabled()),
        }
    }

    /// Build an inert driver that never contacts a Redis server.
    pub fn disabled() -> Self {
        Self {
            pool: None,
            prefix: DEFAULT_PREFIX.to_string(),
        }
    }

    /// Override the key prefix (defaults to [`DEFAULT_PREFIX`]).
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Whether this driver has a configured Redis pool.
    pub fn is_enabled(&self) -> bool {
        self.pool.is_some()
    }

    /// Borrow the pool, or `StoreUnavailable` when disabled.
    fn pool(&self) -> Result<&Pool> {
        self.pool
            .as_ref()
            .ok_or_else(|| QueueError::StoreUnavailable("redis connection not configured".into()))
    }

    /// The pending-list key for `queue`.
    fn list_key(&self, queue: &str) -> String {
        format!("{}{}", self.prefix, queue)
    }

    /// The delayed sorted-set key for `queue`.
    fn delayed_key(&self, queue: &str) -> String {
        format!("{}{}:delayed", self.prefix, queue)
    }

    /// The reserved sorted-set key for `queue`.
    fn reserved_key(&self, queue: &str) -> String {
        format!("{}{}:reserved", self.prefix, queue)
    }

    /// The shared dead-letter list key.
    fn failed_key(&self) -> String {
        format!("{}failed", self.prefix)
    }

    /// Promote every delayed member that is now due onto the pending list.
    async fn promote_due(&self, conn: &mut deadpool_redis::Connection, queue: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp() as f64;
        let delayed = self.delayed_key(queue);
        let list = self.list_key(queue);
        let due: Vec<String> = conn
            .zrangebyscore(&delayed, f64::NEG_INFINITY, now)
            .await
            .map_err(redis_error)?;
        for member in due {
            let removed: i64 = conn.zrem(&delayed, &member).await.map_err(redis_error)?;
            if removed > 0 {
                let _: i64 = conn.lpush(&list, &member).await.map_err(redis_error)?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl QueueDriver for RedisDriver {
    /// Push `payload` to the delayed set (future) or the pending list (now).
    async fn push(&self, payload: JobPayload) -> Result<()> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        let body = serde_json::to_string(&payload)
            .map_err(|e| QueueError::Serialization(e.to_string()))?;
        match payload.available_at {
            Some(at) if at > chrono::Utc::now() => {
                let score = at.timestamp() as f64;
                let _: i64 = conn
                    .zadd(self.delayed_key(&payload.queue), &body, score)
                    .await
                    .map_err(redis_error)?;
            }
            _ => {
                let _: i64 = conn
                    .lpush(self.list_key(&payload.queue), &body)
                    .await
                    .map_err(redis_error)?;
            }
        }
        Ok(())
    }

    /// Reserve the next job, promoting due delayed members first.
    async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<JobPayload>> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        self.promote_due(&mut conn, queue).await?;

        let seconds = timeout.as_secs_f64().max(0.1);
        let popped: Option<(String, String)> = conn
            .brpop(self.list_key(queue), seconds)
            .await
            .map_err(redis_error)?;
        let Some((_key, body)) = popped else {
            return Ok(None);
        };
        let payload: JobPayload =
            serde_json::from_str(&body).map_err(|e| QueueError::Serialization(e.to_string()))?;

        let expiry = (chrono::Utc::now()
            + chrono::Duration::from_std(RESERVATION_TTL).unwrap_or_default())
        .timestamp() as f64;
        let _: i64 = conn
            .zadd(self.reserved_key(queue), &body, expiry)
            .await
            .map_err(redis_error)?;
        Ok(Some(payload))
    }

    /// Count pending jobs (due delayed members plus list length).
    async fn pending_size(&self, queue: &str) -> Result<usize> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        let now = chrono::Utc::now().timestamp() as f64;
        let delayed: usize = conn
            .zcount(self.delayed_key(queue), f64::NEG_INFINITY, now)
            .await
            .map_err(redis_error)?;
        let listed: usize = conn.llen(self.list_key(queue)).await.map_err(redis_error)?;
        Ok(delayed + listed)
    }

    /// Count delayed jobs not yet due.
    async fn delayed_size(&self, queue: &str) -> Result<usize> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        let now = chrono::Utc::now().timestamp() as f64;
        let count: usize = conn
            .zcount(self.delayed_key(queue), now, f64::INFINITY)
            .await
            .map_err(redis_error)?;
        Ok(count)
    }

    /// Count reserved (in-flight) jobs.
    async fn reserved_size(&self, queue: &str) -> Result<usize> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        let count: usize = conn
            .zcard(self.reserved_key(queue))
            .await
            .map_err(redis_error)?;
        Ok(count)
    }

    /// Report no oldest pending instant (Redis lists carry no per-member time).
    async fn creation_time_of_oldest_pending_job(
        &self,
        _queue: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        Ok(None)
    }

    /// Remove the reservation on success.
    async fn ack(&self, payload: &JobPayload) -> Result<()> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        let body =
            serde_json::to_string(payload).map_err(|e| QueueError::Serialization(e.to_string()))?;
        let _: i64 = conn
            .zrem(self.reserved_key(&payload.queue), &body)
            .await
            .map_err(redis_error)?;
        Ok(())
    }

    /// Clear the old reservation and push the delayed retry.
    async fn release(&self, payload: &JobPayload, delay: Duration) -> Result<()> {
        self.ack(payload).await?;
        let mut next = payload.clone();
        next.id = None;
        next.attempts = payload.attempts.saturating_add(1);
        next.available_at = if delay.is_zero() {
            None
        } else {
            Some(chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default())
        };
        self.push(next).await
    }

    /// Append the dead letter and clear its reservation.
    async fn dead_letter(&self, failed: FailedJob) -> Result<()> {
        let pool = self.pool()?;
        let mut conn = pool.get().await.map_err(pool_error)?;
        let body =
            serde_json::to_string(&failed).map_err(|e| QueueError::Serialization(e.to_string()))?;
        let _: i64 = conn
            .lpush(self.failed_key(), body)
            .await
            .map_err(redis_error)?;
        Ok(())
    }
}

/// Map a Redis command error onto the queue's store error.
fn redis_error(error: deadpool_redis::redis::RedisError) -> QueueError {
    QueueError::StoreUnavailable(error.to_string())
}

/// Map a pool acquisition error onto the queue's store error.
fn pool_error(error: deadpool_redis::PoolError) -> QueueError {
    QueueError::StoreUnavailable(error.to_string())
}
