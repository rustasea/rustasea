//! Integration tests for queue-backed listeners (GAP-008).
//!
//! Covers the positive enqueue→worker-execute path, the unchanged inline path,
//! and the typed enqueue-failure path. Each test uses its own event type so the
//! process-wide listener/queue registries cannot collide.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rustasea_events::{Dispatcher, Event, EventError, Listener, QueueConfig};
use rustasea_queue::{run_worker, Queue, QueueDriver, SyncDriver};

/// Event delivered to the queued listener.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct QueuedEvent {
    /// Entity identifier carried by the event.
    id: u64,
}

impl Event for QueuedEvent {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "QueuedEvent"
    }
}

/// Number of times the queued listener has run.
static QUEUED_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Listener that must be enqueued rather than awaited inline.
struct QueuedListener;

#[rustasea_events::async_trait]
impl Listener<QueuedEvent> for QueuedListener {
    const QUEUE: bool = true;

    /// Route the listener job to the in-memory test connection.
    fn queue_config(&self) -> QueueConfig {
        QueueConfig {
            enable: true,
            connection: Some("events-queued-test"),
            queue: Some("events"),
            delay: None,
        }
    }

    /// Record the run; a queued listener is only invoked by the worker.
    async fn handle(&self, _event: QueuedEvent) -> Result<(), EventError> {
        QUEUED_RUNS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A queue-enabled listener is stored on the driver, then run by the worker.
#[tokio::test]
async fn queued_listener_is_enqueued_then_executed_by_worker() {
    QUEUED_RUNS.store(0, Ordering::SeqCst);
    let driver = Arc::new(SyncDriver::new());
    Queue::register_driver("events-queued-test", driver.clone());

    Dispatcher::listen::<QueuedEvent, QueuedListener>(QueuedListener);
    Dispatcher::dispatch(QueuedEvent { id: 7 })
        .await
        .expect("queued dispatch succeeds");

    // Enqueued, not run inline: one payload stored, listener not yet invoked.
    assert_eq!(driver.pending_size("events").await.unwrap(), 1);
    assert_eq!(QUEUED_RUNS.load(Ordering::SeqCst), 0);

    // The worker resolves the registered job kind and executes the listener.
    let processed = run_worker(
        driver.clone() as Arc<dyn QueueDriver>,
        vec!["events".to_string()],
        10,
    )
    .await
    .expect("worker drains the queue");
    assert_eq!(processed, 1);
    assert_eq!(QUEUED_RUNS.load(Ordering::SeqCst), 1);
}

/// Event delivered to the inline listener.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct InlineEvent;

impl Event for InlineEvent {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "InlineEvent"
    }
}

/// Number of times the inline listener has run.
static INLINE_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Listener that runs inline (queue disabled).
struct InlineListener;

#[rustasea_events::async_trait]
impl Listener<InlineEvent> for InlineListener {
    /// Record the run during dispatch.
    async fn handle(&self, _event: InlineEvent) -> Result<(), EventError> {
        INLINE_RUNS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Inline listener behaviour is unchanged: it runs during dispatch.
#[tokio::test]
async fn inline_listener_runs_during_dispatch() {
    INLINE_RUNS.store(0, Ordering::SeqCst);
    Dispatcher::listen::<InlineEvent, InlineListener>(InlineListener);

    Dispatcher::dispatch(InlineEvent)
        .await
        .expect("inline dispatch succeeds");
    assert_eq!(INLINE_RUNS.load(Ordering::SeqCst), 1);
}

/// Event delivered to the failing listener.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FailingEvent;

impl Event for FailingEvent {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "FailingEvent"
    }
}

/// Queue-enabled listener routed to an unregistered connection.
struct FailingListener;

#[rustasea_events::async_trait]
impl Listener<FailingEvent> for FailingListener {
    const QUEUE: bool = true;

    /// Route the listener job to a connection with no registered driver.
    fn queue_config(&self) -> QueueConfig {
        QueueConfig {
            enable: true,
            connection: Some("events-missing-connection"),
            queue: Some("events"),
            delay: None,
        }
    }

    /// Never reached: enqueue fails before the worker runs.
    async fn handle(&self, _event: FailingEvent) -> Result<(), EventError> {
        Ok(())
    }
}

/// An enqueue failure surfaces as a typed `EventError::Queue`, never a panic.
#[tokio::test]
async fn enqueue_failure_surfaces_typed_error() {
    Dispatcher::listen::<FailingEvent, FailingListener>(FailingListener);

    match Dispatcher::dispatch(FailingEvent).await {
        Err(EventError::Queue(message)) => {
            assert!(message.contains("events-missing-connection"), "{message}");
        }
        other => panic!("expected EventError::Queue, got {other:?}"),
    }
}
