//! Test-database migration hook — apply migrations once per test binary.
//!
//! [`migrate_once`] runs a [`Migrator`] against a live pool; the `migrations`
//! tracking table makes it a no-op after the first call for a given pool, so a
//! shared Postgres/SQLite container migrates exactly once per binary while a
//! fresh in-memory database still gets its own schema. Implement
//! [`MigrateHarness`] to expose the hook through [`crate::TestCase`].

use async_trait::async_trait;

use rustasea_orm::{DbPool, Migrator};

/// Apply every pending migration against `pool`.
///
/// Safe to call before each test: the `migrations` table records applied names,
/// so only the first call for a given pool performs work. Returns the names
/// applied by this call (empty when already current).
pub async fn migrate_once(pool: &DbPool, migrator: &Migrator) -> crate::Result<Vec<String>> {
    let applied = migrator.run(pool).await?;
    Ok(applied)
}

/// Opt-in hook applying a [`Migrator`] to a test database once per binary.
///
/// Implementors supply their application's [`Migrator`] via
/// [`MigrateHarness::migrator`]; the default [`MigrateHarness::migrate`] applies
/// it to the supplied pool. SQLite is always available; Postgres/MySQL work
/// when the matching `rustasea-orm` driver feature is enabled.
#[async_trait]
pub trait MigrateHarness: crate::TestCase {
    /// The migrations this test binary requires.
    fn migrator(&self) -> Migrator {
        Migrator::new()
    }

    /// Apply the registered migrations to `pool` (idempotent per pool).
    async fn migrate(&self, pool: &DbPool) -> crate::Result<Vec<String>> {
        migrate_once(pool, &self.migrator()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustasea_orm::{Migration, Result as OrmResult};

    /// Minimal migration creating one table.
    struct CreateWidgets;

    impl Migration for CreateWidgets {
        fn name(&self) -> &str {
            "0001_create_widgets_table"
        }

        fn up(&self) -> OrmResult<String> {
            Ok("CREATE TABLE widgets (id INTEGER PRIMARY KEY);".into())
        }

        fn down(&self) -> OrmResult<String> {
            Ok("DROP TABLE widgets;".into())
        }
    }

    /// Verifies `migrate_once` applies once then no-ops for the same pool.
    #[tokio::test]
    async fn migrate_once_is_idempotent_per_pool() {
        let pool = DbPool::connect("sqlite::memory:").await.unwrap();
        let mut migrator = Migrator::new();
        migrator.add(CreateWidgets);

        let first = migrate_once(&pool, &migrator).await.unwrap();
        assert_eq!(first, vec!["0001_create_widgets_table".to_string()]);

        let second = migrate_once(&pool, &migrator).await.unwrap();
        assert!(second.is_empty());
    }
}
