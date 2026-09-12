/// Central queue routing registry — `Queue::route` + dispatch resolution.
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use crate::batch::BatchId;
use crate::driver::{
    record_failed, DatabaseDriver, QueueDriver, SyncDriver, DATABASE_CONNECTION, SYNC_CONNECTION,
};
use crate::error::{QueueError, Result};
use crate::job::{
    run_erased, DispatchHandle, ErasedJob, FailedJob, Job, JobId, JobOutcome, JobPayload,
};
use crate::metrics::Queues;

/// Lock-free post-boot state shared by every `Queue` facade.
static REGISTRY: OnceLock<RwLock<RegistryInner>> = OnceLock::new();

/// Interior of the central routing registry.
pub(crate) struct RegistryInner {
    /// Type-name -> resolved route for typed dispatch.
    routes: HashMap<&'static str, Route>,
    /// Named drivers available for dispatch.
    drivers: HashMap<String, Arc<dyn QueueDriver>>,
}

/// A resolved route: connection + queue for one job type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Connection (driver) name.
    pub connection: &'static str,
    /// Target queue name.
    pub queue: &'static str,
}

impl Route {
    /// Queue this route targets.
    pub fn queue_name(&self) -> &'static str {
        self.queue
    }

    /// Connection this route targets.
    pub fn connection_name(&self) -> &'static str {
        self.connection
    }
}

/// Returns the registry inner, initializing it with a sync driver on first use.
pub(crate) fn registry() -> &'static RwLock<RegistryInner> {
    REGISTRY.get_or_init(|| {
        let mut inner = RegistryInner {
            routes: HashMap::new(),
            drivers: HashMap::new(),
        };
        inner
            .drivers
            .insert(SYNC_CONNECTION.to_string(), Arc::new(SyncDriver::new()));
        RwLock::new(inner)
    })
}

/// Registers a route for `type_key` under `connection`/`queue`.
fn insert_route(type_key: &'static str, route: Route) -> Result<()> {
    let reg = registry();
    let mut guard = reg.write().map_err(QueueError::from)?;
    if guard.routes.contains_key(type_key) {
        return Err(QueueError::DuplicateRoute {
            type_name: type_key,
        });
    }
    guard.routes.insert(type_key, route);
    Ok(())
}

/// Build the serialized payload envelope for a dispatch.
fn to_payload(
    job: &str,
    payload: serde_json::Value,
    queue: String,
    connection: String,
    delay: Duration,
) -> Result<JobPayload> {
    let available_at = if delay.is_zero() {
        None
    } else {
        Some(chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default())
    };
    Ok(JobPayload {
        queue,
        connection,
        available_at,
        attempts: 1,
        id: None,
        job: Some(job.to_string()),
        payload,
    })
}

/// The `Queue` facade — typed dispatch, chain, batch, and metrics.
///
/// Mirrors the doc-shape `Queue` surface: `route` registers a job type to a
/// connection/queue once (boot-time), `dispatch` resolves the route and pushes
/// through the connection's driver, `onQueue`/`onConnection`/`delay` override
/// per dispatch via `DispatchHandle`, and the metric methods surface the Cloud
/// gauges per connection/queue (FS-M4-03).
pub struct Queue;

