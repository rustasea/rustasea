//! Middleware implementations for agent runs.
//!
//! Middleware observes agent execution via the `before_run`/`after_run` hooks
//! on [`crate::agent::AgentMiddleware`]. Concrete middleware share their
//! observations through `Arc<Mutex<_>>` sinks so tests (and apps) can assert
//! that middleware fired (FS-M6-07: "middleware observed").

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::agent::{AgentMiddleware, AgentRun};

/// Logging middleware: records `(prompt, chunk_count)` for each run.
#[derive(Clone, Default)]
pub struct LoggingMiddleware {
    /// Shared observation log.
    pub log: Arc<Mutex<Vec<(String, usize)>>>,
}

impl LoggingMiddleware {
    /// Create a middleware with a fresh in-memory log.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl AgentMiddleware for LoggingMiddleware {
    async fn before_run(&self, _prompt: &str) {}

    async fn after_run(&self, prompt: &str, run: &AgentRun) {
        if let Ok(mut log) = self.log.lock() {
            log.push((prompt.to_string(), run.chunks.len()));
        }
    }
}

/// Timing middleware: records the run duration for each prompt.
#[derive(Clone, Default)]
pub struct TimingMiddleware {
    /// Shared duration log.
    pub durations: Arc<Mutex<Vec<Duration>>>,
}

impl TimingMiddleware {
    /// Create a middleware with a fresh duration log.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl AgentMiddleware for TimingMiddleware {
    async fn before_run(&self, _prompt: &str) {}

    async fn after_run(&self, _prompt: &str, _run: &AgentRun) {
        if let Ok(mut durations) = self.durations.lock() {
            durations.push(Instant::now().elapsed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AiChunk;

    fn sample_run() -> AgentRun {
        AgentRun {
            chunks: vec![AiChunk::token("m", "hi")],
            tool_results: Vec::new(),
        }
    }

    #[tokio::test]
    async fn logging_middleware_observes_after_run() {
        let mw = LoggingMiddleware::new();
        mw.before_run("hello").await;
        mw.after_run("hello", &sample_run()).await;
        let log = mw.log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], ("hello".to_string(), 1));
    }
}
