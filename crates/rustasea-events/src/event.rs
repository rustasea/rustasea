/// Event trait and the contractual framework events.
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A typed domain event.
///
/// Events are plain serializable, cloneable data (`Serialize +
/// DeserializeOwned + Clone + Send + Sync + 'static`) — no domain `Any`
/// (C-03). `Clone` lets one dispatch fan out an owned copy per listener;
/// `DeserializeOwned` lets a queue-backed listener job round-trip the event
/// through a worker.
pub trait Event: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// Stable event name reported to listeners/spies.
    fn event_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// A job-attempted notification with the failure `exception`.
///
/// Contract rename vs Laravel 12's `exceptionOccurred`: the field is
/// `exception` (FSD §4.3 observability crossing). Field shape is contractual —
/// do not rename.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAttempted {
    /// Job type/queue that was attempted.
    pub job: String,
    /// Failure detail (`JobError::Timeout`, exception message, …).
    pub exception: String,
}

impl Event for JobAttempted {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "JobAttempted"
    }
}

/// Queue-busy notification carrying the overloaded connection name.
///
/// Contract rename vs Laravel's `connection`: the field is `connectionName`
/// (FSD §4.3). Do not rename.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueBusy {
    /// Connection that reported backlog pressure.
    pub connection_name: String,
    /// Queue depth that crossed the busy threshold.
    pub depth: usize,
}

impl Event for QueueBusy {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "QueueBusy"
    }
}

/// Emitted when the scheduler is paused (schedule:pause).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SchedulePaused;

impl Event for SchedulePaused {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "SchedulePaused"
    }
}

/// Emitted when the scheduler resumes (schedule:resume).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScheduleResumed;

impl Event for ScheduleResumed {
    /// Stable event name.
    fn event_name(&self) -> &'static str {
        "ScheduleResumed"
    }
}
