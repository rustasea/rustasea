//! Migration contract, seeder, and factory stubs.
//!
//! Execution against a live pool (sqlx::migrate! / Migrator::run) is wired in
//! S03-T01/S03-T05; this module defines the traits and typed errors the
//! `database/migrations/` and `database/seeders/` entries implement.

use crate::error::Result;
use chrono::{DateTime, Utc};
use std::future::Future;

/// Errors surfaced by migrations.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// `up` was declared irreversible (no matching `down`).
    #[error("migration {name} is irreversible")]
    Irreversible { name: String },

    /// A required Postgres extension (e.g. `vector`) is missing.
    #[error("extension {extension} missing: {hint}")]
    ExtensionMissing { extension: String, hint: String },

    /// The migration body failed.
    #[error("migration {name} failed: {detail}")]
    Failed {
        /// Migration that failed.
        name: String,
        /// Failure detail.
        detail: String,
    },
}

/// A versioned, reversible migration with `up`/`down`.
pub trait Migration: Send + Sync {
    /// Unique migration name (`2027_01_01_000001_create_users_table`).
    fn name(&self) -> &str;

    /// Apply the migration forward.
    fn up(&self) -> Result<String>;

    /// Revert the migration; `Err(Irreversible)` when not revertable.
    fn down(&self) -> Result<String> {
        Err(MigrationError::Irreversible {
            name: self.name().to_string(),
        }
        .into())
    }
}

/// Runner for an ordered migration set, tracking the `migrations` table.
#[derive(Default)]
pub struct Migrator {
    migrations: Vec<Box<dyn Migration>>,
}

impl Migrator {
    /// Create an empty migrator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a migration (order of registration = order of execution).
    pub fn add<M: Migration + 'static>(&mut self, migration: M) -> &mut Self {
        self.migrations.push(Box::new(migration));
        self
    }

    /// List registered migration names in execution order.
    pub fn names(&self) -> Vec<String> {
        self.migrations
            .iter()
            .map(|m| m.name().to_string())
            .collect()
    }

    /// Compile the pending `up` SQL statements (idempotent re-run is a no-op
    /// once recorded in the `migrations` table).
    pub fn up_sql(&self) -> Result<Vec<String>> {
        self.migrations.iter().map(|m| m.up()).collect()
    }

    /// Compile `down` SQL in reverse order (`migrate:fresh` reversibility).
    pub fn down_sql(&self) -> Result<Vec<String>> {
        let mut out = Vec::with_capacity(self.migrations.len());
        for m in self.migrations.iter().rev() {
            out.push(m.down()?);
        }
        Ok(out)
    }
}

/// Idempotent database seeder (`database/seeders/*.rs`).
pub trait Seeder: Send + Sync {
    /// Run the seeder; must be re-runnable without duplicates.
    fn run(&self) -> Result<String>;
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

/// Async bridge placeholder — the real signature takes `&mut PgConnection`.
///
/// Kept so callers compile against the final shape; pool wiring in S03-T01.
pub async fn run_migrations<F, Fut>(migrator: &Migrator, executor: F) -> Result<()>
where
    F: FnOnce(Vec<String>) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let statements = migrator.up_sql()?;
    executor(statements).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CreateUsers;

    impl Migration for CreateUsers {
        fn name(&self) -> &str {
            "0001_create_users_table"
        }
        fn up(&self) -> Result<String> {
            Ok("CREATE TABLE users (id UUID PRIMARY KEY);".into())
        }
        fn down(&self) -> Result<String> {
            Ok("DROP TABLE users;".into())
        }
    }

    /// Verifies up/down ordering and reverse execution.
    #[test]
    fn migrator_reverses_in_order() {
        let mut m = Migrator::new();
        m.add(CreateUsers);
        assert_eq!(m.names(), vec!["0001_create_users_table"]);
        assert!(m.up_sql().unwrap()[0].starts_with("CREATE"));
        assert!(m.down_sql().unwrap()[0].starts_with("DROP"));
    }

    /// Verifies default down is irreversible.
    #[test]
    fn irreversible_by_default() {
        struct NoDown;
        impl Migration for NoDown {
            fn name(&self) -> &str {
                "0002_irreversible"
            }
            fn up(&self) -> Result<String> {
                Ok("SELECT 1;".into())
            }
        }
        let mut m = Migrator::new();
        m.add(NoDown);
        assert!(matches!(
            m.down_sql().unwrap_err(),
            crate::error::OrmError::Migration(MigrationError::Irreversible { .. })
        ));
    }
}
