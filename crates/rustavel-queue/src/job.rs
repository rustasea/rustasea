/// Typed job definition, retry-aware execution, and dispatch handles.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::JobError;
use crate::Result;

/// Unique identifier for an enqueued job (UUIDv4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(Uuid);

impl JobId {
    /// Create a fresh random job id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for JobId {
    /// Create a fresh random job id.
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    /// Render the job id as its hyphenated UUID string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Outcome of a single job execution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutcome {
    /// `handle` returned `Ok`.
    Succeeded,
    /// `handle` returned `Err` and the retry budget is exhausted.
    Failed,
    /// `handle` returned `Err`; the job will be retried after `delay`.
    Retrying { attempt: u32, delay: Duration },
}

/// Serialized job envelope stored on a queue.
///
/// `queue`/`connection`/`available_at` are routing and timing metadata;
/// `payload` is the JSON body of the typed job (contract
/// `job-payload.schema.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPayload {
    /// Target queue (resolved from `Queue::route` or `on_queue` override).
    pub queue: String,
    /// Target connection/driver name.
    pub connection: String,
    /// UTC instant after which the job may run (delay support).
    #[serde(default)]
    pub available_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Attempts so far (initial dispatch counts as one).
    #[serde(default)]
    pub attempts: u32,
    /// JSON body of the typed job.
    pub payload: serde_json::Value,
}

/// A typed queue job.
///
/// Implementors declare their own serializable payload struct (no domain
/// `Any` — C-03) and provide `handle(self)`. `handle` consumes the job, so the
/// trait is not dyn-compatible by design; dispatch erases execution into a
/// `ConcreteJob` at the call site, where the concrete type is still known.
///
/// Retry/timing behaviour is declared per type via `#[tries]`/`#[backoff]`/
/// `#[timeout]` (macro wiring lands with the M5 CLI generators); the trait
/// accessors below mirror those declarations so drivers execute identical
/// semantics today.
#[async_trait]
pub trait Job: Serialize + Send + Sync + 'static {
    /// Execute the job body. Return `Err(JobError::Exception(..))` on domain
    /// failure; `JobError::Timeout`/`MaxAttemptsExceeded` are worker-produced.
    async fn handle(self) -> std::result::Result<(), JobError>;

    /// Stable queue/type name reported to the routing registry.
    fn queue_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Number of attempts allowed (initial run counts as one).
    fn tries(&self) -> u32 {
        1
    }

    /// Base delay before the first retry; later retries double.
    fn backoff(&self) -> Duration {
        Duration::ZERO
    }

    /// Per-attempt timeout; `Duration::ZERO` means no timeout.
    fn timeout(&self) -> Duration {
        Duration::ZERO
    }

    /// Whether attempt `attempt` (1-based, already failed) is retried.
    ///
    /// Default honours the `tries` budget; opt-in `ShouldRetry` refines it.
    fn should_retry(&self, attempt: u32, err: &JobError) -> bool {
        let _ = err;
        attempt < self.tries()
    }

    /// Hook invoked when the per-job timeout elapses (`#[failOnTimeout]`).
    fn on_timeout(&self) {}

    /// Type identity used by the routing registry.
    fn type_id(&self) -> std::any::TypeId
    where
        Self: 'static,
    {
        std::any::TypeId::of::<Self>()
    }

    /// Start an async dispatch for this job (builder chain).
    ///
    /// `ProcessPodcast { id: 42 }.dispatch().await` routes through the central
    /// registry; `.on_queue("urgent")`/`.on_connection(..)`/`.delay(..)`
    /// override the routed defaults per dispatch.
    fn dispatch(self) -> DispatchHandle
    where
        Self: Sized,
    {
        DispatchHandle::new(Arc::new(ConcreteJob::new(self)))
    }
}

/// Type-erased executable job captured at dispatch time.
///
/// The concrete job body is stored behind `Mutex<Option<..>>` so `run` can
/// consume it exactly once; retry/timing metadata is copied from the concrete
/// value when the handle is built, keeping every method object-safe.
#[async_trait]
pub trait ErasedJob: Send + Sync {
    /// Stable queue/type name.
    fn type_key(&self) -> &'static str;

    /// Serialize the captured body into its JSON payload.
    fn as_json(&self) -> serde_json::Value;

    /// Maximum attempts.
    fn tries(&self) -> u32;

    /// Base retry backoff.
    fn backoff(&self) -> Duration;

    /// Per-attempt timeout (`ZERO` = none).
    fn timeout(&self) -> Duration;

    /// Retry decision for a failed attempt.
    fn should_retry(&self, attempt: u32, err: &JobError) -> bool;

    /// Timeout hook.
    fn on_timeout(&self);

    /// Execute the captured body once (with timeout + retry policy).
    async fn run(&self) -> JobOutcome;
}

/// Concrete erased job wrapping a typed `Job`.
pub struct ConcreteJob<J: Job> {
    body: Mutex<Option<J>>,
    key: &'static str,
    tries: u32,
    backoff: Duration,
    timeout: Duration,
}

