//! Real background queueing for agent runs (feature `queue`).
//!
//! Bridges the AI SDK agent runtime onto the workspace queue
//! ([`rustasea_queue`]): an [`AgentRunJob`] is a typed [`rustasea_queue::Job`]
//! that carries the agent name, prompt, and a stable run id. Because the job is
//! serializable, a worker rebuilds it from its JSON payload through the shared
//! handler registry and executes it via [`rustasea_queue::run_worker`]; the
//! outcome is stored in-process, keyed by run id, for the dispatcher to collect.
//!
//! Agents own live trait objects and are therefore not serializable; the job
//! references a *factory* registered with [`register_agent`] instead. A worker
//! resolving a job whose agent was never registered fails the job with a typed
//! `JobError::Exception` (never a panic).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rustasea_queue::{
    register_job, run_worker, Job, JobError, JobId, JobPayload, QueueDriver, QueueError,
};
use serde::{Deserialize, Serialize};

use crate::agent::{Agent, AgentRun};
use crate::error::{AiError, Result};

/// Connection name used by the global AI queue facade.
pub const AI_CONNECTION: &str = "ai";

/// Queue name agent runs are routed to by default.
pub const AI_QUEUE: &str = "agents";

/// Factory that rebuilds an [`Agent`] from its registered name.
pub type AgentFactory = Arc<dyn Fn() -> Agent + Send + Sync>;

/// Typed job that executes one agent run.
///
/// The body is fully serializable so a worker can rebuild it from the queue
/// payload; the live [`Agent`] is resolved from the factory registry by
/// [`AgentRunJob::agent`] at execution time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRunJob {
    /// Stable identifier for the run (result lookup key).
    pub run_id: String,
    /// Registered agent name (see [`register_agent`]).
    pub agent: String,
    /// Prompt handed to the agent.
    pub prompt: String,
}

impl AgentRunJob {
    /// Create a job with a fresh UUID run id.
    pub fn new(agent: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            run_id: JobId::new().to_string(),
            agent: agent.into(),
            prompt: prompt.into(),
        }
    }
}

#[async_trait::async_trait]
impl Job for AgentRunJob {
    /// Rebuild the agent, run the prompt, and store the outcome by run id.
    async fn handle(self) -> std::result::Result<(), JobError> {
        let factory = factory(&self.agent).ok_or_else(|| {
            JobError::Exception(format!(
                "no agent factory registered for '{}' (call register_agent first)",
                self.agent
            ))
        })?;
        let agent = factory();
        let run = agent.run(&self.prompt).await;
        store_result(&self.run_id, run);
        Ok(())
    }

    /// Stable queue/type name reported to the routing registry.
    fn queue_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// A failed agent run is not retried by default.
    fn tries(&self) -> u32 {
        1
    }
}

/// Process-wide registry of agent factories keyed by agent name.
static FACTORIES: OnceLock<Mutex<HashMap<String, AgentFactory>>> = OnceLock::new();

/// Process-wide sink of completed run outcomes keyed by run id.
static RESULTS: OnceLock<Mutex<HashMap<String, AgentRun>>> = OnceLock::new();

/// Access the factory registry, initializing it on first use.
fn factories() -> &'static Mutex<HashMap<String, AgentFactory>> {
    FACTORIES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Access the result sink, initializing it on first use.
fn results() -> &'static Mutex<HashMap<String, AgentRun>> {
    RESULTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register (or replace) the factory used to rebuild the agent named `name`.
///
/// The worker resolves [`AgentRunJob::agent`] through this registry, so every
/// agent name a job may reference must be registered before the worker drains.
pub fn register_agent<F>(name: impl Into<String>, factory: F)
where
    F: Fn() -> Agent + Send + Sync + 'static,
{
    if let Ok(mut guard) = factories().lock() {
        guard.insert(name.into(), Arc::new(factory));
    }
}

/// Look up the factory registered for `name`.
fn factory(name: &str) -> Option<AgentFactory> {
    factories().lock().ok()?.get(name).cloned()
}

/// Store a completed run outcome under `run_id`.
fn store_result(run_id: &str, run: AgentRun) {
    if let Ok(mut guard) = results().lock() {
        guard.insert(run_id.to_string(), run);
    }
}

/// Clone the stored outcome for `run_id`, if the worker has produced it.
pub fn result(run_id: &str) -> Option<AgentRun> {
    results().lock().ok()?.get(run_id).cloned()
}

/// Remove and return the stored outcome for `run_id`, if present.
pub fn take_result(run_id: &str) -> Option<AgentRun> {
    results().lock().ok()?.remove(run_id)
}

/// Map a queue-layer failure into the AI error space.
fn map_queue_error(error: QueueError) -> AiError {
    AiError::queue_unavailable(error.to_string())
}

/// Ensure `AgentRunJob` is routed on the global AI connection (idempotent).
fn ensure_route() -> Result<()> {
    match rustasea_queue::Queue::route::<AgentRunJob>(AI_CONNECTION, AI_QUEUE) {
        Ok(_) | Err(QueueError::DuplicateRoute { .. }) => Ok(()),
        Err(error) => Err(map_queue_error(error)),
    }
}

