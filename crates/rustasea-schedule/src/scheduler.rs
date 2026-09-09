/// Scheduler — command registry, tick loop, `skipIfStillRunning` guard.
use std::sync::OnceLock;
use std::sync::RwLock;
use std::time::Duration;

use chrono::Utc;

use crate::command::ScheduleCommand;
use crate::error::{Result, ScheduleError};
use crate::pause::SchedulerStatus;

/// Outcome of one tick evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// The scheduler is paused; no command was evaluated for dispatch.
    Suppressed,
    /// At least one command matched and was dispatched.
    Dispatched,
    /// No command matched this tick.
    Idle,
}

/// Shared registry of scheduled commands + pause state.
///
/// `Scheduler::global()` is the `OnceLock` singleton the `schedule:run`
/// ticker and `schedule:list` read; `withScheduling` registration is deferred
/// to the first tick by the app provider, not applied at `boot` (US-M4-05).
pub struct Scheduler {
    commands: RwLock<Vec<ScheduleCommand>>,
    running: RwLock<Vec<String>>,
}

impl Scheduler {
    /// Access the process-wide scheduler singleton.
    pub fn global() -> &'static Scheduler {
        static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();
        SCHEDULER.get_or_init(Scheduler::new)
    }

    /// Create an empty scheduler.
    pub fn new() -> Self {
        Self {
            commands: RwLock::new(Vec::new()),
            running: RwLock::new(Vec::new()),
        }
    }

    /// Register a scheduled command.
    pub fn register(&self, command: ScheduleCommand) {
        let Ok(mut cmds) = self.commands.write() else {
            return;
        };
        if !cmds.iter().any(|c| c.command == command.command) {
            cmds.push(command);
        }
    }

    /// Snapshot of every registered command.
    pub fn commands(&self) -> Vec<ScheduleCommand> {
        let Ok(cmds) = self.commands.read() else {
            return Vec::new();
        };
        cmds.clone()
    }

    /// Clear all registered commands (test isolation).
    pub fn clear(&self) {
        if let Ok(mut cmds) = self.commands.write() {
            cmds.clear();
        }
    }

    /// Whether a command is currently mid-run (`skipIfStillRunning` guard).
    pub fn is_running(&self, command: &str) -> bool {
        let Ok(running) = self.running.read() else {
            return false;
        };
        running.iter().any(|c| c == command)
    }

    /// Mark a command as running.
    fn mark_running(&self, command: &str) {
        if let Ok(mut running) = self.running.write() {
            if !running.iter().any(|c| c == command) {
                running.push(command.to_string());
            }
        }
    }

    /// Clear the running mark for a command.
    fn mark_idle(&self, command: &str) {
        if let Ok(mut running) = self.running.write() {
            running.retain(|c| c != command);
        }
    }

    /// Run one tick: evaluate every command and dispatch the matches.
    ///
    /// Honours the paused flag (suppresses all dispatch) and each command's
    /// `skipIfStillRunning` guard. `onOneServer` is resolved by the caller
    /// through the cache lock before invoking dispatch.
    pub async fn tick(&self) -> Result<TickOutcome> {
        if crate::pause::state().paused() {
            return Ok(TickOutcome::Suppressed);
        }
        let now = Utc::now();
        let mut dispatched = false;
        for command in self.commands() {
            if command.skip_if_running && self.is_running(command.command) {
                continue;
            }
            let Some(next) = command.next_run(now) else {
                continue;
            };
            if next <= now + chrono::Duration::seconds(1) {
                self.mark_running(command.command);
                let result = dispatch_command(&command).await;
                self.mark_idle(command.command);
                result?;
                dispatched = true;
            }
        }
        Ok(if dispatched {
            TickOutcome::Dispatched
        } else {
            TickOutcome::Idle
        })
    }
}

impl Default for Scheduler {
    /// Create an empty scheduler.
    fn default() -> Self {
        Self::new()
    }
}

/// Dispatch one scheduled command through the queue layer.
///
/// The command is enqueued as a queue job on the default connection
/// (`queue:work` drains it). The real enqueue lands with the queue driver
/// wiring (S05-T03 follow-up); until then a due command cannot be dispatched,
/// so a no-op ack would silently drop every scheduled run — fail loud with a
/// typed `Dispatch` error instead (the tick loop aborts and the process stays
/// alive; Chaos: pause during a tick keeps the process alive).
async fn dispatch_command(command: &ScheduleCommand) -> Result<()> {
    Err(ScheduleError::Dispatch(format!(
        "command `{}` is due but no queue dispatch path is wired yet (S05-T03)",
        command.command
    )))
}

/// Emit `SchedulePaused` through the events dispatcher (pause.rs).
pub(crate) async fn emit_paused() -> Result<()> {
    rustasea_events::Dispatcher::dispatch(rustasea_events::SchedulePaused)
        .await
        .map_err(|e| ScheduleError::StateUnavailable(e.to_string()))
}

/// Emit `ScheduleResumed` through the events dispatcher (pause.rs).
pub(crate) async fn emit_resumed() -> Result<()> {
    rustasea_events::Dispatcher::dispatch(rustasea_events::ScheduleResumed)
        .await
        .map_err(|e| ScheduleError::StateUnavailable(e.to_string()))
}

/// Scheduled tick interval used by `schedule:run` (60s per sprint-05.md).
pub fn tick_interval() -> Duration {
    Duration::from_secs(60)
}

/// Convenience: run the global scheduler one tick.
pub async fn run_once() -> Result<TickOutcome> {
    Scheduler::global().tick().await
}

/// Suppressed-status helper for pause-aware tick callers.
pub fn suppressed() -> Result<TickOutcome> {
    let _ = SchedulerStatus::Paused;
    Ok(TickOutcome::Suppressed)
}
