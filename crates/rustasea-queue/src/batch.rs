/// Batch dispatch — group jobs under one `BatchId` with aggregate accounting.
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::job::{ErasedJob, Job, JobId};

/// Identifier of a dispatched job batch (UUIDv4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BatchId(JobId);

impl BatchId {
    /// Create a fresh random batch id.
    pub fn new() -> Self {
        Self(JobId::new())
    }

    /// Access the underlying job id.
    pub fn as_job_id(&self) -> &JobId {
        &self.0
    }
}

impl Default for BatchId {
    /// A fresh random batch id (matches [`BatchId::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BatchId {
    /// Render the batch id as its hyphenated UUID string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Aggregate accounting for a dispatched batch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchHandle {
    /// Number of jobs in the batch.
    pub total_jobs: usize,
    /// Number of jobs still pending.
    pub pending_jobs: usize,
    /// Number of jobs that failed.
    pub failed_jobs: usize,
}

impl BatchHandle {
    /// Create accounting for a batch of `total` jobs.
    pub fn new(total_jobs: usize) -> Self {
        Self {
            total_jobs,
            pending_jobs: total_jobs,
            failed_jobs: 0,
        }
    }

    /// Record one completed (non-failed) job.
    pub fn complete_one(&mut self) {
        self.pending_jobs = self.pending_jobs.saturating_sub(1);
    }

    /// Record one failed job.
    pub fn fail_one(&mut self) {
        self.failed_jobs += 1;
        self.pending_jobs = self.pending_jobs.saturating_sub(1);
    }
}

/// Dispatch a batch of typed jobs and return a fresh `BatchId`.
///
/// Each job is dispatched individually through the central routing registry;
/// the `BatchId` aggregates them for `job_batches` accounting once the real
/// driver lands (S05-T03 follow-up).
pub async fn dispatch_batch<J>(jobs: Vec<J>) -> Result<BatchId>
where
    J: Job + 'static,
{
    for job in jobs {
        let _ = crate::registry::Queue::dispatch(job).await?;
    }
    Ok(BatchId::new())
}

/// Dispatch a pre-erased batch (each element already wrapped in an `Arc`).
pub async fn dispatch_batch_erased(jobs: Vec<Arc<dyn ErasedJob>>) -> Result<BatchId> {
    crate::registry::Queue::batch(jobs).await
}
