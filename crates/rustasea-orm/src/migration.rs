//! Migration contract, runner, seeder, and factory traits.
//!
//! [`Migrator`] executes registered [`Migration`] bodies against a live
//! [`DbPool`], recording each in a `migrations` tracking table so re-runs are
//! idempotent and the last batch can be rolled back in reverse. The sync
//! `up_sql`/`down_sql` emit API is retained for backward compatibility. Only the
//! `sqlx` runtime API is used — never the compile-time `query!` macros.

use crate::db::DbPool;
use crate::error::{OrmError, Result};
use crate::types::Value;
use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;

mod fresh;
mod traits;

pub use traits::{run_migrations, Factory, MigrationRecord, Seeder};

/// Name of the migration tracking table.
const MIGRATIONS_TABLE: &str = "migrations";

/// Process-wide migration registry populated by an application at startup.
///
/// `cargo artisan migrate` reads this registry so it executes the app's
/// migrations without the CLI depending on the application crate. Register
/// migrations with [`register_migration`] (order = execution order) and seeders
/// with [`register_seeder`]; [`registered_migrator`] returns a clone for the
/// runner.
static MIGRATIONS: std::sync::OnceLock<std::sync::Mutex<Migrator>> = std::sync::OnceLock::new();

/// Register a migration into the process-wide registry.
///
/// Call once per migration from application boot code (`main`/provider) in the
/// intended execution order; the CLI's `migrate` command then runs them.
/// Aborts (panics) if the registry mutex was poisoned mid-registration.
pub fn register_migration<M: Migration + 'static>(migration: M) {
    let registry = MIGRATIONS.get_or_init(|| std::sync::Mutex::new(Migrator::new()));
    lock_registry_for_write(registry, "register_migration").add(migration);
}

/// Register a seeder into the process-wide registry.
///
/// Seeders run (in registration order) when `migrate --seed` or `migrate:fresh
/// --seed` executes. Aborts (panics) if the registry mutex was poisoned.
pub fn register_seeder<S: Seeder + 'static>(seeder: S) {
    let registry = MIGRATIONS.get_or_init(|| std::sync::Mutex::new(Migrator::new()));
    lock_registry_for_write(registry, "register_seeder").add_seeder(seeder);
}

/// Clone the process-wide registry for execution.
///
/// Returns an empty [`Migrator`] when nothing has been registered. Recovers a
/// poisoned lock via [`std::sync::PoisonError::into_inner`] — see the helper.
pub fn registered_migrator() -> Migrator {
    MIGRATIONS
        .get()
        .map(|registry| lock_registry_for_read(registry).clone())
        .unwrap_or_default()
}

/// Lock the registry for a mutating registration, aborting on poison.
///
/// A poisoned lock may be mid-mutation, so registration panics rather than
/// silently dropping the entry; `operation` names the failing call site.
fn lock_registry_for_write<'a>(
    registry: &'a std::sync::Mutex<Migrator>,
    operation: &str,
) -> std::sync::MutexGuard<'a, Migrator> {
    registry.lock().unwrap_or_else(|_poison| {
        panic!(
            "migration registry poisoned during {operation}: a prior holder \
             panicked mid-registration; aborting to avoid silent data loss"
        )
    })
}

/// Lock the registry for a read-only clone, recovering a poisoned lock.
///
/// Recovering via [`std::sync::PoisonError::into_inner`] is safe here: the worst
/// case is a partially-registered snapshot, never a dropped registration.
fn lock_registry_for_read(
    registry: &std::sync::Mutex<Migrator>,
) -> std::sync::MutexGuard<'_, Migrator> {
    registry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

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

    /// Compile the forward SQL body.
    fn up(&self) -> Result<String>;

    /// Compile the reverse SQL body; `Err(Irreversible)` when not revertable.
    fn down(&self) -> Result<String> {
        Err(MigrationError::Irreversible {
            name: self.name().to_string(),
        }
        .into())
    }
}

/// Runner for an ordered migration set, tracking the `migrations` table.
#[derive(Default, Clone)]
pub struct Migrator {
    migrations: Vec<Arc<dyn Migration>>,
    seeders: Vec<Arc<dyn Seeder>>,
}

