/// Typed errors for the events layer.
use thiserror::Error;

/// Alias for results produced by event dispatch.
pub type Result<T> = std::result::Result<T, EventError>;

/// Top-level event error type.
#[derive(Debug, Error)]
pub enum EventError {
    /// A listener's `handle` returned an error during inline dispatch.
    #[error("listener failed: {0}")]
    Listener(String),

    /// Queue-backed dispatch failed (queue unavailable / serialization).
    #[error("queued event dispatch failed: {0}")]
    Queue(String),

    /// No listener is registered for the dispatched event type.
    #[error("no listener registered for event {0}")]
    Unhandled(&'static str),
}

impl From<rustasea_queue::QueueError> for EventError {
    /// Map a queue dispatch failure onto the events `Queue` variant.
    fn from(error: rustasea_queue::QueueError) -> Self {
        EventError::Queue(error.to_string())
    }
}
