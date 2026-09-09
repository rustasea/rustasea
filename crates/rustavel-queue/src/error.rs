/// Typed errors for the queue layer.
use std::sync::PoisonError;

use thiserror::Error;

/// Alias for results produced by queue operations.
pub type Result<T> = std::result::Result<T, QueueError>;

/// Top-level queue error type.
///
/// Mirrors the `QueueError`/`JobError` catalogue from api-queue.md §4: a
/// duplicated boot-time route is a `DuplicateRoute`, an unknown connection
/// name is `UnknownConnection`, and an unreachable backing store surfaces as
/// `StoreUnavailable` — never a panic.
#[derive(Debug, Error)]
pub enum QueueError {
    /// A second `Queue::route::<J>` was attempted for an already-registered type.
    #[error("duplicate queue route for {type_name}: already routed")]
    DuplicateRoute {
        /// Fully-qualified job type name.
        type_name: &'static str,
    },

    /// No driver is registered under the requested connection name.
    #[error("unknown queue connection: {0}")]
    UnknownConnection(String),

    /// The backing store (Redis/DB) is unreachable.
    #[error("queue store unavailable: {0}")]
    StoreUnavailable(String),

    /// A job could not be serialized into its queue payload.
    #[error("job serialization failed: {0}")]
    Serialization(String),

    /// The routed queue registry is not yet booted (no route registered).
    #[error("no route registered for job type {0}; register with Queue::route before dispatch")]
    Unrouted(String),

    /// The registry lock was poisoned by a panicking dispatcher.
    #[error("queue registry lock poisoned")]
    RegistryPoisoned,

    /// No queued job was available for reservation.
    #[error("queue {0} is empty")]
    Empty(String),
}

impl From<PoisonError<std::sync::RwLockWriteGuard<'_, crate::registry::RegistryInner>>>
    for QueueError
{
    /// Convert a poisoned registry write lock into a typed error.
    fn from(
        _: PoisonError<std::sync::RwLockWriteGuard<'_, crate::registry::RegistryInner>>,
    ) -> Self {
        QueueError::RegistryPoisoned
    }
}

impl From<PoisonError<std::sync::RwLockReadGuard<'_, crate::registry::RegistryInner>>>
    for QueueError
{
    /// Convert a poisoned registry read lock into a typed error.
    fn from(
        _: PoisonError<std::sync::RwLockReadGuard<'_, crate::registry::RegistryInner>>,
    ) -> Self {
        QueueError::RegistryPoisoned
    }
}

/// Job-execution error returned by `Job::handle`.
///
/// `Timeout`/`MaxAttemptsExceeded` are produced by the worker loop; user
/// handlers return `Exception` for domain failures.
#[derive(Debug, Error)]
pub enum JobError {
    /// A configured per-job timeout elapsed before `handle` returned.
    #[error("job timed out")]
    Timeout,

    /// The job exhausted its retry budget.
    #[error("max attempts exceeded")]
    MaxAttemptsExceeded,

    /// A domain failure reported by `Job::handle`.
    #[error("{0}")]
    Exception(String),
}
