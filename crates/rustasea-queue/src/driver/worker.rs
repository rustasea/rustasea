//! Queue worker loop — pop, execute, ack, retry, or dead-letter.
//!
//! [`run_worker`] drains a set of queues through a [`QueueDriver`], resolving
//! each popped [`JobPayload`] to an erased handler via a type registry (or a
//! caller-supplied resolver). Execution goes through the existing
//! [`crate::job::run_erased`] timeout/retry path; success is `ack`ed, a
//! retryable failure is `release`d, and a permanent failure is dead-lettered.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use crate::driver::QueueDriver;
use crate::error::Result;
use crate::job::{run_erased, ConcreteJob, ErasedJob, FailedJob, Job, JobOutcome, JobPayload};

/// How long a worker blocks on `pop` before concluding a queue is drained.
const POP_TIMEOUT: Duration = Duration::from_millis(250);

/// Factory that rebuilds an erased handler from a serialized job body.
type HandlerFactory = fn(&serde_json::Value) -> Option<Arc<dyn ErasedJob>>;

/// Process-wide map from job type name to its deserializing handler factory.
static HANDLERS: OnceLock<RwLock<HashMap<&'static str, HandlerFactory>>> = OnceLock::new();

/// Access the handler registry, initializing it on first use.
fn handlers() -> &'static RwLock<HashMap<&'static str, HandlerFactory>> {
    HANDLERS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a handler for job type `J` under its `type_name` key.
///
/// A worker resolving a payload whose `job` equals `J`'s type name deserializes
/// the JSON body into `J` and runs it through the standard retry/timeout path.
/// Registering the same type twice replaces the prior factory.
pub fn register_job<J>()
where
    J: Job + serde::de::DeserializeOwned + 'static,
{
    register_job_handler(std::any::type_name::<J>(), |body| {
        let job: J = serde_json::from_value(body.clone()).ok()?;
        Some(Arc::new(ConcreteJob::new(job)))
    });
}

/// Register a handler factory for an explicit `type_key`.
///
/// The factory receives the deserialized JSON body and returns the erased job
/// to run, or `None` when the body is invalid. Use this for dynamic types whose
/// key is not a Rust `type_name`.
pub fn register_job_handler(
    type_key: &'static str,
    factory: HandlerFactory,
) {
    if let Ok(mut guard) = handlers().write() {
        guard.insert(type_key, factory);
    }
}

/// Resolver backed by the global handler registry.
///
/// Looks up `payload.job` in the registry; returns `None` for an unregistered
/// or unnamed job, which the worker treats as a permanent failure.
pub fn default_resolver() -> impl Fn(&JobPayload) -> Option<Arc<dyn ErasedJob>> + Send + Sync {
    |payload: &JobPayload| {
        let key = payload.job.as_deref()?;
        let guard = handlers().read().ok()?;
        let factory = guard.get(key)?;
        factory(&payload.payload)
    }
}

/// Drain `queues` through `driver`, running at most `max_jobs` jobs.
///
/// Uses [`default_resolver`] to resolve handlers from the global registry.
/// Returns the number of jobs processed. Stops after `max_jobs`, when every
/// queue is drained, or on a store error.
pub async fn run_worker(
    driver: Arc<dyn QueueDriver>,
    queues: Vec<String>,
    max_jobs: usize,
) -> Result<usize> {
    run_worker_with(driver, queues, max_jobs, default_resolver()).await
}

/// Drain `queues` through `driver` using a custom `resolver`.
///
/// `resolver` maps a popped payload to its erased handler; returning `None`
/// dead-letters the job with a `no handler registered` exception. The loop
/// round-robins the queues, stopping once a full cycle yields no job.
pub async fn run_worker_with<F>(
    driver: Arc<dyn QueueDriver>,
    queues: Vec<String>,
    max_jobs: usize,
    resolver: F,
) -> Result<usize>
where
    F: Fn(&JobPayload) -> Option<Arc<dyn ErasedJob>> + Send + Sync,
{
    if queues.is_empty() || max_jobs == 0 {
        return Ok(0);
    }

    let mut processed = 0usize;
    let mut cursor = 0usize;
    let mut misses = 0usize;
    while processed < max_jobs {
        let queue = queues[cursor % queues.len()].clone();
        cursor += 1;
        let Some(payload) = driver.pop(&queue, POP_TIMEOUT).await? else {
            // Stop only after a full cycle with no job (all queues drained).
            misses += 1;
            if misses >= queues.len() {
                break;
            }
            continue;
        };
        misses = 0;
        process_one(driver.as_ref(), &payload, &resolver).await?;
        processed += 1;
    }
    Ok(processed)
}

/// Build the dead-letter payload preserving the full job envelope.
///
/// Serializing the whole [`JobPayload`] (not just its inner body) keeps the job
/// type name and queue metadata, so `queue:retry` can rebuild a runnable
/// payload; falls back to the inner body if serialization fails.
fn dead_letter_payload(payload: &JobPayload) -> serde_json::Value {
    serde_json::to_value(payload).unwrap_or_else(|_| payload.payload.clone())
}

/// Execute one reserved payload and finalize it on the driver.
///
/// Success/skip `ack`s the reservation; a retryable failure `release`s it when
/// the attempt budget remains, otherwise it is dead-lettered and the
/// reservation removed.
async fn process_one<F>(
    driver: &dyn QueueDriver,
    payload: &JobPayload,
    resolver: &F,
) -> Result<()>
where
    F: Fn(&JobPayload) -> Option<Arc<dyn ErasedJob>> + Send + Sync,
{
    let Some(exec) = resolver(payload) else {
        driver
            .dead_letter(FailedJob::new(
                payload.connection.clone(),
                payload.queue.clone(),
                dead_letter_payload(payload),
                "no handler registered for job payload",
            ))
            .await?;
        driver.ack(payload).await?;
        return Ok(());
    };

    let tries = exec.tries();
    match run_erased(exec.as_ref()).await {
        JobOutcome::Succeeded | JobOutcome::Skipped => {
            driver.ack(payload).await?;
        }
        JobOutcome::Retrying { delay, .. } if payload.attempts < tries => {
            driver.release(payload, delay).await?;
        }
        JobOutcome::Failed | JobOutcome::Retrying { .. } => {
            driver
                .dead_letter(FailedJob::new(
                    payload.connection.clone(),
                    payload.queue.clone(),
                    dead_letter_payload(payload),
                    "JobError::MaxAttemptsExceeded",
                ))
                .await?;
            driver.ack(payload).await?;
        }
    }
    Ok(())
}
