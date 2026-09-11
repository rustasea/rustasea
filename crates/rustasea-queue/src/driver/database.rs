//! `database` queue driver — reserves jobs from an ORM-backed `jobs` table.
//!
//! All SQL goes through the M2 [`rustasea_orm`] runtime API (`DbPool::fetch_json`
//! / `execute_bind` / `execution::transaction`) with `$n` positional binds, so
//! the driver works on SQLite/Postgres/MySQL without compile-time macros.
//! Timestamps are stored as RFC3339 UTC strings (portable across drivers) and
//! compared lexicographically; the payload column stores the serialized
//! [`JobPayload`] envelope.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use rustasea_orm::{DbPool, Value};

use crate::driver::QueueDriver;
use crate::error::{QueueError, Result};
use crate::job::{FailedJob, JobId, JobPayload};

/// Default `jobs` table name.
pub const JOBS_TABLE: &str = "jobs";
/// Default `failed_jobs` table name.
pub const FAILED_JOBS_TABLE: &str = "failed_jobs";

/// How long the pop poll sleeps between reservation attempts.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Database-backed queue driver (the `database` connection).
///
/// `push` inserts a `jobs` row; `pop` reserves the oldest available row in a
/// transaction and stamps its id into [`JobPayload::id`]; `ack` deletes the
/// reserved row, `release` re-schedules it, and `dead_letter` inserts into
/// `failed_jobs`. Safe against two workers claiming the same row because the
/// claim `UPDATE` is guarded by `reserved_at IS NULL`.
#[derive(Debug, Clone)]
pub struct DatabaseDriver {
    pool: DbPool,
    jobs_table: String,
    failed_table: String,
}

