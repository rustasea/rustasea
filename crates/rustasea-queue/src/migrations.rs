//! Queue schema migrations — `jobs` and `failed_jobs` tables.
//!
//! Dialect-portable DDL (TEXT columns, RFC3339 timestamps) so the same bodies
//! run on SQLite (tests) and Postgres. Register them with [`register`] at
//! application boot so `cargo artisan migrate` creates the queue tables.

use std::sync::OnceLock;

use rustasea_orm::{Migration, Result};

/// Guards [`register`] so the queue migrations enter the process-wide registry
/// exactly once even when both app boot and the CLI call it.
static REGISTERED: OnceLock<()> = OnceLock::new();

/// `jobs` table — pending, delayed, and reserved queue rows.
pub struct CreateJobsTable;

impl Migration for CreateJobsTable {
    /// Unique migration name.
    fn name(&self) -> &str {
        "2027_01_01_000001_create_jobs_table"
    }

    /// Create the `jobs` table plus its lookup indexes.
    fn up(&self) -> Result<String> {
        Ok("\
CREATE TABLE IF NOT EXISTS jobs (\
id TEXT PRIMARY KEY, \
queue TEXT NOT NULL, \
payload TEXT NOT NULL, \
attempts INTEGER NOT NULL DEFAULT 0, \
reserved_at TEXT, \
available_at TEXT NOT NULL, \
created_at TEXT NOT NULL\
); \
CREATE INDEX IF NOT EXISTS idx_jobs_queue_available ON jobs (queue, available_at); \
CREATE INDEX IF NOT EXISTS idx_jobs_reserved_at ON jobs (reserved_at)"
            .to_string())
    }

    /// Drop the `jobs` table.
    fn down(&self) -> Result<String> {
        Ok("DROP TABLE IF EXISTS jobs".to_string())
    }
}

/// `failed_jobs` table — dead-lettered jobs awaiting `queue:retry`.
pub struct CreateFailedJobsTable;

impl Migration for CreateFailedJobsTable {
    /// Unique migration name.
    fn name(&self) -> &str {
        "2027_01_01_000002_create_failed_jobs_table"
    }

    /// Create the `failed_jobs` table plus its lookup indexes.
    fn up(&self) -> Result<String> {
        Ok("\
CREATE TABLE IF NOT EXISTS failed_jobs (\
id TEXT PRIMARY KEY, \
connection TEXT NOT NULL, \
queue TEXT NOT NULL, \
payload TEXT NOT NULL, \
exception TEXT NOT NULL, \
failed_at TEXT NOT NULL\
); \
CREATE INDEX IF NOT EXISTS idx_failed_jobs_queue ON failed_jobs (queue); \
CREATE INDEX IF NOT EXISTS idx_failed_jobs_failed_at ON failed_jobs (failed_at)"
            .to_string())
    }

    /// Drop the `failed_jobs` table.
    fn down(&self) -> Result<String> {
        Ok("DROP TABLE IF EXISTS failed_jobs".to_string())
    }
}

/// Register the queue migrations into the process-wide migrator.
///
/// Idempotent: repeated calls (app boot + CLI) register only once. Order is the
/// execution order: `jobs` then `failed_jobs`.
pub fn register() {
    REGISTERED.get_or_init(|| {
        rustasea_orm::register_migration(CreateJobsTable);
        rustasea_orm::register_migration(CreateFailedJobsTable);
    });
}

/// Build a migrator containing only the queue migrations (tests/tools).
pub fn migrator() -> rustasea_orm::Migrator {
    let mut migrator = rustasea_orm::Migrator::new();
    migrator.add(CreateJobsTable);
    migrator.add(CreateFailedJobsTable);
    migrator
}
