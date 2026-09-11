//! Seeder, factory, and record types split out of [`crate::migration`].
//!
//! Keeps the migration runner module within the file-size limit; these items are
//! re-exported from [`crate::migration`] so call sites are unchanged.

use super::Migrator;
use crate::db::DbPool;
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::future::Future;

/// Idempotent database seeder (`database/seeders/*.rs`).
///
/// [`Seeder::sql`] compiles the statements and [`Seeder::run`] executes them
/// against a live pool — the default `run` body is sufficient for SQL-only
/// seeders. Implementations are object-safe (via `async_trait`), so
/// [`Migrator`] can hold a heterogeneous `Box<dyn Seeder>` list.
#[async_trait]
pub trait Seeder: Send + Sync {
    /// Seeder name reported by [`Migrator::seed`].
    fn name(&self) -> &str;

    /// Compile the idempotent SQL statements (`ON CONFLICT DO NOTHING`).
    fn sql(&self) -> Result<String>;

    /// Execute the seeder against `pool` (real round-trip, not joined strings).
    async fn run(&self, pool: &DbPool) -> Result<()> {
        let sql = self.sql()?;
        if sql.trim().is_empty() {
            return Ok(());
        }
        pool.execute_script(&sql).await
    }
}

/// Model factory for test/seed data generation (`UserFactory::create(n)`).
pub trait Factory<T>: Send + Sync {
    /// Produce the next model instance with sequence-based uniqueness.
    fn definition(&mut self) -> T;

    /// Number of instances produced so far (sequence counter).
    fn count(&self) -> usize;

    /// Produce the next instance (alias of [`Factory::definition`]).
    ///
    /// Matches the `Factory::create` call shape used by seed/test code; a
    /// single instance is returned, `create_many(n)` batches them.
    fn create(&mut self) -> T {
        self.definition()
    }
}

/// Marker type for the `migrations` table row recorded per batch.
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// Migration name (unique).
    pub name: String,
    /// Batch number for `migrate:fresh` reverse ordering.
    pub batch: i64,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

/// Async bridge helper — compile `up` SQL and hand it to a caller executor.
///
/// Retained for backward compatibility with SQL-emit callers; [`Migrator::run`]
/// is the real pool-backed executor.
pub async fn run_migrations<F, Fut>(migrator: &Migrator, executor: F) -> Result<()>
where
    F: FnOnce(Vec<String>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let statements = migrator.up_sql()?;
    executor(statements).await?;
    Ok(())
}
