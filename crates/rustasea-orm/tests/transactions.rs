//! Real transaction integration tests (M2-C) against in-memory SQLite.
//!
//! Exercises the [`Transaction`] handle and the [`transaction`] helper: commit
//! persists, an error mid-body rolls back, double-commit is rejected, and
//! `for_update` is driver-gated (`UnsupportedDriver` on SQLite). The sqlx
//! runtime API is used throughout — no compile-time `query!` macros.

use rustasea_orm::{transaction, DbPool, OrmError, QueryBuilder, Transaction, Value};
use uuid::Uuid;

/// Build a fresh in-memory pool with a `users` table applied.
async fn pool_with_users() -> DbPool {
    let pool = DbPool::connect("sqlite::memory:").await.unwrap();
    pool.execute_bind(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )
    .await
    .unwrap();
    pool
}

/// Build a fresh file-backed pool with a `users` table applied.
///
/// Unlike the single-connection `sqlite::memory:` pool, a file-backed pool can
/// open a second connection, letting a pool query observe committed state while
/// a transaction holds its own connection. The temp file path is returned so
/// the caller can clean it up.
async fn file_pool_with_users() -> (DbPool, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("rustasea-orm-tx-{}.db", Uuid::now_v7()));
    let pool = DbPool::connect(&format!("sqlite://{}", path.display()))
        .await
        .unwrap();
    pool.execute_bind("PRAGMA journal_mode=WAL", &[])
        .await
        .unwrap();
    pool.execute_bind("PRAGMA busy_timeout=5000", &[])
        .await
        .unwrap();
    pool.execute_bind(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )
    .await
    .unwrap();
    (pool, path)
}

/// Count rows in `users`.
async fn user_count(pool: &DbPool) -> u64 {
    let rows = pool
        .fetch_json("SELECT COUNT(*) AS n FROM users", &[])
        .await
        .unwrap();
    rows.first()
        .and_then(|row| row.get("n"))
        .and_then(|n| n.as_u64())
        .unwrap_or(0)
}

