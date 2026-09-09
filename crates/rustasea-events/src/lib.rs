//! RustaSea Events — typed events, listeners, dispatch + dispatchAfterResponse.
//!
//! Sprint 05 (M4) scope per sprint-05.md S05-T05: the `Event` trait, the
//! `Listener<E>` trait with `Queue { enable: true }` support (enqueue the
//! listener as a queue job instead of awaiting it inline), immediate
//! `dispatch` + response-buffered `dispatchAfterResponse`, and the contractual
//! `JobAttempted { exception }` / `QueueBusy { connectionName }` field renames
//! (FSD §4.3).

pub mod dispatcher;
pub mod error;
pub mod event;
pub mod listener;

pub use async_trait::async_trait;
pub use dispatcher::{Dispatcher, EventSink};
pub use error::{EventError, Result};
pub use event::{Event, JobAttempted, QueueBusy, SchedulePaused, ScheduleResumed};
pub use listener::{Listener, QueueConfig};