impl Queue {
    /// Register `J` to be dispatched to `queue` on `connection` by default.
    ///
    /// The route is keyed by `std::any::type_name::<J>()` (which matches the
    /// default `Job::queue_name`), so dispatch resolves the same key from the
    /// erased job. Duplicate registration for the same type returns
    /// `QueueError::DuplicateRoute`.
    pub fn route<J: Job + 'static>(connection: &'static str, queue: &'static str) -> Result<Route> {
        let route = Route { connection, queue };
        insert_route(std::any::type_name::<J>(), route.clone())?;
        Ok(route)
    }

    /// Register `J` on the default sync connection (registry-only testing).
    pub fn route_sync<J: Job + 'static>(queue: &'static str) -> Result<Route> {
        Self::route::<J>(SYNC_CONNECTION, queue)
    }

    /// Resolve the registered route for a job type name, or `Unrouted`.
    pub fn resolve(type_key: &str) -> Result<Route> {
        let reg = registry();
        let guard = reg.read().map_err(QueueError::from)?;
        guard
            .routes
            .get(type_key)
            .cloned()
            .ok_or_else(|| QueueError::Unrouted(type_key.to_string()))
    }

    /// Resolve the driver registered under `connection`.
    pub fn driver(connection: &str) -> Result<Arc<dyn QueueDriver>> {
        driver(connection)
    }

    /// Register a driver under `connection` (boot-time).
    pub fn register_driver(connection: impl Into<String>, driver: Arc<dyn QueueDriver>) {
        let reg = registry();
        if let Ok(mut guard) = reg.write() {
            guard.drivers.insert(connection.into(), driver);
        }
    }

    /// Register a route + driver in one call (convenience).
    pub fn connect<J: Job + 'static>(
        connection: &'static str,
        queue: &'static str,
        driver: Arc<dyn QueueDriver>,
    ) -> Result<Route> {
        let route = Self::route::<J>(connection, queue)?;
        Self::register_driver(connection, driver);
        Ok(route)
    }

    /// Enqueue an already-built dispatch handle through its resolved route.
    ///
    /// The `sync` connection executes the erased job body inline (deterministic
    /// test semantics) and records permanent failures — `Failed` and (on sync)
    /// `Retrying`, which cannot re-enqueue without a real worker — into the
    /// `failed_jobs` sink. `database`/`redis` connections push the serialized
    /// payload to their registered driver for a worker to drain.
    pub async fn dispatch_handle(handle: DispatchHandle) -> Result<JobId> {
        let routed = Self::resolve(handle.exec.type_key()).ok();
        let connection = match (&handle.connection, &routed) {
            (Some(c), _) => c.clone(),
            (None, Some(r)) => r.connection.to_string(),
            (None, None) => return Err(QueueError::Unrouted(handle.exec.type_key().to_string())),
        };
        let queue = match (&handle.queue, &routed) {
            (Some(q), _) => q.clone(),
            (None, Some(r)) => r.queue.to_string(),
            (None, None) => return Err(QueueError::Unrouted(handle.exec.type_key().to_string())),
        };
        if connection == SYNC_CONNECTION {
            // Inline sync dispatch: do NOT buffer the payload — the queue must
            // stay empty (pending_size == 0) once the job has run.
            return Self::execute_sync(&handle).await.map(|_| JobId::new());
        }
        let payload = to_payload(
            handle.exec.type_key(),
            handle.exec.as_json(),
            queue.clone(),
            connection.clone(),
            handle.delay,
        )?;
        driver(&connection)?.push(payload).await?;
        Ok(JobId::new())
    }

    /// Enqueue a typed job through its routed connection/queue.
    pub async fn dispatch<J: Job + 'static>(job: J) -> Result<JobId> {
        Self::dispatch_handle(job.dispatch()).await
    }

    /// Execute an erased job inline on the sync connection.
    ///
    /// Runs the body and returns the `JobOutcome`. `Failed` outcomes are
    /// dead-lettered; `Retrying` cannot be re-enqueued on sync (no worker to
    /// release it later), so it is dead-lettered too — a sync dispatch never
    /// silently drops a failed job. `Skipped` outcomes (missing-model
    /// suppression, FR-605) are recorded as diagnostics but never dead-lettered
    /// nor retried. The driver buffer is untouched.
    async fn execute_sync(handle: &DispatchHandle) -> Result<JobOutcome> {
        let outcome = run_erased(handle.exec.as_ref()).await;
        match outcome {
            JobOutcome::Failed => {
                record_failed(FailedJob::new(
                    SYNC_CONNECTION,
                    handle.exec.type_key(),
                    handle.exec.as_json(),
                    "JobError::MaxAttemptsExceeded",
                ));
                Ok(outcome)
            }
            JobOutcome::Retrying { attempt, .. } => {
                record_failed(FailedJob::new(
                    SYNC_CONNECTION,
                    handle.exec.type_key(),
                    handle.exec.as_json(),
                    format!("JobError::MaxAttemptsExceeded (sync retry {attempt} not schedulable)"),
                ));
                Ok(outcome)
            }
            JobOutcome::Succeeded | JobOutcome::Skipped => Ok(outcome),
        }
    }

    /// Execute a typed job inline on the sync connection.
    ///
    /// Returns the execution `JobOutcome`; `Failed` and `Retrying` outcomes are
    /// dead-lettered (see `execute_sync`) before the outcome is returned.
    pub async fn dispatch_sync<J: Job + 'static>(job: J) -> Result<JobOutcome> {
        let handle = job.dispatch().on_connection(SYNC_CONNECTION);
        Self::execute_sync(&handle).await
    }

    /// Run a chain of erased jobs inline on the sync connection.
    ///
    /// Executes sequentially and stops at the first non-succeeded outcome —
    /// `Failed` and `Retrying` are both dead-lettered (sync cannot schedule a
    /// later retry), so no chain step is ever silently dropped; a `Skipped`
    /// step stops the chain without dead-lettering (FR-605 suppression).
    pub async fn chain(jobs: Vec<Arc<dyn ErasedJob>>) -> Result<Vec<JobOutcome>> {
        let mut outcomes = Vec::with_capacity(jobs.len());
        for exec in jobs {
            let outcome = run_erased(exec.as_ref()).await;
            let stopped = !matches!(outcome, JobOutcome::Succeeded);
            if matches!(outcome, JobOutcome::Failed | JobOutcome::Retrying { .. }) {
                record_failed(FailedJob::new(
                    SYNC_CONNECTION,
                    exec.type_key(),
                    exec.as_json(),
                    "JobError::MaxAttemptsExceeded",
                ));
            }
            outcomes.push(outcome);
            if stopped {
                break;
            }
        }
        Ok(outcomes)
    }

    /// Dispatch a batch of erased jobs and return its `BatchId`.
    pub async fn batch(jobs: Vec<Arc<dyn ErasedJob>>) -> Result<BatchId> {
        for exec in jobs {
            DispatchHandle::new(exec).dispatch().await?;
        }
        Ok(BatchId::new())
    }

    /// Number of available jobs for a connection/queue pair.
    pub async fn pending_size(connection: &str, queue: &str) -> Result<usize> {
        driver(connection)?.pending_size(queue).await
    }

    /// Number of delayed jobs for a connection/queue pair.
    pub async fn delayed_size(connection: &str, queue: &str) -> Result<usize> {
        driver(connection)?.delayed_size(queue).await
    }

    /// Number of reserved jobs for a connection/queue pair.
    pub async fn reserved_size(connection: &str, queue: &str) -> Result<usize> {
        driver(connection)?.reserved_size(queue).await
    }

    /// UTC instant of the oldest pending job (RFC3339), `None` when empty.
    pub async fn creation_time_of_oldest_pending_job(
        connection: &str,
        queue: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        driver(connection)?
            .creation_time_of_oldest_pending_job(queue)
            .await
    }

    /// Aggregate metric view across all registered queues.
    pub async fn metrics() -> Queues {
        Queues::default()
    }
}

