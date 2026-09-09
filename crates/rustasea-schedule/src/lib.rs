//! RustaSea Schedule — scheduler, frequencies, pause/resume, events.
//!
//! Sprint 05 (M4) scope per sprint-05.md S05-T06: the `Schedule::command`
//! builder with `daily`/`cron`/`everyMinute`/`skipIfStillRunning`/
//! `onOneServer` modifiers, the `schedule:list`/`schedule:run` tick loop,
//! idempotent `schedule:pause`/`schedule:resume` emitting `SchedulePaused`/
//! `ScheduleResumed`, deferred `withScheduling` semantics (first tick, not
//! boot), and the `schedule_state` paused flag.

pub mod builder;
pub mod command;
pub mod error;
pub mod pause;
pub mod scheduler;

pub use builder::{Schedule, ScheduleBuilder};
pub use command::ScheduleCommand;
pub use error::{Result, ScheduleError};
pub use pause::{pause, resume, ScheduleState, SchedulerStatus};
pub use rustasea_events::{SchedulePaused, ScheduleResumed};
pub use scheduler::{Scheduler, TickOutcome};
