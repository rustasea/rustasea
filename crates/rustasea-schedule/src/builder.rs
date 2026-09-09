/// `Schedule::command(..)` builder producing registered `ScheduleCommand`s.
use crate::command::ScheduleCommand;
use crate::error::Result;
use crate::scheduler::Scheduler;

/// Entry point for declaring scheduled commands.
///
/// ```rust,ignore
/// Schedule::command("emails:send").daily().at("08:00")
///     .skip_if_running().on_one_server().register()?;
/// ```
pub struct Schedule;

impl Schedule {
    /// Start building a scheduled command named `command`.
    pub fn command(command: &'static str) -> ScheduleBuilder {
        ScheduleBuilder::new(command)
    }
}

/// Chainable frequency + modifier builder.
///
/// Frequencies (one of `cron`/`daily`/`every_minute`) set the schedule
/// expression; modifiers (`skip_if_running`, `on_one_server`) add guards.
/// `register` hands the finished command to the shared `Scheduler`.
#[derive(Debug)]
pub struct ScheduleBuilder {
    command: &'static str,
    cron: Option<String>,
    daily_at: Option<String>,
    skip_if_running: bool,
    on_one_server: bool,
}

impl ScheduleBuilder {
    /// Start building for `command`.
    pub fn new(command: &'static str) -> Self {
        Self {
            command,
            cron: None,
            daily_at: None,
            skip_if_running: false,
            on_one_server: false,
        }
    }

    /// Set an explicit cron expression (5 fields).
    pub fn cron(mut self, expression: &str) -> Self {
        self.cron = Some(expression.to_string());
        self.daily_at = None;
        self
    }

    /// Run daily at `HH:MM` (UTC).
    pub fn daily(mut self) -> Self {
        self.daily_at = Some("00:00".to_string());
        self
    }

    /// Set the daily time (used after `.daily()`).
    pub fn at(mut self, time: &str) -> Self {
        self.daily_at = Some(time.to_string());
        self
    }

    /// Run every minute.
    pub fn every_minute(mut self) -> Self {
        self.cron = Some("* * * * *".to_string());
        self.daily_at = None;
        self
    }

    /// Suppress a run whose previous run is still active.
    pub fn skip_if_running(mut self) -> Self {
        self.skip_if_running = true;
        self
    }

    /// Run on exactly one server via a distributed lock.
    pub fn on_one_server(mut self) -> Self {
        self.on_one_server = true;
        self
    }

    /// Alias of `skip_if_running` (snake-case doc surface).
    pub fn skip_if_still_running(mut self) -> Self {
        self.skip_if_running = true;
        self
    }

    /// Build the command entry.
    pub fn build(self) -> Result<ScheduleCommand> {
        let mut cmd = if let Some(expr) = self.cron {
            ScheduleCommand::cron(self.command, &expr)?
        } else {
            let time = self.daily_at.unwrap_or_else(|| "00:00".to_string());
            ScheduleCommand::daily_at(self.command, &time)?
        };
        if self.skip_if_running {
            cmd = cmd.skip_if_running();
        }
        if self.on_one_server {
            cmd = cmd.on_one_server();
        }
        Ok(cmd)
    }

    /// Build and register the command with the shared `Scheduler`.
    pub fn register(self) -> Result<()> {
        let command = self.build()?;
        Scheduler::global().register(command);
        Ok(())
    }
}