/// Enqueue an agent run on the global AI connection/queue.
///
/// Returns the run id used by [`result`]/[`take_result`]. The caller must have
/// registered a factory for `agent` and installed a driver under
/// [`AI_CONNECTION`]; otherwise enqueueing surfaces a typed
/// [`AiError::QueueUnavailable`].
pub async fn enqueue(agent: &str, prompt: &str) -> Result<String> {
    ensure_route()?;
    register_job::<AgentRunJob>();
    let job = AgentRunJob::new(agent, prompt);
    let run_id = job.run_id.clone();
    rustasea_queue::Queue::dispatch(job)
        .await
        .map_err(map_queue_error)?;
    Ok(run_id)
}

/// Enqueue an agent run directly onto an explicit `driver`/`queue`.
///
/// Bypasses the global routing registry — useful for wiring a specific driver
/// (e.g. an in-memory [`rustasea_queue::SyncDriver`]) and for deterministic
/// tests. The job is pushed as a real [`JobPayload`] so
/// [`drain`]/[`rustasea_queue::run_worker`] can execute it.
pub async fn enqueue_with(
    driver: Arc<dyn QueueDriver>,
    connection: impl Into<String>,
    queue: impl Into<String>,
    agent: &str,
    prompt: &str,
) -> Result<String> {
    register_job::<AgentRunJob>();
    let job = AgentRunJob::new(agent, prompt);
    let run_id = job.run_id.clone();
    let body = serde_json::to_value(&job)
        .map_err(|error| AiError::queue_unavailable(error.to_string()))?;
    let payload = JobPayload::new(
        queue,
        connection,
        Some(std::any::type_name::<AgentRunJob>().to_string()),
        body,
    );
    driver.push(payload).await.map_err(map_queue_error)?;
    Ok(run_id)
}

/// Drain up to `max_jobs` agent runs from `queue` through `driver`.
///
/// Uses the real [`rustasea_queue::run_worker`] loop with the shared handler
/// registry, so each popped [`AgentRunJob`] is rebuilt and executed. Returns
/// the number of jobs processed.
pub async fn drain(
    driver: Arc<dyn QueueDriver>,
    queue: impl Into<String>,
    max_jobs: usize,
) -> Result<usize> {
    register_job::<AgentRunJob>();
    run_worker(driver, vec![queue.into()], max_jobs)
        .await
        .map_err(map_queue_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Tool;
    use rustasea_queue::SyncDriver;
    use serde_json::{json, Value};

    /// Tool that upper-cases its `text` argument (agent observable output).
    struct UpperTool;

    #[async_trait::async_trait]
    impl Tool for UpperTool {
        fn name(&self) -> &'static str {
            "upper"
        }

        fn description(&self) -> &'static str {
            "Uppercases the text argument"
        }

        async fn call(&self, arguments: Value) -> Result<Value> {
            let text = arguments["text"].as_str().unwrap_or_default();
            Ok(json!({ "text": text.to_uppercase() }))
        }
    }

    #[tokio::test]
    async fn enqueued_agent_run_is_executed_by_the_worker() {
        register_agent("queue-test-upper", || Agent::new("echo").tool(UpperTool));

        let driver = Arc::new(SyncDriver::new());
        let run_id = enqueue_with(
            driver.clone(),
            "ai-test",
            "agents-test",
            "queue-test-upper",
            "tool:upper({\"text\":\"hi\"})",
        )
        .await
        .expect("enqueue");

        // The job is stored, not yet executed.
        assert_eq!(driver.pending_size("agents-test").await.unwrap(), 1);
        assert!(result(&run_id).is_none());

        // The real worker drains and executes it.
        let processed = drain(driver.clone(), "agents-test", 8)
            .await
            .expect("drain");
        assert_eq!(processed, 1);
        assert_eq!(driver.pending_size("agents-test").await.unwrap(), 0);

        let run = take_result(&run_id).expect("worker stored the outcome");
        assert_eq!(run.tool_results.len(), 1);
        assert_eq!(run.tool_results[0].0, "upper");
        assert_eq!(run.tool_results[0].1["text"], "HI");
    }

    #[tokio::test]
    async fn unregistered_agent_fails_the_job_without_panicking() {
        let driver = Arc::new(SyncDriver::new());
        let run_id = enqueue_with(
            driver.clone(),
            "ai-test",
            "agents-test-missing",
            "never-registered-agent",
            "hello",
        )
        .await
        .expect("enqueue");

        let processed = drain(driver, "agents-test-missing", 4)
            .await
            .expect("drain");
        assert_eq!(processed, 1);
        // The worker dead-letters the failed job; no result is stored.
        assert!(result(&run_id).is_none());
    }

    #[tokio::test]
    async fn enqueue_without_driver_is_a_typed_error() {
        // The global `ai` connection has no driver installed in this test
        // process, so dispatch must fail with a typed queue error.
        let error = enqueue("some-agent", "hello").await.unwrap_err();
        assert!(matches!(error, AiError::QueueUnavailable { .. }));
    }
}
