//! Migration commands — `migrate`, `migrate:fresh`, `migrate:rollback`.
//!
//! Each command opens a real pool from the resolved database URL and drives the
//! process-wide [`rustasea_orm::Migrator`] registry built by
//! `register_migration`/`register_seeder` at application boot. No SQL is echoed.

use async_trait::async_trait;

use crate::artisan::{Command, Io};
use crate::error::{CliError, CliResult};
use rustasea_orm::{DbPool, Migrator};

/// Return the database URL from the process environment, when set and non-blank.
///
/// `DATABASE_URL` wins over the nested `DATABASE__URL` overlay key. Both are
/// checked before any config file so an explicit environment value can never be
/// shadowed by `config/database.toml`.
fn database_url_from_env() -> Option<String> {
    ["DATABASE_URL", "DATABASE__URL"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok().filter(|url| !url.trim().is_empty()))
}

/// Resolve the database connection URL from the environment, then layered config.
///
/// Precedence, highest to lowest:
/// 1. `DATABASE_URL` — plain process environment variable.
/// 2. `DATABASE__URL` — nested environment overlay key.
/// 3. `database.url` from `config/database.toml` (the fallback).
/// 4. The legacy top-level `database_url` key.
///
/// The environment is consulted first because [`rustasea_config::ConfigLoader`]'s
/// overlay maps `DATABASE_URL` to the unrelated top-level `database_url` key; a
/// `config/database.toml` `database.url` value would otherwise shadow it.
/// Returns a typed error when nothing is configured.
pub(crate) fn database_url() -> CliResult<String> {
    if let Some(url) = database_url_from_env() {
        return Ok(url);
    }

    let loader = rustasea_config::ConfigLoader::load_from(&["config/database", "config/app"])
        .map_err(|error| CliError::Domain(format!("failed to load database config: {error}")))?;

    if let Ok(url) = loader.get_key::<String>("database.url") {
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

/// Shared body for `migrate:rollback`.
///
/// Opens a real pool from config and reverts the most recent `step` batches via
/// [`Migrator::rollback`]. Each call reverts the then-most-recent batch, so
/// asking for more batches than exist stops cleanly once the ledger empties.
/// Every migration's `down` body and its tracking-row deletion commit in one
/// transaction, mirroring the `migrate` command's error handling.
async fn run_rollback(step: usize, io: &mut Io) -> CliResult<()> {
    let migrator: Migrator = rustasea_orm::registered_migrator();

    if migrator.is_empty() {
        io.line("No migrations registered in this binary — call `rustasea::orm::register_migration(..)` at application boot.");
        return Ok(());
    }

    let url = database_url()?;
    let pool = DbPool::connect(&url)
        .await
        .map_err(|error| CliError::Domain(error.to_string()))?;

    let mut rolled_back = Vec::new();
    for _ in 0..step {
        let batch = migrator
            .rollback(&pool)
            .await
            .map_err(|error| CliError::Domain(error.to_string()))?;
        if batch.is_empty() {
            break;
        }
        rolled_back.extend(batch);
    }

    if rolled_back.is_empty() {
        io.line("Nothing to roll back.");
    } else {
        io.line(format!("Rolled back {} migration(s):", rolled_back.len()));
        for name in &rolled_back {
            io.line(format!("  rolled back: {name}"));
        }
    }

    pool.close().await;
    Ok(())
}

/// Parse the `--step` flag (`--step N` or `--step=N`), defaulting to one batch.
fn parse_step(args: &[String]) -> CliResult<usize> {
    let mut step = 1usize;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--step" {
            let value = iter.next().ok_or_else(|| CliError::InvalidArguments {
                command: "migrate:rollback".to_string(),
                detail: "`--step` requires a positive integer, e.g. `migrate:rollback --step 2`"
                    .to_string(),
            })?;
            step = parse_step_value(value)?;
        } else if let Some(value) = arg.strip_prefix("--step=") {
            step = parse_step_value(value)?;
        }
    }
    Ok(step)
}

/// Parse and validate a positive `--step` value.
fn parse_step_value(value: &str) -> CliResult<usize> {
    match value.trim().parse::<usize>() {
        Ok(n) if n >= 1 => Ok(n),
        _ => Err(CliError::InvalidArguments {
            command: "migrate:rollback".to_string(),
            detail: format!("`--step` expects a positive integer, got `{value}`"),
        }),
    }
}

/// `migrate:rollback` — revert the most recent migration batch.
///
/// Opens a real pool from config and calls [`Migrator::rollback`] once per
/// requested step (default `1`), so `--step 2` reverts the two most recent
/// batches. An irreversible migration surfaces a typed domain error and leaves
/// its tracking row intact for a retry.
pub struct MigrateRollback;

#[async_trait]
impl Command for MigrateRollback {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "migrate:rollback"
    }
    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("migrate:rollback [--step N]")
    }
    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Roll back the last migration batch")
    }
    /// Execute: open a pool and revert the requested number of batches.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let step = parse_step(&args)?;
        run_rollback(step, io).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that read or write process-global environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// `DATABASE_URL` takes precedence over the nested `DATABASE__URL` overlay.
    #[test]
    fn env_plain_var_wins_over_nested() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("DATABASE__URL", "sqlite://nested");
        std::env::set_var("DATABASE_URL", "sqlite://plain");
        let resolved = database_url_from_env();
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("DATABASE__URL");
        assert_eq!(resolved.as_deref(), Some("sqlite://plain"));
    }

    /// `DATABASE__URL` is used when `DATABASE_URL` is absent.
    #[test]
    fn env_nested_var_is_fallback() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("DATABASE_URL");
        std::env::set_var("DATABASE__URL", "sqlite://nested");
        let resolved = database_url_from_env();
        std::env::remove_var("DATABASE__URL");
        assert_eq!(resolved.as_deref(), Some("sqlite://nested"));
    }

    /// Blank environment values are ignored so config can supply the fallback.
    #[test]
    fn env_blank_value_is_ignored() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("DATABASE__URL");
        std::env::set_var("DATABASE_URL", "   ");
        let resolved = database_url_from_env();
        std::env::remove_var("DATABASE_URL");
        assert!(resolved.is_none());
    }
}
