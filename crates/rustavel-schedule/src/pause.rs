/// Scheduler state, pause/resume flag handling, and CLI-surface operations.
use std::sync::{Arc, RwLock};

use crate::error::{Result, ScheduleError};
use crate::scheduler::Scheduler;

/// Pause state of the scheduler (`schedule_state` singleton semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerStatus {
    /// The ticker is dispatching scheduled commands.
    Running,
    /// The ticker is halted; no command dispatch happens.
    Paused,
}

impl SchedulerStatus {
    /// Whether the scheduler is currently paused.
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }
}

/// Singleton scheduler state mirroring the `schedule_state` row (`id = 1`).
///
/// The real persistence lands with the database driver (S05-T06 follow-up);
/// in-memory `RwLock` keeps pause/resume + the paused flag testable today.
#[derive(Debug)]
pub struct ScheduleState {
    status: RwLock<SchedulerStatus>,
}

impl ScheduleState {
    /// Create a running scheduler state.
    pub fn new() -> Self {
        Self {
            status: RwLock::new(SchedulerStatus::Running),
        }
    }

    /// Current scheduler status.
    pub fn status(&self) -> SchedulerStatus {
        self.status
            .read()
            .map(|s| *s)
            .unwrap_or(SchedulerStatus::Running)
    }

    /// Whether the scheduler is paused.
    pub fn paused(&self) -> bool {
        self.status().is_paused()
    }

    /// Set the paused flag; returns the previous status.
    pub fn set_paused(&self, paused: bool) -> SchedulerStatus {
        let previous = self.status();
        if let Ok(mut s) = self.status.write() {
            *s = if paused {
                SchedulerStatus::Paused
            } else {
                SchedulerStatus::Running
            };
        }
        previous
    }
}

impl Default for ScheduleState {
    /// Create a running scheduler state.
    fn default() -> Self {
        Self::new()
    }
}

/// Shared state accessor used by the pause/resume CLI surfaces.
pub(crate) fn state() -> &'static ScheduleState {
    state_cell().as_ref()
}

/// Fetch the shared state as an `Arc` (for `withScheduling` wiring).
///
/// Returns a clone of the process-wide singleton so callers observe the
/// same paused flag the pause/resume facade mutates.
pub fn shared_state() -> Arc<ScheduleState> {
    Arc::clone(state_cell())
}

/// The process-wide singleton — one cell shared by `state()` and
/// `shared_state()`, so pause/resume and `withScheduling` wiring never
/// observe divergent instances.
fn state_cell() -> &'static Arc<ScheduleState> {
    static CELL: std::sync::OnceLock<Arc<ScheduleState>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| Arc::new(ScheduleState::new()))
}

/// Pause the scheduler (`schedule:pause`).
///
/// Idempotent: setting the flag when already paused does not emit a second
/// `SchedulePaused`. Returns whether the scheduler was running before.
pub async fn pause() -> Result<bool> {
    let previous = state().set_paused(true);
    let was_running = previous == SchedulerStatus::Running;
    if was_running {
        crate::scheduler::emit_paused().await?;
    }
    Ok(was_running)
}

/// Resume the scheduler (`schedule:resume`).
///
/// Idempotent: clearing the flag when already running emits nothing. Returns
/// whether the scheduler was paused before.
pub async fn resume() -> Result<bool> {
    let previous = state().set_paused(false);
    let was_paused = previous == SchedulerStatus::Paused;
    if was_paused {
        crate::scheduler::emit_resumed().await?;
    }
    Ok(was_paused)
}

/// `schedule:list` view — every registered command with its next run time.
pub fn list() -> Vec<crate::command::ScheduleCommand> {
    Scheduler::global().commands()
}

/// `schedule:list --json`-style payload builder (kept for CLI ergonomics).
pub fn list_entries() -> Vec<ScheduleListEntry> {
    list()
        .into_iter()
        .map(|c| ScheduleListEntry {
            command: c.command,
            frequency: c.frequency.clone(),
            modifiers: modifier_names(&c),
        })
        .collect()
}

/// One row of `schedule:list`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleListEntry {
    /// Command name.
    pub command: &'static str,
    /// Frequency description.
    pub frequency: String,
    /// Active modifier names.
    pub modifiers: Vec<&'static str>,
}

/// Collect the active modifier names for a command.
fn modifier_names(cmd: &crate::command::ScheduleCommand) -> Vec<&'static str> {
    let mut out = Vec::new();
    if cmd.skip_if_running {
        out.push("skipIfStillRunning");
    }
    if cmd.on_one_server {
        out.push("onOneServer");
    }
    out
}

/// Alias keeping the pause/resume doc surface discoverable.
pub type SchedulePauseFacade = Arc<ScheduleState>;

/// Pause-state error shape used by the CLI when idempotence is asserted.
pub fn already_paused() -> ScheduleError {
    ScheduleError::StateUnavailable("schedule already paused (idempotent)".into())
}
