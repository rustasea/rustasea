/// Event dispatcher — immediate and after-response dispatch.
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;

use crate::error::{EventError, Result};
use crate::event::Event;
use crate::listener::Listener;

/// Global listener registry shared by `Dispatcher` facades.
///
/// Entries are erased over the concrete listener type (never over the event —
/// the map key is the event `TypeId`), so the non-object-safe `QUEUE` const on
/// `Listener` never has to live behind a vtable.
static LISTENERS: OnceLock<Mutex<HashMap<TypeId, Vec<Arc<dyn ErasedListener>>>>> = OnceLock::new();

/// Global after-response buffer shared by `EventSink`.
static SINK: OnceLock<Mutex<Vec<Box<dyn Any + Send>>>> = OnceLock::new();

/// Access the global listener map.
fn listeners() -> &'static Mutex<HashMap<TypeId, Vec<Arc<dyn ErasedListener>>>> {
    LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Object-safe listener dispatch entry.
#[async_trait]
trait ErasedListener: Send + Sync {
    /// Handle `event`, downcast by the concrete implementation.
    async fn invoke_erased(&self, event: &(dyn Any + Sync)) -> Result<()>;
}

/// Concrete listener stored behind `ErasedListener`.
struct TypedListener<E: Event, L: Listener<E>> {
    inner: L,
    _marker: std::marker::PhantomData<fn(E)>,
}

impl<E: Event, L: Listener<E>> TypedListener<E, L> {
    /// Wrap a concrete listener.
    fn new(inner: L) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<E: Event, L: Listener<E>> ErasedListener for TypedListener<E, L> {
    /// Handle the event, honouring the listener's queue config.
    async fn invoke_erased(&self, event: &(dyn Any + Sync)) -> Result<()> {
        let any_ref: &dyn Any = event;
        let typed = any_ref.downcast_ref::<E>().ok_or_else(|| {
            EventError::Listener("listener received an event of the wrong type".into())
        })?;
        let config = self.inner.queue_config();
        if !config.enable {
            return self.inner.handle(typed.clone()).await;
        }
        // Queue-backed listener (QUEUE=true): the typed listener-job wrapper
        // (`impl Job` bridging the events layer into the queue) lands with the
        // real queue driver. Until then there is nothing that can enqueue this
        // event, so acking silently would drop it — fail loud instead so a
        // QUEUE=true listener can never be lost without an error.
        Err(EventError::Queue(format!(
            "listener for {} is QUEUE=true but no queue enqueue path is wired yet",
            typed.event_name(),
        )))
    }
}

impl<E: Event, L: Listener<E>> std::fmt::Debug for TypedListener<E, L> {
    /// Debug rendering without the erased listener body.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedListener").finish_non_exhaustive()
    }
}

/// The `Dispatcher` facade — `dispatch` + `dispatchAfterResponse`.
///
/// `dispatch` runs every registered listener for the event's type: inline when
/// the listener's queue is disabled, enqueued as a queue job when enabled.
/// `dispatch_after_response` appends to the response-flush sink so the event
/// fires only after the HTTP response has been sent.
pub struct Dispatcher;

impl Dispatcher {
    /// Register a listener for `E` (boot-time).
    pub fn listen<E: Event, L: Listener<E>>(listener: L) {
        let mut map = listeners().lock().unwrap_or_else(|p| p.into_inner());
        map.entry(TypeId::of::<E>())
            .or_default()
            .push(Arc::new(TypedListener::<E, L>::new(listener)) as Arc<dyn ErasedListener>);
    }

    /// Dispatch `event` to all registered listeners now.
    pub async fn dispatch<E: Event>(event: E) -> Result<()> {
        let type_id = TypeId::of::<E>();
        let snapshot = {
            let map = listeners().lock().unwrap_or_else(|p| p.into_inner());
            map.get(&type_id).cloned().unwrap_or_default()
        };
        if snapshot.is_empty() {
            return Err(EventError::Unhandled(event.event_name()));
        }
        for erased in &snapshot {
            erased.invoke_erased(&event).await?;
        }
        Ok(())
    }

    /// Buffer `event` until the HTTP response is flushed (`EventSink`).
    pub async fn dispatch_after_response<E: Event>(event: E) -> Result<()> {
        EventSink::push(Box::new(event));
        Ok(())
    }
}

/// Test spy / response-flush sink for `dispatchAfterResponse`.
///
/// Buffered events are drained by the HTTP layer after the response is sent;
/// tests drain explicitly to assert FIFO ordering (US-M4-04 spy).
pub struct EventSink;

impl EventSink {
    /// Buffer an erased event (static storage).
    pub fn push(event: Box<dyn Any + Send>) {
        let mut buf = Self::global().lock().unwrap_or_else(|p| p.into_inner());
        buf.push(event);
    }

    /// Drain buffered events in FIFO order.
    pub fn drain() -> Vec<Box<dyn Any + Send>> {
        let mut buf = Self::global().lock().unwrap_or_else(|p| p.into_inner());
        std::mem::take(&mut *buf)
    }

    /// Number of buffered events.
    pub fn len() -> usize {
        let buf = Self::global().lock().unwrap_or_else(|p| p.into_inner());
        buf.len()
    }

    /// Whether the sink is empty.
    pub fn is_empty() -> bool {
        Self::len() == 0
    }

    /// Drain and downcast every buffered event of `T`, preserving FIFO order.
    pub fn drain_typed<T: Event + 'static>() -> Vec<T> {
        let mut out = Vec::new();
        for item in Self::drain() {
            if let Ok(typed) = item.downcast::<T>() {
                out.push(*typed);
            }
        }
        out
    }

    /// Global buffer accessor.
    fn global() -> &'static Mutex<Vec<Box<dyn Any + Send>>> {
        SINK.get_or_init(|| Mutex::new(Vec::new()))
    }
}
