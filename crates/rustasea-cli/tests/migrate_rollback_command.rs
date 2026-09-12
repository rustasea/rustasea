//! End-to-end `migrate:rollback` command test against a real SQLite pool.
//!
//! Verifies the CLI opens a pool from `DATABASE_URL` and reverts the process-wide
//! migration registry's most recent batch via `Migrator::rollback` — covering the
//! default step, the `--step`/`--step=N` forms, the empty-ledger no-op, and
//! invalid-step rejection.

use rustasea_cli::Artisan;
use rustasea_orm::{register_migration, Migration, Result as OrmResult};

/// Serializes tests that mutate process-global environment variables.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Creates and drops the `widgets` table.
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

/// Runs the `migrate:rollback` lifecycle end to end.
#[tokio::test]
async fn migrate_rollback_reverts_last_batch() {
    let db_path = std::env::temp_dir().join(format!(
        "rustasea-cli-rollback-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    let url = format!("sqlite://{}?mode=rwc", db_path.display());
    // Pin this test to its own SQLite file. `database_url()` consults the
    // environment before any workspace `config/database.toml`, so the run is
    // deterministic regardless of the test working directory.
    let _env = ENV_LOCK.lock().await;
    std::env::remove_var("DATABASE__URL");
    std::env::set_var("DATABASE_URL", &url);

    register_migration(CreateWidgets);
    rustasea_cli::load_default_commands();

    let applied = Artisan::call("migrate", vec![]).await.expect("call");
    assert!(applied.is_success(), "stderr: {}", applied.stderr);
    assert!(
        applied
            .stdout
            .contains("migrated: 0001_create_widgets_table"),
        "stdout: {}",
        applied.stdout
    );

    // Default rollback reverts the single batch (widgets plus the queue tables).
    let rolled = Artisan::call("migrate:rollback", vec![])
        .await
        .expect("call");
    assert!(rolled.is_success(), "stderr: {}", rolled.stderr);
    assert!(
        rolled
            .stdout
            .contains("rolled back: 0001_create_widgets_table"),
        "stdout: {}",
        rolled.stdout
    );

    // Re-applying proves the `down` body actually dropped the table: a stale
    // `CREATE TABLE widgets` would fail with "table already exists".
    let reapplied = Artisan::call("migrate", vec![]).await.expect("call");
    assert!(reapplied.is_success(), "stderr: {}", reapplied.stderr);
    assert!(
        reapplied
            .stdout
            .contains("migrated: 0001_create_widgets_table"),
        "stdout: {}",
        reapplied.stdout
    );

    // `--step 2` with only one batch left reverts that batch and stops.
    let stepped = Artisan::call("migrate:rollback", vec!["--step".into(), "2".into()])
        .await
        .expect("call");
    assert!(stepped.is_success(), "stderr: {}", stepped.stderr);
    assert!(
        stepped
            .stdout
            .contains("rolled back: 0001_create_widgets_table"),
        "stdout: {}",
        stepped.stdout
    );

    // An empty ledger is a clean no-op; the `--step=N` form is accepted too.
    let empty = Artisan::call("migrate:rollback", vec!["--step=1".into()])
        .await
        .expect("call");
    assert!(empty.is_success(), "stderr: {}", empty.stderr);
    assert!(
        empty.stdout.contains("Nothing to roll back."),
        "stdout: {}",
        empty.stdout
    );

    // A non-positive step is rejected before any database work (exit code 3).
    let invalid = Artisan::call("migrate:rollback", vec!["--step".into(), "0".into()])
        .await
        .expect("call");
    assert!(!invalid.is_success(), "stdout: {}", invalid.stdout);
    assert_eq!(invalid.exit_code, 3, "stderr: {}", invalid.stderr);
    assert!(
        invalid
            .stderr
            .contains("invalid arguments for `migrate:rollback`"),
        "stderr: {}",
        invalid.stderr
    );

    let _ = std::fs::remove_file(&db_path);
    std::env::remove_var("DATABASE_URL");
}
