//! Integration round-trip for `Migrator` against a live SQLite pool.
//!
//! Covers the QA contract from `test-orm.md §5`: up → present, idempotent
//! re-run, rollback removes the schema, `fresh` resets, seeders execute, and an
//! irreversible migration surfaces [`MigrationError::Irreversible`].

use rustasea_orm::migration::MigrationError;
use rustasea_orm::{DbPool, Migration, Migrator, Result as OrmResult, SqlSeeder, Value};

/// Creates the `users` table.
struct CreateUsers;

impl Migration for CreateUsers {
    fn name(&self) -> &str {
        "0001_create_users_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL);".into())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE users;".into())
    }
}

/// Creates the `posts` table (second migration, same batch).
struct CreatePosts;

impl Migration for CreatePosts {
    fn name(&self) -> &str {
        "0002_create_posts_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL);".into())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE posts;".into())
    }
}

/// Migration with no `down` override (irreversible by default).
struct CreateAuditLog;

impl Migration for CreateAuditLog {
    fn name(&self) -> &str {
        "0003_create_audit_log_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE audit_log (id INTEGER PRIMARY KEY);".into())
    }
}

/// Migration whose `up` emits syntactically invalid SQL (exec fails in the tx).
struct BrokenMigration;

impl Migration for BrokenMigration {
    fn name(&self) -> &str {
        "0004_broken_migration"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE broken (".into())
    }
}

/// Migration that creates `alpha` but shares a name with [`DuplicateBeta`], so
/// the tracking-row insert for the second one violates the primary key.
struct DuplicateAlpha;

impl Migration for DuplicateAlpha {
    fn name(&self) -> &str {
        "0005_duplicate"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE alpha (id INTEGER PRIMARY KEY);".into())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE alpha;".into())
    }
}

/// Migration that creates `beta` under the same name as [`DuplicateAlpha`].
struct DuplicateBeta;

impl Migration for DuplicateBeta {
    fn name(&self) -> &str {
        "0005_duplicate"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE beta (id INTEGER PRIMARY KEY);".into())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE beta;".into())
    }
}

/// Creates the `accounts` table (first of a two-migration rollback batch).
struct CreateAccounts;

impl Migration for CreateAccounts {
    fn name(&self) -> &str {
        "0006_create_accounts_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE accounts (id INTEGER PRIMARY KEY);".into())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE accounts;".into())
    }
}

/// Migration whose `down` partially reverts then fails, so the whole revert must
/// roll back and its tracking row must stay intact.
struct FailingDown;

impl Migration for FailingDown {
    fn name(&self) -> &str {
        "0007_failing_down"
    }

    fn up(&self) -> OrmResult<String> {
        Ok("CREATE TABLE widgets (id INTEGER PRIMARY KEY);".into())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE widgets; THIS IS NOT VALID SQL;".into())
    }
}

/// In-memory SQLite pool for one test.
async fn pool() -> DbPool {
    DbPool::connect("sqlite::memory:")
        .await
        .expect("sqlite in-memory pool")
}

/// Whether `table` exists in the pool's schema.
async fn table_exists(pool: &DbPool, table: &str) -> bool {
    let rows = pool
        .fetch_json(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = $1",
            &[Value::Text(table.to_string())],
        )
        .await
        .expect("schema query");
    !rows.is_empty()
}

/// Names recorded in the `migrations` tracking table.
async fn recorded_names(pool: &DbPool) -> Vec<String> {
    let rows = pool
        .fetch_json("SELECT name FROM migrations ORDER BY name", &[])
        .await
        .expect("tracking query");
    rows.iter()
        .filter_map(|row| row.get("name").and_then(|n| n.as_str()).map(str::to_string))
        .collect()
}

/// Verifies up creates the schema, a second run is a no-op, and rollback
/// reverses the last batch in reverse order.
#[tokio::test]
async fn migrate_fresh_and_rollback_roundtrip() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(CreateUsers).add(CreatePosts);

    let applied = migrator.run(&pool).await.expect("first run");
    assert_eq!(
        applied,
        vec![
            "0001_create_users_table".to_string(),
            "0002_create_posts_table".to_string()
        ]
    );
    assert!(table_exists(&pool, "users").await);
    assert!(table_exists(&pool, "posts").await);

    let second = migrator.run(&pool).await.expect("idempotent run");
    assert!(second.is_empty(), "second run must be a no-op");

    let rolled_back = migrator.rollback(&pool).await.expect("rollback");
    assert_eq!(
        rolled_back,
        vec![
            "0002_create_posts_table".to_string(),
            "0001_create_users_table".to_string()
        ],
        "rollback must reverse batch order"
    );
    assert!(!table_exists(&pool, "users").await);
    assert!(!table_exists(&pool, "posts").await);

    let reapplied = migrator.fresh(&pool).await.expect("fresh");
    assert_eq!(reapplied.len(), 2);
    assert!(table_exists(&pool, "users").await);
    assert!(table_exists(&pool, "posts").await);
}

