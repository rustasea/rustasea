/// Typed errors for the schedule layer.
use thiserror::Error;

/// Alias for results produced by schedule operations.
pub type Result<T> = std::result::Result<T, ScheduleError>;

/// Top-level schedule error type.
#[derive(Debug, Error)]
pub enum ScheduleError {
    /// A cron expression failed to parse.
    #[error("invalid cron expression `{0}`: {1}")]
    InvalidCron(String, String),

    /// An invalid frequency/time combination was requested.
    #[error("invalid schedule: {0}")]
    Invalid(String),

    /// The scheduler state store is unavailable.
    #[error("schedule state unavailable: {0}")]
    StateUnavailable(String),

    /// Dispatch of a scheduled command failed.
    #[error("schedule dispatch failed: {0}")]
    Dispatch(String),
}