impl<J: Job> ConcreteJob<J> {
    /// Capture a concrete job plus its retry/timing policy.
    pub fn new(job: J) -> Self {
        let key = job.queue_name();
        let tries = job.tries();
        let backoff = job.backoff();
        let timeout = job.timeout();
        Self {
            body: Mutex::new(Some(job)),
            key,
            tries,
            backoff,
            timeout,
        }
    }
}

#[async_trait]
impl<J: Job> ErasedJob for ConcreteJob<J> {
    /// Stable queue/type name.
    fn type_key(&self) -> &'static str {
        self.key
    }

    /// Serialize the captured body into its JSON payload.
    fn as_json(&self) -> serde_json::Value {
        let guard = self.body.lock().unwrap_or_else(|p| p.into_inner());
        guard
            .as_ref()
            .and_then(|j| serde_json::to_value(j).ok())
            .unwrap_or(serde_json::Value::Null)
    }

    /// Maximum attempts captured at dispatch.
    fn tries(&self) -> u32 {
        self.tries
    }

    /// Base backoff captured at dispatch.
    fn backoff(&self) -> Duration {
        self.backoff
    }

    /// Timeout captured at dispatch.
    fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Retry decision via the concrete job's contract.
    fn should_retry(&self, attempt: u32, err: &JobError) -> bool {
        let guard = self.body.lock().unwrap_or_else(|p| p.into_inner());
        match guard.as_ref() {
            Some(job) => job.should_retry(attempt, err),
            None => attempt < self.tries,
        }
    }

    /// Forward the timeout hook to the concrete job.
    fn on_timeout(&self) {
        let guard = self.body.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(job) = guard.as_ref() {
            job.on_timeout();
        }
    }

    /// Consume the body and execute it under the retry policy.
    async fn run(&self) -> JobOutcome {
        let body = self
            .body
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
            .expect("erased job executed twice");
        let timeout = self.timeout;
        let deadline = if timeout.is_zero() {
            None
        } else {
            Some(std::time::Instant::now() + timeout)
        };

        let result = match deadline {
            None => body.handle().await,
            Some(deadline) => match tokio::time::timeout_at(deadline.into(), body.handle()).await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(_) => {
                    self.on_timeout();
                    Err(JobError::Timeout)
                }
            },
        };

        match result {
            Ok(()) => JobOutcome::Succeeded,
            Err(e) if self.should_retry(1, &e) && self.tries > 1 => JobOutcome::Retrying {
                attempt: 2,
                delay: retry_delay(self.backoff, 1),
            },
            Err(_) => JobOutcome::Failed,
        }
    }
}

/// Compute the delay before retry `attempt + 1` (exponential on the base).
fn retry_delay(base: Duration, failed_attempt: u32) -> Duration {
    if base.is_zero() || failed_attempt <= 1 {
        return base;
    }
    let factor = 1u32 << (failed_attempt - 1).min(30);
    base.saturating_mul(factor)
}

/// Chainable per-dispatch overrides returned by `Job::dispatch`.
///
/// Every override mutates a shared plan; the terminal `await` performs the
/// enqueue through the resolved connection's driver.
#[derive(Clone)]
pub struct DispatchHandle {
    pub(crate) exec: Arc<dyn ErasedJob>,
    pub(crate) queue: Option<String>,
    pub(crate) connection: Option<String>,
    pub(crate) delay: Duration,
}

impl DispatchHandle {
    /// Create a dispatch handle for an erased job with no overrides yet.
    pub fn new(exec: Arc<dyn ErasedJob>) -> Self {
        Self {
            exec,
            queue: None,
            connection: None,
            delay: Duration::ZERO,
        }
    }

    /// Route this single dispatch to `queue`, overriding the routed queue.
    pub fn on_queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = Some(queue.into());
        self
    }

    /// Route this single dispatch to `connection`, overriding the routed one.
    pub fn on_connection(mut self, connection: impl Into<String>) -> Self {
        self.connection = Some(connection.into());
        self
    }

    /// Delay this single dispatch by `delay` (sets `available_at`).
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Enqueue the job through the resolved connection and queue.
    pub async fn dispatch(self) -> Result<JobId> {
        crate::registry::Queue::dispatch_handle(self).await
    }
}

/// Dead-letter record for a permanently failed job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedJob {
    /// Dead-letter identifier.
    pub id: JobId,
    /// Connection name the job ran on.
    pub connection: String,
    /// Queue the job was pulled from.
    pub queue: String,
    /// Serialized job payload.
    pub payload: serde_json::Value,
    /// Human-readable exception (e.g. `JobError::Exception(msg)`).
    pub exception: String,
    /// UTC instant the job was dead-lettered.
    pub failed_at: chrono::DateTime<chrono::Utc>,
}

impl FailedJob {
    /// Convenience constructor for a dead-letter record.
    pub fn new(
        connection: impl Into<String>,
        queue: impl Into<String>,
        payload: serde_json::Value,
        exception: impl Into<String>,
    ) -> Self {
        Self {
            id: JobId::new(),
            connection: connection.into(),
            queue: queue.into(),
            payload,
            exception: exception.into(),
            failed_at: chrono::Utc::now(),
        }
    }
}

/// Run an erased job once, returning its outcome.
pub async fn run_erased(exec: &dyn ErasedJob) -> JobOutcome {
    exec.run().await
}