impl DatabaseDriver {
    /// Create a driver over `pool` using the default table names.
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            jobs_table: JOBS_TABLE.to_string(),
            failed_table: FAILED_JOBS_TABLE.to_string(),
        }
    }

    /// Create a driver over `pool` with custom table names.
    pub fn with_tables(
        pool: DbPool,
        jobs_table: impl Into<String>,
        failed_table: impl Into<String>,
    ) -> Self {
        Self {
            pool,
            jobs_table: jobs_table.into(),
            failed_table: failed_table.into(),
        }
    }

    /// Access the underlying pool (for callers that own the schema).
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// List persisted dead-letter rows, oldest first.
    pub async fn failed_jobs(&self) -> Result<Vec<FailedJob>> {
        let sql = format!(
            "SELECT id, connection, queue, payload, exception, failed_at \
             FROM {} ORDER BY failed_at ASC",
            self.failed_table
        );
        let rows = self.pool.fetch_json(&sql, &[]).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(record) = failed_from_row(&row)? else {
                continue;
            };
            out.push(record);
        }
        Ok(out)
    }

    /// Re-enqueue a dead-lettered job, deleting the row only after a push.
    ///
    /// The stored `payload` column holds the full serialized [`JobPayload`]
    /// envelope, so the job type key and inner body survive; it is re-pushed as
    /// a fresh immediate attempt and the `failed_jobs` row is deleted only
    /// after the push succeeds. A non-envelope payload (legacy row) falls back
    /// to rebuilding from the `connection`/`queue`/`payload` columns.
    pub async fn retry_failed(&self, id: JobId) -> Result<()> {
        let sql = format!(
            "SELECT id, connection, queue, payload, exception, failed_at \
             FROM {} WHERE id = $1",
            self.failed_table
        );
        let rows = self
            .pool
            .fetch_json(&sql, &[Value::Text(id.to_string())])
            .await?;
        let row = rows
            .first()
            .ok_or_else(|| QueueError::Empty(id.to_string()))?;
        let text = row.get("payload").and_then(|v| v.as_str()).unwrap_or("");
        let mut payload: JobPayload = match serde_json::from_str(text) {
            Ok(payload) => payload,
            Err(_) => JobPayload::new(
                string_field(row, "queue"),
                string_field(row, "connection"),
                None,
                row.get("payload").cloned().unwrap_or(serde_json::Value::Null),
            ),
        };
        payload.id = None;
        payload.attempts = 1;
        payload.available_at = None;
        self.push(payload).await?;
        let delete = format!("DELETE FROM {} WHERE id = $1", self.failed_table);
        self.pool
            .execute_bind(&delete, &[Value::Text(id.to_string())])
            .await?;
        Ok(())
    }

    /// Claim the oldest available row, returning its reservation or `None`.
    async fn claim(&self, queue: &str) -> Result<Option<JobPayload>> {
        let now = now_string();
        let select = format!(
            "SELECT id, payload, attempts FROM {} \
             WHERE queue = $1 AND reserved_at IS NULL AND available_at <= $2 \
             ORDER BY available_at ASC, created_at ASC, id ASC LIMIT 1",
            self.jobs_table
        );
        let update = format!(
            "UPDATE {} SET reserved_at = $1 WHERE id = $2 AND reserved_at IS NULL",
            self.jobs_table
        );
        let queue = queue.to_string();

        let claimed = rustasea_orm::transaction(&self.pool, |tx| {
            let select = select.clone();
            let update = update.clone();
            let queue = queue.clone();
            let now = now.clone();
            Box::pin(async move {
                let rows = tx
                    .fetch_json(&select, &[Value::Text(queue), Value::Text(now.clone())])
                    .await?;
                let Some(row) = rows.first() else {
                    return Ok(None);
                };
                let Some(id) = row.get("id").and_then(|v| v.as_str()) else {
                    return Ok(None);
                };
                let affected = tx
                    .execute_bind(&update, &[Value::Text(now), Value::Text(id.to_string())])
                    .await?;
                if affected == 0 {
                    // Another worker won the race; let the caller re-poll.
                    return Ok(None);
                }
                let payload = row
                    .get("payload")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                let attempts = row
                    .get("attempts")
                    .and_then(|v| v.as_i64())
                    .and_then(|v| u32::try_from(v).ok());
                Ok(payload.map(|payload| (id.to_string(), payload, attempts)))
            })
        })
        .await?;

        match claimed {
            Some((id, text, attempts)) => {
                let mut payload: JobPayload = serde_json::from_str(&text)
                    .map_err(|e| QueueError::Serialization(e.to_string()))?;
                // The `attempts` column is authoritative: `release` advances it
                // without rewriting the stored payload JSON.
                if let Some(attempts) = attempts {
                    payload.attempts = attempts;
                }
                payload.id = Some(id);
                Ok(Some(payload))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl QueueDriver for DatabaseDriver {
    /// Insert a `jobs` row for `payload` (immediate unless `available_at` set).
    async fn push(&self, payload: JobPayload) -> Result<()> {
        let available_at = payload
            .available_at
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Micros, true);
        let body = serde_json::to_string(&payload)
            .map_err(|e| QueueError::Serialization(e.to_string()))?;
        let sql = format!(
            "INSERT INTO {} (id, queue, payload, attempts, reserved_at, available_at, created_at) \
             VALUES ($1, $2, $3, $4, NULL, $5, $6)",
            self.jobs_table
        );
        self.pool
            .execute_bind(
                &sql,
                &[
                    Value::Text(uuid::Uuid::new_v4().to_string()),
                    Value::Text(payload.queue.clone()),
                    Value::Text(body),
                    Value::Int(i64::from(payload.attempts)),
                    Value::Text(available_at),
                    Value::Text(now_string()),
                ],
            )
            .await?;
        Ok(())
    }

    /// Reserve the next available job, polling until `timeout` elapses.
    async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<JobPayload>> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(payload) = self.claim(queue).await? {
                return Ok(Some(payload));
            }
            if std::time::Instant::now() >= deadline {
                return Ok(None);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// Number of available (not reserved, `available_at <= now`) jobs.
    async fn pending_size(&self, queue: &str) -> Result<usize> {
        let sql = format!(
            "SELECT COUNT(*) AS c FROM {} \
             WHERE queue = $1 AND reserved_at IS NULL AND available_at <= $2",
            self.jobs_table
        );
        self.count(&sql, queue).await
    }

    /// Number of delayed (not reserved, `available_at > now`) jobs.
    async fn delayed_size(&self, queue: &str) -> Result<usize> {
        let sql = format!(
            "SELECT COUNT(*) AS c FROM {} \
             WHERE queue = $1 AND reserved_at IS NULL AND available_at > $2",
            self.jobs_table
        );
        self.count(&sql, queue).await
    }

    /// Number of reserved (in-flight) jobs.
    async fn reserved_size(&self, queue: &str) -> Result<usize> {
        let sql = format!(
            "SELECT COUNT(*) AS c FROM {} WHERE queue = $1 AND reserved_at IS NOT NULL",
            self.jobs_table
        );
        let rows = self
            .pool
            .fetch_json(&sql, &[Value::Text(queue.to_string())])
            .await?;
        Ok(count_from_rows(&rows))
    }

    /// UTC instant of the oldest pending job, `None` when the queue is empty.
    async fn creation_time_of_oldest_pending_job(
        &self,
        queue: &str,
    ) -> Result<Option<DateTime<Utc>>> {
        let sql = format!(
            "SELECT MIN(available_at) AS c FROM {} \
             WHERE queue = $1 AND reserved_at IS NULL AND available_at <= $2",
            self.jobs_table
        );
        let rows = self
            .pool
            .fetch_json(
                &sql,
                &[Value::Text(queue.to_string()), Value::Text(now_string())],
            )
            .await?;
        let Some(value) = rows.first().and_then(|r| r.get("c")).and_then(|v| v.as_str()) else {
            return Ok(None);
        };
        Ok(DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| dt.with_timezone(&Utc)))
    }

    /// Delete the reserved row on success.
    async fn ack(&self, payload: &JobPayload) -> Result<()> {
        let Some(id) = payload.id.as_deref() else {
            return Ok(());
        };
        let sql = format!("DELETE FROM {} WHERE id = $1", self.jobs_table);
        self.pool
            .execute_bind(&sql, &[Value::Text(id.to_string())])
            .await?;
        Ok(())
    }

    /// Re-schedule the reserved row for a later retry (in place, no re-insert).
    async fn release(&self, payload: &JobPayload, delay: Duration) -> Result<()> {
        let Some(id) = payload.id.as_deref() else {
            return Ok(());
        };
        let next_attempt = payload.attempts.saturating_add(1);
        let available_at = (Utc::now()
            + chrono::Duration::from_std(delay).unwrap_or_default())
        .to_rfc3339_opts(SecondsFormat::Micros, true);
        let sql = format!(
            "UPDATE {} SET reserved_at = NULL, available_at = $1, attempts = $2 WHERE id = $3",
            self.jobs_table
        );
        self.pool
            .execute_bind(
                &sql,
                &[
                    Value::Text(available_at),
                    Value::Int(i64::from(next_attempt)),
                    Value::Text(id.to_string()),
                ],
            )
            .await?;
        Ok(())
    }

    /// Insert a `failed_jobs` row for a permanently failed job.
    async fn dead_letter(&self, failed: FailedJob) -> Result<()> {
        let body = serde_json::to_string(&failed.payload)
            .map_err(|e| QueueError::Serialization(e.to_string()))?;
        let sql = format!(
            "INSERT INTO {} (id, connection, queue, payload, exception, failed_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            self.failed_table
        );
        self.pool
            .execute_bind(
                &sql,
                &[
                    Value::Text(failed.id.to_string()),
                    Value::Text(failed.connection.clone()),
                    Value::Text(failed.queue.clone()),
                    Value::Text(body),
                    Value::Text(failed.exception.clone()),
                    Value::Text(
                        failed
                            .failed_at
                            .to_rfc3339_opts(SecondsFormat::Micros, true),
                    ),
                ],
            )
            .await?;
        Ok(())
    }
}

impl DatabaseDriver {
    /// Run a `COUNT(*)` query for `queue` plus the current instant.
    async fn count(&self, sql: &str, queue: &str) -> Result<usize> {
        let rows = self
            .pool
            .fetch_json(
                sql,
                &[Value::Text(queue.to_string()), Value::Text(now_string())],
            )
            .await?;
        Ok(count_from_rows(&rows))
    }
}

/// Current UTC instant as an RFC3339 micros string.
fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
}

/// Read a `COUNT(*) AS c` result, defaulting to zero.
fn count_from_rows(rows: &[serde_json::Value]) -> usize {
    rows.first()
        .and_then(|row| row.get("c"))
        .and_then(|value| value.as_i64())
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

/// Decode a [`FailedJob`] from a `failed_jobs` row.
fn failed_from_row(row: &serde_json::Value) -> Result<Option<FailedJob>> {
    let Some(id) = row.get("id").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    let Ok(uuid) = uuid::Uuid::parse_str(id) else {
        return Ok(None);
    };
    let failed_at = row
        .get("failed_at")
        .and_then(|v| v.as_str())
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let payload = row
        .get("payload")
        .and_then(|v| v.as_str())
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or(serde_json::Value::Null);
    Ok(Some(FailedJob {
        id: JobId::from_uuid(uuid),
        connection: string_field(row, "connection"),
        queue: string_field(row, "queue"),
        payload,
        exception: string_field(row, "exception"),
        failed_at,
    }))
}

/// Read a string field from a JSON row, defaulting to an empty string.
fn string_field(row: &serde_json::Value, key: &str) -> String {
    row.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}
