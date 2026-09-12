//! `cargo xtask migrate` — run the framework's registered migrations.
//!
//! Mirrors the CLI `migrate` command wiring
//! (`crates/rustasea-cli/src/commands/ops.rs:312-429`): resolve the database URL
//! from the same layered config source, register the framework migrations, open a
//! real [`DbPool`], and execute the process-wide [`Migrator`]. The bootstrap is
//! intentionally duplicated rather than imported because the CLI's helpers are
//! crate-private; keep both call sites in sync when the config precedence changes.

use rustasea_config::ConfigLoader;
use rustasea_orm::{registered_migrator, DbPool};

use crate::FAILURE;

/// Resolve the database connection URL from layered config.
///
/// Precedence mirrors the CLI's `database_url`: `database.url` (TOML or the
/// `DATABASE__URL` env overlay), then plain `DATABASE_URL`, then the
/// `database_url` key produced by the config env overlay.
fn database_url() -> Result<String, String> {
    let loader = ConfigLoader::load_from(&["config/database", "config/app"])
        .map_err(|error| format!("failed to load database config: {error}"))?;

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
    Err("database URL not configured — set `database.url` in config/database.toml or the DATABASE_URL env var".to_string())
}

/// Run every pending migration against the configured database.
///
/// Registers the framework's built-in migrations (queue `jobs`/`failed_jobs`)
/// before executing the process-wide registry, matching `cargo artisan migrate`.
/// Returns a process exit code; arguments are accepted for forward compatibility.
pub fn run(_args: &[String]) -> i32 {
    rustasea_queue::register_queue_migrations();
    let migrator = registered_migrator();
    if migrator.is_empty() {
        eprintln!("xtask migrate: no migrations registered.");
        return FAILURE;
    }

    let url = match database_url() {
        Ok(url) => url,
        Err(message) => {
            eprintln!("xtask migrate: {message}");
            return FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("xtask migrate: failed to start async runtime: {error}");
            return FAILURE;
        }
    };

    runtime.block_on(async move {
        let pool = match DbPool::connect(&url).await {
            Ok(pool) => pool,
            Err(error) => {
                eprintln!("xtask migrate: failed to connect to database: {error}");
                return FAILURE;
            }
        };

        let applied = match migrator.run(&pool).await {
            Ok(applied) => applied,
            Err(error) => {
                eprintln!("xtask migrate: {error}");
                pool.close().await;
                return FAILURE;
            }
        };

        if applied.is_empty() {
            println!("xtask migrate: nothing to migrate.");
        } else {
            println!("xtask migrate: applied {} migration(s):", applied.len());
            for name in &applied {
                println!("  migrated: {name}");
            }
        }
        pool.close().await;
        0
    })
}
