//! End-to-end `migrate` command test against a real SQLite pool.
//!
//! Verifies the CLI opens a pool from `DATABASE_URL` and executes the
//! process-wide migration registry — applying, idempotent re-run, `--fresh`,
//! and `--seed` — rather than echoing SQL.

use rustasea_cli::Artisan;
use rustasea_orm::{register_migration, register_seeder, Migration, Result as OrmResult, Seeder};

/// Serializes tests that mutate process-global environment variables.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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

/// Idempotently seeds one user row.
struct UserSeeder;

impl Seeder for UserSeeder {
    fn name(&self) -> &str {
        "UserSeeder"
    }

    fn sql(&self) -> OrmResult<String> {
        Ok(
            "INSERT INTO users (id, email) VALUES (1, 'ada@example.test') ON CONFLICT DO NOTHING"
                .into(),
        )
    }
}

/// Runs the `migrate` lifecycle end to end.
#[tokio::test]
async fn migrate_command_executes_against_pool() {
    let db_path = std::env::temp_dir().join(format!(
        "rustasea-cli-migrate-{}-{}.sqlite",
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

    register_migration(CreateUsers);
    register_seeder(UserSeeder);
    rustasea_cli::load_default_commands();

    let first = Artisan::call("migrate", vec![]).await.expect("call");
    assert!(first.is_success(), "stderr: {}", first.stderr);
    assert!(
        first.stdout.contains("migrated: 0001_create_users_table"),
        "stdout: {}",
        first.stdout
    );

    let second = Artisan::call("migrate", vec![]).await.expect("call");
    assert!(second.is_success(), "stderr: {}", second.stderr);
    assert!(
        second.stdout.contains("Nothing to migrate."),
        "second run must be a no-op, stdout: {}",
        second.stdout
    );

    let fresh = Artisan::call("migrate", vec!["--fresh".into(), "--seed".into()])
        .await
        .expect("call");
    assert!(fresh.is_success(), "stderr: {}", fresh.stderr);
    assert!(
        fresh.stdout.contains("Dropped all tables"),
        "stdout: {}",
        fresh.stdout
    );
    assert!(
        fresh.stdout.contains("seeded: UserSeeder"),
        "stdout: {}",
        fresh.stdout
    );

    let fresh_sig = Artisan::call("migrate:fresh", vec!["--seed".into()])
        .await
        .expect("call");
    assert!(fresh_sig.is_success(), "stderr: {}", fresh_sig.stderr);
    assert!(
        fresh_sig.stdout.contains("Dropped all tables"),
        "stdout: {}",
        fresh_sig.stdout
    );

    let _ = std::fs::remove_file(&db_path);
    std::env::remove_var("DATABASE_URL");
}