/// Verifies `fresh` drops pre-existing tables before re-running.
#[tokio::test]
async fn fresh_drops_all_tables() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(CreateUsers);

    migrator.run(&pool).await.expect("initial run");
    pool.execute_script("CREATE TABLE stray (id INTEGER PRIMARY KEY);")
        .await
        .expect("stray table");
    assert!(table_exists(&pool, "stray").await);

    migrator.fresh(&pool).await.expect("fresh");
    assert!(!table_exists(&pool, "stray").await, "fresh drops every table");
    assert!(table_exists(&pool, "users").await);
}

/// Verifies an irreversible migration surfaces the typed error on rollback.
#[tokio::test]
async fn irreversible_migration_errors_on_rollback() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(CreateAuditLog);
    migrator.run(&pool).await.expect("apply irreversible");

    let error = migrator.rollback(&pool).await.expect_err("must be irreversible");
    assert!(matches!(
        error,
        rustasea_orm::OrmError::Migration(MigrationError::Irreversible { name })
            if name == "0003_create_audit_log_table"
    ));
}

/// Verifies a registered seeder executes idempotently against the pool.
#[tokio::test]
async fn seeder_runs_idempotently() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(CreateUsers);
    migrator.add_seeder(SqlSeeder {
        name: "UserSeeder".into(),
        statements: vec![
            "INSERT INTO users (id, email) VALUES (1, 'ada@example.test') ON CONFLICT DO NOTHING"
                .into(),
        ],
    });

    migrator.run(&pool).await.expect("migrate");
    let ran = migrator.seed(&pool).await.expect("seed");
    assert_eq!(ran, vec!["UserSeeder".to_string()]);
    migrator.seed(&pool).await.expect("idempotent seed");

    let rows = pool
        .fetch_json("SELECT COUNT(*) AS count FROM users", &[])
        .await
        .expect("count query");
    assert_eq!(
        rows[0].get("count").and_then(|c| c.as_i64()),
        Some(1),
        "seeder must not duplicate rows"
    );
}

/// Verifies a failing later migration does not corrupt an earlier migration's
/// applied/tracked state (no schema applied-but-untracked).
#[tokio::test]
async fn failing_later_migration_leaves_earlier_state_consistent() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(CreateUsers).add(BrokenMigration);

    migrator
        .run(&pool)
        .await
        .expect_err("broken second migration must fail");

    assert!(
        table_exists(&pool, "users").await,
        "first migration's schema must persist"
    );
    assert_eq!(
        recorded_names(&pool).await,
        vec!["0001_create_users_table".to_string()],
        "first migration must be recorded; failed one must not"
    );
    assert!(
        !table_exists(&pool, "broken").await,
        "failed migration must leave no schema behind"
    );

    let retry = migrator.run(&pool).await;
    assert!(retry.is_err(), "retry still fails on the broken migration");
    assert_eq!(
        recorded_names(&pool).await,
        vec!["0001_create_users_table".to_string()],
        "retry must not re-apply or re-record the completed migration"
    );
}

/// Verifies a `record` failure rolls back the migration's schema change, so the
/// schema never lands without its tracking row.
#[tokio::test]
async fn record_failure_rolls_back_schema_change() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(DuplicateAlpha).add(DuplicateBeta);

    migrator
        .run(&pool)
        .await
        .expect_err("duplicate tracking row must fail");

    assert!(
        table_exists(&pool, "alpha").await,
        "committed migration's schema must persist"
    );
    assert_eq!(
        recorded_names(&pool).await,
        vec!["0005_duplicate".to_string()],
        "exactly one tracking row survives"
    );
    assert!(
        !table_exists(&pool, "beta").await,
        "schema from the record-failed migration must be rolled back"
    );
}

/// Verifies a failing `down` in a two-migration batch reverts atomically: the
/// failed migration's row and schema stay intact, and the earlier migration in
/// reverse order remains applied and recorded for a later retry.
#[tokio::test]
async fn failing_rollback_leaves_batch_consistent() {
    let pool = pool().await;
    let mut migrator = Migrator::new();
    migrator.add(CreateAccounts).add(FailingDown);

    migrator.run(&pool).await.expect("apply batch");
    assert!(table_exists(&pool, "accounts").await);
    assert!(table_exists(&pool, "widgets").await);

    migrator
        .rollback(&pool)
        .await
        .expect_err("first down in reverse order must fail");

    assert!(
        table_exists(&pool, "widgets").await,
        "failed migration's schema must survive its rolled-back revert"
    );
    assert!(
        table_exists(&pool, "accounts").await,
        "second migration must stay applied when the first revert fails"
    );
    assert_eq!(
        recorded_names(&pool).await,
        vec![
            "0006_create_accounts_table".to_string(),
            "0007_failing_down".to_string()
        ],
        "both tracking rows must stay intact for retry"
    );

    migrator
        .rollback(&pool)
        .await
        .expect_err("retry must fail identically");
    assert_eq!(
        recorded_names(&pool).await,
        vec![
            "0006_create_accounts_table".to_string(),
            "0007_failing_down".to_string()
        ],
        "retry must not corrupt batch membership"
    );
}
