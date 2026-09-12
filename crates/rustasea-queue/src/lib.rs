//! RustaSea Queue — typed jobs, central routing, drivers, chain/batch, metrics.
//!
//! Sprint 05 (M4) scope per sprint-05.md S05-T01..T03: the typed `Job` trait
//! with retry contracts, the `Queue::route::<Job>` registry with per-dispatch
//! `onQueue`/`onConnection` overrides, `sync`/`database`/`redis` drivers (the
//! `database` driver persists to the ORM `jobs` table; `redis` is opt-in behind
//! the `redis` feature), `chain`/`batch` dispatch, the `failed_jobs`
//! dead-letter surface and the Cloud queue metric shapes.

pub mod batch;
pub mod driver;
pub mod error;
pub mod job;
pub mod metrics;
pub mod migrations;
pub mod notification;
pub mod registry;
pub mod retry;

pub use async_trait::async_trait;
pub use batch::{dispatch_batch, BatchHandle, BatchId};
#[cfg(feature = "redis")]
pub use driver::RedisDriver;
pub use driver::{
    default_resolver, failed_jobs, register_job, register_job_handler, retry_failed, run_worker,
    run_worker_with, DatabaseDriver, QueueDriver, SyncDriver, DATABASE_CONNECTION, DATABASE_DRIVER,
    REDIS_CONNECTION, REDIS_DRIVER, SYNC_CONNECTION,
};
pub use error::{JobError, QueueError, Result};
pub use job::{
    run_erased, ConcreteJob, DispatchHandle, ErasedJob, FailedJob, Job, JobId, JobOutcome,
    JobPayload,
};
pub use metrics::{JobQueueMetrics, QueueMetrics, Queues};
pub use migrations::{migrator as queue_migrator, register as register_queue_migrations};
pub use notification::{
    should_suppress, skipped_notifications, NotificationGuard, NotificationSkipReason,
    NotificationSkipped,
};
#[cfg(feature = "redis")]
pub use registry::register_redis_driver;
pub use registry::{register_database_driver, Queue, QueueRegistry, Route};
pub use retry::{ShouldRetry, ShouldRetryUntil};
