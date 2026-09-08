/// Listener trait with opt-in queue-backed execution.
use async_trait::async_trait;

use crate::error::EventError;
use crate::event::Event;

/// Queue configuration for a listener.
///
/// `enable: true` enqueues the listener as a queue job rather than awaiting
/// it inline (US-M4-04). `connection`/`queue`/`delay` mirror the queue
/// dispatch overrides.
#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    /// Whether the listener is dispatched through the queue.
    pub enable: bool,
    /// Queue connection name (defaults to the routed connection).
    pub connection: Option<&'static str>,
    /// Queue name (defaults to the routed queue).
    pub queue: Option<&'static str>,
    /// Delay before the listener job becomes available.
    pub delay: Option<std::time::Duration>,
}

impl QueueConfig {
    /// Queue config with the queue disabled (inline execution).
    pub const fn disabled() -> Self {
        Self {
            enable: false,
            connection: None,
            queue: None,
            delay: None,
        }
    }

    /// Queue config with the queue enabled.
    pub const fn enabled() -> Self {
        Self {
            enable: true,
            connection: None,
            queue: None,
            delay: None,
        }
    }
}

impl Default for QueueConfig {
    /// Listeners run inline unless opted into the queue.
    fn default() -> Self {
        Self::disabled()
    }
}

/// A listener reacting to `E`.
///
/// Implementors declare `const QUEUE: bool` (or override `queue()`) to opt
/// into queue-backed execution; inline listeners `handle` the event when it is
/// dispatched.
#[async_trait]
pub trait Listener<E: Event>: Send + Sync + 'static {
    /// Whether this listener is enqueued as a job instead of run inline.
    const QUEUE: bool = false;

    /// Queue configuration (defaults honour `QUEUE`).
    fn queue_config(&self) -> QueueConfig {
        if Self::QUEUE {
            QueueConfig::enabled()
        } else {
            QueueConfig::disabled()
        }
    }

    /// Handle an inline-dispatched event.
    async fn handle(&self, event: E) -> std::result::Result<(), EventError>;
}
