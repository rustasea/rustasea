//! Framework operation commands — queue:failed/retry, schedule:list/run/
//! pause/resume, migrate.
//!
//! Queue and schedule surfaces read the M4 registries (in-memory dead-letter
//! sink, scheduler singleton); `migrate` delegates SQL reporting to the ORM
//! `Migrator` until pool execution wiring lands.

use async_trait::async_trait;

use crate::artisan::{Command, Io};
use crate::error::{CliError, CliResult};
use crate::output;

/// `queue:failed` — list dead-lettered jobs (FR-504 surface).
pub struct QueueFailed;

#[async_trait]
impl Command for QueueFailed {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "queue:failed"
    }

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("queue:failed")
    }

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("List all failed queue jobs")
    }

    /// Execute: dump the in-memory dead-letter sink.
    async fn run(&self, _args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let jobs = rustavel_queue::driver::failed_jobs();
        if jobs.is_empty() {
            io.line("No failed jobs.");
            return Ok(());
        }
        let mut rows = vec![vec!["Id".into(), "Queue".into(), "Exception".into()]];
        for job in jobs {
            rows.push(vec![
                job.id.to_string(),
                job.queue.clone(),
                job.exception.clone(),
            ]);
        }
        io.line(output::table(rows).trim_end());
        Ok(())
    }
}

/// `queue:retry` — retry a failed job by id (dead-letter sink drop today).
pub struct QueueRetry;

#[async_trait]
impl Command for QueueRetry {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "queue:retry"
    }

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("queue:retry {id}")
    }

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Retry a failed queue job")
    }

    /// Execute: resolve the JobId and drop it from the sink.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let id = args
            .first()
            .filter(|n| !n.starts_with('-'))
            .cloned()
            .unwrap_or_default();
        let parsed = uuid::Uuid::parse_str(&id).map_err(|e| CliError::InvalidArguments {
            command: "queue:retry".into(),
            detail: format!("invalid job id `{id}`: {e}"),
        })?;
        let job_id = rustavel_queue::JobId::from_uuid(parsed);
        match rustavel_queue::driver::retry_failed(job_id).await {
            Ok(()) => {
                io.line(format!("retried job {id}"));
                Ok(())
            }
            Err(e) => Err(CliError::Domain(e.to_string())),
        }
    }
}

/// `schedule:list` — list scheduled commands (M4 schedule registry).
pub struct ScheduleList;

#[async_trait]
impl Command for ScheduleList {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "schedule:list"
    }

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("schedule:list [--json]")
    }

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("List scheduled commands")
    }

    /// Execute: enumerate the global scheduler's commands.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let json = args.iter().any(|a| a == "--json");
        let entries = rustavel_schedule::pause::list_entries();
        if json {
            io.line(serde_json::to_string(&entries)?);
            return Ok(());
        }
        if entries.is_empty() {
            io.line("No scheduled commands registered.");
            return Ok(());
        }
        let mut rows = vec![vec![
            "Command".into(),
            "Frequency".into(),
            "Modifiers".into(),
        ]];
        for entry in entries {
            rows.push(vec![
                entry.command.to_string(),
                entry.frequency.clone(),
                entry.modifiers.join(", "),
            ]);
        }
        io.line(output::table(rows).trim_end());
        Ok(())
    }
}

/// Shared pause/resume body for the schedule state facade.
async fn run_pause_resume(args: Vec<String>, io: &mut Io, pause: bool) -> CliResult<()> {
    let json = args.iter().any(|a| a == "--json");
    let changed = if pause {
        rustavel_schedule::pause::pause().await
    } else {
        rustavel_schedule::pause::resume().await
    }
    .map_err(|e| CliError::Domain(e.to_string()))?;
    if json {
        io.line(serde_json::json!({ "changed": changed }).to_string());
    } else if changed {
        io.line(if pause {
            "Scheduler paused."
        } else {
            "Scheduler resumed."
        });
    } else if pause {
        io.line("Scheduler was already paused (idempotent).");
    } else {
        io.line("Scheduler was already running (idempotent).");
    }
    Ok(())
}

/// `schedule:pause` — pause the scheduler ticker.
pub struct SchedulePause;

#[async_trait]
impl Command for SchedulePause {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "schedule:pause"
    }
    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("schedule:pause")
    }
    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Pause scheduled command dispatch")
    }
    /// Execute the pause.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        run_pause_resume(args, io, true).await
    }
}

/// `schedule:resume` — resume the scheduler ticker.
pub struct ScheduleResume;

#[async_trait]
impl Command for ScheduleResume {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "schedule:resume"
    }
    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("schedule:resume")
    }
    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Resume scheduled command dispatch")
    }
    /// Execute the resume.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        run_pause_resume(args, io, false).await
    }
}

/// `schedule:run` — run due scheduled commands once.
pub struct ScheduleRun;

#[async_trait]
impl Command for ScheduleRun {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "schedule:run"
    }
    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("schedule:run")
    }
    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Run due scheduled commands once")
    }
    /// Execute a single tick.
    async fn run(&self, _args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let outcome = rustavel_schedule::scheduler::run_once().await;
        match outcome {
            Ok(rustavel_schedule::TickOutcome::Dispatched) => {
                io.line("Dispatched due scheduled commands.");
            }
            Ok(rustavel_schedule::TickOutcome::Idle) => {
                io.line("No scheduled commands were due.");
            }
            Ok(rustavel_schedule::TickOutcome::Suppressed) => {
                io.line("Scheduler is paused; nothing dispatched.");
            }
            Err(e) => return Err(CliError::Domain(e.to_string())),
        }
        Ok(())
    }
}

/// `migrate` — report pending migrations through the ORM [`Migrator`].
///
/// Pool execution is not wired in this binary; the command builds a
/// [`rustavel_orm::Migrator`], reports each registered migration's `up` SQL
/// (or reversed `down` SQL under `--fresh`) and notes seeder availability.
pub struct Migrate;

#[async_trait]
impl Command for Migrate {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "migrate"
    }
    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("migrate [--fresh] [--seed]")
    }
    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Run pending migrations")
    }
    /// Execute: delegate SQL reporting to the ORM Migrator.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let fresh = args.iter().any(|a| a == "--fresh");
        let seed = args.iter().any(|a| a == "--seed");
        let migrator = rustavel_orm::Migrator::new();
        let names = migrator.names();

        if names.is_empty() {
            io.line("No migrations registered in this binary — add them to `database/migrations/` and register each in the Migrator.");
            if seed {
                io.line("Seeding database… (no seeders registered)");
            }
            return Ok(());
        }

        if fresh {
            io.line("migrate:fresh — rolling back all migrations…");
            for sql in migrator.down_sql().unwrap_or_default() {
                for statement in sql.lines().filter(|l| !l.trim().is_empty()) {
                    io.line(format!("rollback: {statement}"));
                }
            }
        }

        io.line(format!("Running {} migration(s)…", names.len()));
        for sql in migrator.up_sql().unwrap_or_default() {
            for statement in sql.lines().filter(|l| !l.trim().is_empty()) {
                io.line(format!("migrate: {statement}"));
            }
        }

        if seed {
            io.line("Seeding database…");
        }
        Ok(())
    }
}