impl Migrator {
    /// Create an empty migrator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a migration (order of registration = order of execution).
    pub fn add<M: Migration + 'static>(&mut self, migration: M) -> &mut Self {
        self.migrations.push(Arc::new(migration));
        self
    }

    /// Register a seeder run by [`Migrator::seed`].
    pub fn add_seeder<S: Seeder + 'static>(&mut self, seeder: S) -> &mut Self {
        self.seeders.push(Arc::new(seeder));
        self
    }

    /// Whether any migration has been registered.
    pub fn is_empty(&self) -> bool {
        self.migrations.is_empty()
    }

    /// List registered migration names in execution order.
    pub fn names(&self) -> Vec<String> {
        self.migrations
            .iter()
            .map(|m| m.name().to_string())
            .collect()
    }

    /// Compile the pending `up` SQL statements (emit-only, no round-trip).
    pub fn up_sql(&self) -> Result<Vec<String>> {
        self.migrations.iter().map(|m| m.up()).collect()
    }

    /// Compile `down` SQL in reverse order (emit-only, no round-trip).
    pub fn down_sql(&self) -> Result<Vec<String>> {
        let mut out = Vec::with_capacity(self.migrations.len());
        for m in self.migrations.iter().rev() {
            out.push(m.down()?);
        }
        Ok(out)
    }

    /// Apply every not-yet-applied migration in registration order.
    ///
    /// Each migration's schema body and its tracking row commit atomically in a
    /// single transaction (one transaction per migration). SQLite/Postgres DDL is
    /// transactional, so a record failure rolls the schema back too; MySQL DDL is
    /// non-transactional, so its schema change is best-effort and may persist even
    /// when the tracking row rolls back. Creates the `migrations` table first, and
    /// a second call is a no-op. Returns the names applied by this call.
    pub async fn run(&self, pool: &DbPool) -> Result<Vec<String>> {
        self.ensure_table(pool).await?;
        let applied = self.applied_names(pool).await?;
        let pending: Vec<&Arc<dyn Migration>> = self
            .migrations
            .iter()
            .filter(|m| !applied.contains(m.name()))
            .collect();
        if pending.is_empty() {
            return Ok(Vec::new());
        }

        let batch = self.max_batch(pool).await? + 1;
        let mut applied_now = Vec::with_capacity(pending.len());
        for migration in pending {
            let name = migration.name().to_string();
            let sql = migration.up()?;
            let applied = name.clone();
            crate::execution::transaction(pool, |tx| {
                Box::pin(async move {
                    tx.execute_script(&sql)
                        .await
                        .map_err(|error| failed(&name, error))?;
                    tx.execute_bind(
                        &format!(
                            "INSERT INTO {MIGRATIONS_TABLE} (name, batch, executed_at) \
                             VALUES ($1, $2, $3)"
                        ),
                        &[
                            Value::Text(name.clone()),
                            Value::Int(batch),
                            Value::Text(Utc::now().to_rfc3339()),
                        ],
                    )
                    .await?;
                    Ok(())
                })
            })
            .await?;
            applied_now.push(applied);
        }
        Ok(applied_now)
    }

    /// Revert the most recent batch in reverse registration order.
    ///
    /// Each migration's reverse schema body and its tracking-row deletion commit
    /// atomically in a single transaction (one transaction per migration); the
    /// record is deleted only after a successful revert, so a failed `down` leaves
    /// the row intact for a retry. A migration whose `down` is the default surfaces
    /// [`MigrationError::Irreversible`]. An empty tracking table is a no-op.
    pub async fn rollback(&self, pool: &DbPool) -> Result<Vec<String>> {
        self.ensure_table(pool).await?;
        let batch = self.max_batch(pool).await?;
        if batch == 0 {
            return Ok(Vec::new());
        }
        let batch_names = self.batch_names(pool, batch).await?;

        let mut rolled_back = Vec::new();
        for migration in self.migrations.iter().rev() {
            let name = migration.name().to_string();
            if !batch_names.contains(&name) {
                continue;
            }
            let sql = migration.down()?;
            let rolled = name.clone();
            crate::execution::transaction(pool, |tx| {
                Box::pin(async move {
                    tx.execute_script(&sql)
                        .await
                        .map_err(|error| failed(&name, error))?;
                    tx.execute_bind(
                        &format!("DELETE FROM {MIGRATIONS_TABLE} WHERE name = $1"),
                        &[Value::Text(name.clone())],
                    )
                    .await?;
                    Ok(())
                })
            })
            .await?;
            rolled_back.push(rolled);
        }
        Ok(rolled_back)
    }

    /// Drop every table, then re-run all migrations from scratch.
    ///
    /// Returns the names applied. The tracking table is dropped with the rest,
    /// so the following [`Migrator::run`] treats every migration as pending.
    pub async fn fresh(&self, pool: &DbPool) -> Result<Vec<String>> {
        fresh::drop_all_tables(pool).await?;
        self.run(pool).await
    }

    /// Run every registered seeder against `pool`.
    ///
    /// Returns the names of the seeders that ran; seeders are expected to be
    /// idempotent so repeated calls do not duplicate rows.
    pub async fn seed(&self, pool: &DbPool) -> Result<Vec<String>> {
        let mut ran = Vec::with_capacity(self.seeders.len());
        for seeder in &self.seeders {
            seeder.run(pool).await?;
            ran.push(seeder.name().to_string());
        }
        Ok(ran)
    }

    /// Create the `migrations` tracking table when absent.
    async fn ensure_table(&self, pool: &DbPool) -> Result<()> {
        pool.execute_script(&format!(
            "CREATE TABLE IF NOT EXISTS {MIGRATIONS_TABLE} (
                name TEXT PRIMARY KEY,
                batch INTEGER NOT NULL,
                executed_at TEXT NOT NULL
            );"
        ))
        .await
    }

    /// Names already recorded in the tracking table.
    async fn applied_names(&self, pool: &DbPool) -> Result<HashSet<String>> {
        let rows = pool
            .fetch_json(
                &format!("SELECT name FROM {MIGRATIONS_TABLE} ORDER BY batch, name"),
                &[],
            )
            .await?;
        Ok(rows
            .iter()
            .filter_map(|row| row.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect())
    }

    /// Highest recorded batch number, or `0` when none exist.
    async fn max_batch(&self, pool: &DbPool) -> Result<i64> {
        let rows = pool
            .fetch_json(
                &format!("SELECT COALESCE(MAX(batch), 0) AS batch FROM {MIGRATIONS_TABLE}"),
                &[],
            )
            .await?;
        Ok(rows
            .first()
            .and_then(|row| row.get("batch"))
            .and_then(|b| b.as_i64())
            .unwrap_or(0))
    }

    /// Names recorded for a specific batch.
    async fn batch_names(&self, pool: &DbPool, batch: i64) -> Result<HashSet<String>> {
        let rows = pool
            .fetch_json(
                &format!("SELECT name FROM {MIGRATIONS_TABLE} WHERE batch = $1"),
                &[Value::Int(batch)],
            )
            .await?;
        Ok(rows
            .iter()
            .filter_map(|row| row.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect())
    }
}

/// Map a driver failure onto a `Failed` migration error, preserving typed errors.
fn failed(name: &str, error: OrmError) -> OrmError {
    match error {
        OrmError::Migration(inner) => OrmError::Migration(inner),
        other => OrmError::Migration(MigrationError::Failed {
            name: name.to_string(),
            detail: other.to_string(),
        }),
    }
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

    /// Poison a fresh registry mutex by panicking while its lock is held.
    fn poisoned_registry() -> std::sync::Arc<std::sync::Mutex<Migrator>> {
        let registry = std::sync::Arc::new(std::sync::Mutex::new(Migrator::new()));
        let handle = {
            let registry = std::sync::Arc::clone(&registry);
            std::thread::spawn(move || {
                let _guard = registry.lock().expect("fresh lock is unpoisoned");
                panic!("intentional poison");
            })
        };
        assert!(handle.join().is_err());
        registry
    }

    /// Registration must abort loudly when the registry lock is poisoned.
    #[test]
    fn registration_aborts_on_poisoned_lock() {
        let registry = poisoned_registry();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drop(lock_registry_for_write(&registry, "register_migration"));
        }));
        let payload = outcome.expect_err("poisoned write lock must panic");
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(message.contains("poisoned"), "unexpected panic: {message}");
        assert!(message.contains("register_migration"));
    }

    /// Read-only clone must recover a poisoned lock instead of swallowing it.
    #[test]
    fn read_recovers_poisoned_lock() {
        let registry = poisoned_registry();
        let guard = lock_registry_for_read(&registry);
        assert!(guard.is_empty());
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
