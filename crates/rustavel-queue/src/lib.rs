//! Rustavel Queue — typed jobs, central routing, drivers, chain/batch, metrics.
//!
//! Sprint 05 (M4) scope per sprint-05.md S05-T01..T03: the typed `Job` trait
//! with retry contracts, the `Queue::route::<Job>` registry with per-dispatch
//! `onQueue`/`onConnection` overrides, `sync`/`database`/`redis` drivers (the
//! two remote drivers are wiring stubs — no real Redis/DB is required by this
//! sprint), `chain`/`batch` dispatch, the `failed_jobs` dead-letter surface and
//! the Cloud queue metric shapes.

pub mod batch;
pub mod driver;
pub mod error;
pub mod job;
pub mod metrics;
pub mod registry;
pub mod retry;

pub use async_trait::async_trait;
pub use batch::{dispatch_batch, BatchHandle, BatchId};
pub use driver::{
    failed_jobs, retry_failed, QueueDriver, SyncDriver, DATABASE_CONNECTION, DATABASE_DRIVER,
    REDIS_CONNECTION, REDIS_DRIVER, SYNC_CONNECTION,
};
pub use error::{JobError, QueueError, Result};
pub use job::{
    run_erased, ConcreteJob, DispatchHandle, ErasedJob, FailedJob, Job, JobId, JobOutcome,
    JobPayload,
};
pub use metrics::{JobQueueMetrics, QueueMetrics, Queues};
pub use registry::{Queue, QueueRegistry, Route};
pub use retry::{ShouldRetry, ShouldRetryUntil};
