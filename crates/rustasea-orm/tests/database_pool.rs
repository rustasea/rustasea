//! Integration tests for the real `sqlx` pool facade.
//!
//! Enabled only with the `sqlite` feature and backed by an in-memory database,
//! so they need no external service. Traces to sprint S03-T01 and the
//! `QueryError::PoolClosed` chaos case (DATA-MNK-006).

#![cfg(feature = "sqlite")]

use rustasea_orm::{DbPool, OrmError, PoolConfig, Value};

/// Build a single-connection in-memory SQLite pool so schema state persists.
fn memory_pool() -> DbPool {
    let config = PoolConfig {
        min: 1,
        max: 1,
        idle_timeout: 600,
    };
    DbPool::connect_lazy_with("sqlite::memory:", &config).unwrap()
}

/// Verifies execute/fetch_all/fetch_one against a real SQLite database.
#[tokio::test]
async fn execute_and_fetch_roundtrip() {
    let pool = memory_pool();
    pool.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )
    .await
    .unwrap();
    let inserted = pool
        .execute(
            "INSERT INTO users (id, name) VALUES ($1, $2)",
            &[Value::Int(1), Value::Text("ada".into())],
        )
        .await
        .unwrap();
    assert_eq!(inserted, 1);

    let rows = pool
        .fetch_all("SELECT id, name FROM users ORDER BY id", &[])
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("id"), Some(&Value::Int(1)));
    assert_eq!(rows[0].get("name"), Some(&Value::Text("ada".into())));

    let row = pool
        .fetch_one("SELECT name FROM users WHERE id = $1", &[Value::Int(1)])
        .await
        .unwrap();
    assert_eq!(row.get("name"), Some(&Value::Text("ada".into())));
}

/// Verifies a missing row maps to NotFound and fetch_optional yields None.
#[tokio::test]
async fn fetch_one_missing_row_is_not_found() {
    let pool = memory_pool();
    pool.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)", &[])
        .await
        .unwrap();

    let err = pool
        .fetch_one("SELECT id FROM users WHERE id = $1", &[Value::Int(99)])
        .await
        .unwrap_err();
    assert!(matches!(err, OrmError::NotFound));

    let optional = pool
        .fetch_optional("SELECT id FROM users WHERE id = $1", &[Value::Int(99)])
        .await
        .unwrap();
    assert!(optional.is_none());
}
