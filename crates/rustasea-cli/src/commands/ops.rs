//! Framework operation commands — queue:failed/retry, schedule:list/run/
//! pause/resume, migrate.
//!
//! Queue and schedule surfaces read the M4 registries (in-memory dead-letter
//! sink, scheduler singleton); `migrate` opens a real pool from config and runs
//! the ORM [`rustasea_orm::Migrator`] against it (no SQL echo).

use async_trait::async_trait;

use crate::artisan::{Command, Io};
use crate::error::{CliError, CliResult};
use crate::output;
use rustasea_orm::{DbPool, Migrator};

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
        let jobs = rustasea_queue::driver::failed_jobs();
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
        let job_id = rustasea_queue::JobId::from_uuid(parsed);
        match rustasea_queue::driver::retry_failed(job_id).await {
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
        let entries = rustasea_schedule::pause::list_entries();
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
        rustasea_schedule::pause::pause().await
    } else {
        rustasea_schedule::pause::resume().await
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
        let outcome = rustasea_schedule::scheduler::run_once().await;
        match outcome {
            Ok(rustasea_schedule::TickOutcome::Dispatched) => {
                io.line("Dispatched due scheduled commands.");
            }
            Ok(rustasea_schedule::TickOutcome::Idle) => {
                io.line("No scheduled commands were due.");
            }
            Ok(rustasea_schedule::TickOutcome::Suppressed) => {
                io.line("Scheduler is paused; nothing dispatched.");
            }
            Err(e) => return Err(CliError::Domain(e.to_string())),
        }
        Ok(())
    }
}

/// Resolve the database connection URL from layered config.
///
/// Precedence: `database.url` (TOML or `DATABASE__URL`), then `DATABASE_URL`
/// (plain env), then the `database_url` key produced by the config env overlay.
/// Returns a typed error when nothing is configured.
fn database_url() -> CliResult<String> {
    let loader = rustasea_config::ConfigLoader::load_from(&["config/database", "config/app"])
        .map_err(|error| CliError::Domain(format!("failed to load database config: {error}")))?;

    if let Ok(url) = loader.get_key::<String>("database.url") {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }
    if let Ok(url) = loader.get_key::<String>("database_url") {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }
    Err(CliError::Domain(
        "database URL not configured — set `database.url` in config/database.toml or the DATABASE_URL env var".into(),
    ))
}

/// Shared body for `migrate` and `migrate:fresh`.
///
/// Opens a real pool from config and executes the process-wide registry;
/// `fresh` drops every table first, `seed` runs the seeders afterwards.
async fn run_migrate(fresh: bool, seed: bool, io: &mut Io) -> CliResult<()> {
    let migrator: Migrator = rustasea_orm::registered_migrator();

    if migrator.is_empty() {
        io.line("No migrations registered in this binary — call `rustasea::orm::register_migration(..)` at application boot.");
        if seed {
            io.line("Seeding skipped (no seeders registered).");
        }
        return Ok(());
    }

    let url = database_url()?;
    let pool = DbPool::connect(&url)
        .await
        .map_err(|error| CliError::Domain(error.to_string()))?;

    let applied = if fresh {
        migrator
            .fresh(&pool)
            .await
            .map_err(|error| CliError::Domain(error.to_string()))?
    } else {
        migrator
            .run(&pool)
            .await
            .map_err(|error| CliError::Domain(error.to_string()))?
    };

    if fresh {
        io.line(format!(
            "Dropped all tables; applied {} migration(s).",
            applied.len()
        ));
    } else if applied.is_empty() {
        io.line("Nothing to migrate.");
    } else {
        io.line(format!("Applied {} migration(s):", applied.len()));
    }
    for name in &applied {
        io.line(format!("  migrated: {name}"));
    }

    if seed {
        let ran = migrator
            .seed(&pool)
            .await
            .map_err(|error| CliError::Domain(error.to_string()))?;
        if ran.is_empty() {
            io.line("No seeders registered.");
        } else {
            io.line(format!("Ran {} seeder(s):", ran.len()));
            for name in &ran {
                io.line(format!("  seeded: {name}"));
            }
        }
    }

    pool.close().await;
    Ok(())
}

/// `migrate` — run pending migrations against the configured database.
///
/// Opens a real pool from config and executes the process-wide registry built
/// by `register_migration`/`register_seeder` at application boot. `--fresh`
/// drops every table first; `--seed` runs the registered seeders afterwards.
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
    /// Execute: open a pool and run the registered migrator/seeders.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let fresh = args.iter().any(|a| a == "--fresh");
        let seed = args.iter().any(|a| a == "--seed");
        run_migrate(fresh, seed, io).await
    }
}

/// `migrate:fresh` — drop all tables then re-run every migration.
///
/// Equivalent to `migrate --fresh`; `--seed` additionally runs the registered
/// seeders after the schema is rebuilt.
pub struct MigrateFresh;

#[async_trait]
impl Command for MigrateFresh {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "migrate:fresh"
    }
    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("migrate:fresh [--seed]")
    }
    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Drop all tables and re-run every migration")
    }
    /// Execute: drop all tables, re-migrate, optionally seed.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let seed = args.iter().any(|a| a == "--seed");
        run_migrate(true, seed, io).await
    }
}