/// Resolve a driver by connection name.
fn driver(connection: &str) -> Result<Arc<dyn QueueDriver>> {
    let reg = registry();
    let guard = reg.read().map_err(QueueError::from)?;
    guard
        .drivers
        .get(connection)
        .cloned()
        .ok_or_else(|| QueueError::UnknownConnection(connection.to_string()))
}

/// Register a [`DatabaseDriver`] under the canonical `database` connection.
///
/// Boot-time helper: wraps `pool` in a driver and installs it through the same
/// registry path as [`Queue::register_driver`]. Returns the registered driver
/// so callers (e.g. a worker) can hold it directly.
pub fn register_database_driver(pool: rustasea_orm::DbPool) -> Arc<DatabaseDriver> {
    let driver = Arc::new(DatabaseDriver::new(pool));
    Queue::register_driver(DATABASE_CONNECTION, driver.clone());
    driver
}

/// Register a [`crate::driver::RedisDriver`] under the `redis` connection.
///
/// Boot-time helper gated on the `redis` feature; a `None`/empty `url` installs
/// a disabled driver so a worker can skip the connection without erroring.
#[cfg(feature = "redis")]
pub fn register_redis_driver(url: Option<&str>) -> Result<Arc<crate::driver::RedisDriver>> {
    let driver = Arc::new(crate::driver::RedisDriver::from_url(url)?);
    Queue::register_driver(crate::driver::REDIS_CONNECTION, driver.clone());
    Ok(driver)
}

/// Registry facade kept for source parity with the docs' `QueueRegistry`.
pub struct QueueRegistry;

impl QueueRegistry {
    /// Register `J` under `connection`/`queue` (see `Queue::route`).
    pub fn route<J: Job + 'static>(connection: &'static str, queue: &'static str) -> Result<Route> {
        Queue::route::<J>(connection, queue)
    }

    /// The default connection name used when none is provided.
    pub fn default_connection() -> &'static str {
        SYNC_CONNECTION
    }
}