/// Verifies committing a transaction persists the inserted row.
#[tokio::test]
async fn commit_persists_row() {
    let pool = pool_with_users().await;
    let mut tx = Transaction::begin(&pool).await.unwrap();
    tx.execute_bind(
        "INSERT INTO users (id, name) VALUES ($1, $2)",
        &[Value::Int(1), Value::Text("Ada".into())],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(user_count(&pool).await, 1);
    pool.close().await;
}

/// Verifies an error mid-body rolls the transaction back (row absent).
#[tokio::test]
async fn error_mid_body_rolls_back() {
    let pool = pool_with_users().await;
    let error = transaction(&pool, |tx| {
        Box::pin(async move {
            tx.execute_bind(
                "INSERT INTO users (id, name) VALUES ($1, $2)",
                &[Value::Int(2), Value::Text("Grace".into())],
            )
            .await?;
            Err::<(), _>(OrmError::InvalidState("boom".into()))
        })
    })
    .await
    .unwrap_err();

    assert!(matches!(error, OrmError::InvalidState(_)), "got {error:?}");
    assert_eq!(user_count(&pool).await, 0);
    pool.close().await;
}

/// Verifies a successful body commits through the `transaction` helper.
#[tokio::test]
async fn transaction_helper_commits_on_ok() {
    let pool = pool_with_users().await;
    transaction(&pool, |tx| {
        Box::pin(async move {
            tx.execute_bind(
                "INSERT INTO users (id, name) VALUES ($1, $2)",
                &[Value::Int(3), Value::Text("Lin".into())],
            )
            .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    assert_eq!(user_count(&pool).await, 1);
    pool.close().await;
}

/// Verifies double-commit is rejected with a typed, observable error.
#[tokio::test]
async fn double_commit_rejected() {
    let pool = pool_with_users().await;
    let mut tx = Transaction::begin(&pool).await.unwrap();
    tx.commit().await.unwrap();

    let error = tx.commit().await.unwrap_err();
    assert!(
        matches!(
            error,
            OrmError::Transaction(rustasea_orm::TransactionError::Closed)
        ),
        "got {error:?}"
    );
    assert!(!tx.is_open());
    pool.close().await;
}

/// Verifies the compile-time `for_update` builder rejects SQLite.
///
/// [`QueryBuilder::for_update`] resolves its dialect at compile time
/// (`builder::dialect()`), so the `UnsupportedDriver` rejection is only emitted
/// when SQLite is the compiled dialect. The runtime-aware path used by
/// `ModelOps::first_for_update` (via `for_update_with_dialect`) is covered
/// against an SQLite pool with `postgres` compiled in by
/// `tests/model_ops.rs::first_for_update_rejects_sqlite_pool_at_runtime`.
#[cfg(all(feature = "sqlite", not(feature = "postgres"), not(feature = "mysql")))]
#[tokio::test]
async fn for_update_unsupported_on_sqlite() {
    let pool = pool_with_users().await;
    let mut tx = Transaction::begin(&pool).await.unwrap();
    let error = match QueryBuilder::table("users").for_update() {
        Ok(_) => panic!("sqlite has no row locks"),
        Err(error) => error,
    };
    assert!(
        matches!(error, OrmError::UnsupportedDriver(_)),
        "got {error:?}"
    );
    tx.rollback().await.unwrap();
    pool.close().await;
}

/// Remove a file-backed SQLite database and its WAL sidecars.
fn remove_sqlite_files(path: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
        let _ = std::fs::remove_file(candidate);
    }
}

/// Verifies a QueryBuilder run through `&mut tx` sees its own uncommitted row
/// while the pool cannot see it until commit, then can after commit.
#[tokio::test]
async fn query_builder_uses_transaction_visibility() {
    let (pool, path) = file_pool_with_users().await;
    let mut tx = Transaction::begin(&pool).await.unwrap();
    tx.execute_bind(
        "INSERT INTO users (id, name) VALUES ($1, $2)",
        &[Value::Int(1), Value::Text("Ada".into())],
    )
    .await
    .unwrap();

    let seen_in_tx = QueryBuilder::table("users")
        .where_eq("name", "Ada")
        .get(&mut tx)
        .await
        .unwrap();
    assert_eq!(
        seen_in_tx.len(),
        1,
        "open transaction must see its own uncommitted row"
    );

    let seen_in_pool = QueryBuilder::table("users")
        .where_eq("name", "Ada")
        .get(&pool)
        .await
        .unwrap();
    assert!(
        seen_in_pool.is_empty(),
        "pool must not see uncommitted rows before commit"
    );

    tx.commit().await.unwrap();

    let seen_after_commit = QueryBuilder::table("users")
        .where_eq("name", "Ada")
        .get(&pool)
        .await
        .unwrap();
    assert_eq!(
        seen_after_commit.len(),
        1,
        "pool must see the row after commit"
    );

    pool.close().await;
    remove_sqlite_files(&path);
}

/// Verifies `paginate` and `chunk_by` drive their multi-statement SQL through
/// one open transaction via the `Executor` reborrow path.
#[tokio::test]
async fn paginate_and_chunk_by_run_multi_statement_in_transaction() {
    let pool = pool_with_users().await;
    let mut tx = Transaction::begin(&pool).await.unwrap();
    for id in 1..=5 {
        tx.execute_bind(
            "INSERT INTO users (id, name) VALUES ($1, $2)",
            &[Value::Int(id), Value::Text(format!("user-{id}"))],
        )
        .await
        .unwrap();
    }

    let page = QueryBuilder::table("users")
        .paginate(&mut tx, 2, 2)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 2, "second page holds the window");
    assert_eq!(page.total, 5, "COUNT and SELECT share the transaction");
    assert_eq!(page.current_page, 2);

    let mut windows = 0usize;
    let mut rows = 0usize;
    let processed = QueryBuilder::table("users")
        .chunk_by(&mut tx, 2, |chunk| {
            windows += 1;
            rows += chunk.len();
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(windows, 3, "five rows chunk into 2 + 2 + 1");
    assert_eq!(rows, 5);
    assert_eq!(processed, 5);

    tx.commit().await.unwrap();
    assert_eq!(user_count(&pool).await, 5);
    pool.close().await;
}
