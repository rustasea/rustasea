//! Real transaction lifecycle over `sqlx::Transaction`.
//!
//! [`Transaction`] wraps a live driver transaction started from a [`DbPool`].
//! [`Transaction::begin`] opens it, [`Transaction::commit`] / [`Transaction::rollback`]
//! close it, and the executor methods ([`Transaction::fetch_json`] /
//! [`Transaction::execute_bind`]) run statements on the same connection. A
//! transaction is single-use: the inner handle is taken on the first
//! commit/rollback, so a second call is rejected with [`TransactionError::Closed`].
//! Only the `sqlx` runtime API is used — never the compile-time `query!` macros.
//!
//! # Non-nesting invariant
//!
//! Do not call [`crate::execution::transaction`] inside a `transaction()` body —
//! a nested call acquires a second connection and will deadlock; use the passed
//! `&mut Transaction` handle instead. Transactions are not savepoints: the inner
//! call would open a fresh connection while the outer one still holds its locks,
//! self-deadlocking on SQLite (single writer) and lock-waiting on MySQL/Postgres.

use crate::db::DbPool;
#[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
use crate::error::OrmError;
use crate::error::Result;
use crate::types::Value;

/// Transaction lifecycle errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransactionError {
    /// A commit/rollback/execute was attempted after the transaction closed.
    #[error("transaction is not open")]
    Closed,
}

/// A live driver transaction, one variant per compiled-in driver.
///
/// The handle is owned by [`Transaction`]; matching on this enum is the single
/// place driver-specific transaction dispatch lives, mirroring [`DbPool`].
#[derive(Debug)]
pub(crate) enum DbTransaction {
    /// SQLite transaction.
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::Transaction<'static, sqlx::Sqlite>),
    /// PostgreSQL transaction.
    #[cfg(feature = "postgres")]
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
    /// MySQL transaction.
    #[cfg(feature = "mysql")]
    MySql(sqlx::Transaction<'static, sqlx::MySql>),
}

/// An open transaction handle wrapping a live `sqlx` transaction.
///
/// The handle is single-use: committing or rolling back takes the inner
/// [`DbTransaction`], leaving the wrapper closed so a second call fails with
/// [`TransactionError::Closed`] instead of silently succeeding.
#[derive(Debug)]
pub struct Transaction {
    /// The live handle, or `None` once the transaction has been closed.
    inner: Option<DbTransaction>,
}

impl Transaction {
    /// Begin a transaction on `pool`, selecting the driver from the pool variant.
    ///
    /// A pool whose driver feature is not compiled in surfaces
    /// [`OrmError::UnsupportedDriver`]; driver failures surface as
    /// [`OrmError::Storage`] via the `sqlx` error conversion.
    pub async fn begin(pool: &DbPool) -> Result<Self> {
        let inner = match pool {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => DbTransaction::Sqlite(pool.begin().await?),
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => DbTransaction::Postgres(pool.begin().await?),
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => DbTransaction::MySql(pool.begin().await?),
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => {
                return Err(OrmError::UnsupportedDriver(
                    "no sqlx driver compiled in".into(),
                ))
            }
        };
        Ok(Self { inner: Some(inner) })
    }

    /// Whether the transaction is still open.
    pub fn is_open(&self) -> bool {
        self.inner.is_some()
    }

    /// The active driver dialect name, or `"closed"` once the handle is spent.
    pub fn dialect(&self) -> &'static str {
        match self.inner.as_ref() {
            Some(inner) => inner.dialect(),
            None => "closed",
        }
    }

    /// Commit the transaction, consuming the handle.
    ///
    /// Returns [`TransactionError::Closed`] (as [`OrmError::Transaction`]) when
    /// the transaction was already committed or rolled back.
    pub async fn commit(&mut self) -> Result<()> {
        let inner = self.inner.take().ok_or(TransactionError::Closed)?;
        inner.commit().await?;
        Ok(())
    }

    /// Roll back the transaction, consuming the handle.
    ///
    /// Returns [`TransactionError::Closed`] (as [`OrmError::Transaction`]) when
    /// the transaction was already committed or rolled back.
    pub async fn rollback(&mut self) -> Result<()> {
        let inner = self.inner.take().ok_or(TransactionError::Closed)?;
        inner.rollback().await?;
        Ok(())
    }

    /// Run `sql` with `bindings` on the transaction connection, decoding rows.
    ///
    /// The statement must use `$n` positional placeholders matching `bindings`;
    /// they are rewritten to the active driver's native shape.
    pub async fn fetch_json(
        &mut self,
        sql: &str,
        bindings: &[Value],
    ) -> Result<Vec<serde_json::Value>> {
        let inner = self.inner.as_mut().ok_or(TransactionError::Closed)?;
        inner.fetch_json(sql, bindings).await
    }

    /// Run `sql` with `bindings` on the transaction connection, returning the
    /// affected row count.
    pub async fn execute_bind(&mut self, sql: &str, bindings: &[Value]) -> Result<u64> {
        let inner = self.inner.as_mut().ok_or(TransactionError::Closed)?;
        inner.execute_bind(sql, bindings).await
    }

    /// Execute a `;`-separated SQL script on the transaction connection.
    ///
    /// Used for multi-statement bodies (`sqlx::query` accepts one statement
    /// only); the script is sent verbatim — never interpolate user input.
    pub async fn execute_script(&mut self, sql: &str) -> Result<()> {
        let inner = self.inner.as_mut().ok_or(TransactionError::Closed)?;
        inner.execute_script(sql).await
    }
}

impl DbTransaction {
    /// Commit the underlying driver transaction.
    pub(crate) async fn commit(self) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            DbTransaction::Sqlite(tx) => tx.commit().await?,
            #[cfg(feature = "postgres")]
            DbTransaction::Postgres(tx) => tx.commit().await?,
            #[cfg(feature = "mysql")]
            DbTransaction::MySql(tx) => tx.commit().await?,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => {}
        }
        Ok(())
    }

    /// Roll back the underlying driver transaction.
    pub(crate) async fn rollback(self) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            DbTransaction::Sqlite(tx) => tx.rollback().await?,
            #[cfg(feature = "postgres")]
            DbTransaction::Postgres(tx) => tx.rollback().await?,
            #[cfg(feature = "mysql")]
            DbTransaction::MySql(tx) => tx.rollback().await?,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => {}
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    /// Opens an in-memory pool with a `t` table for transaction assertions.
    async fn pool() -> DbPool {
        let pool = DbPool::connect("sqlite::memory:").await.unwrap();
        pool.execute_bind(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            &[],
        )
        .await
        .unwrap();
        pool
    }

    /// Verifies commit persists a row and closes the handle.
    #[tokio::test]
    async fn commit_persists_and_closes() {
        let pool = pool().await;
        let mut tx = Transaction::begin(&pool).await.unwrap();
        tx.execute_bind(
            "INSERT INTO t (id, name) VALUES ($1, $2)",
            &[Value::Int(1), Value::Text("a".into())],
        )
        .await
        .unwrap();
        assert!(tx.is_open());
        tx.commit().await.unwrap();
        assert!(!tx.is_open());

        let rows = pool.fetch_json("SELECT id FROM t", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
        pool.close().await;
    }

    /// Verifies double-commit is a typed, observable error.
    #[tokio::test]
    async fn double_commit_rejected() {
        let pool = pool().await;
        let mut tx = Transaction::begin(&pool).await.unwrap();
        tx.commit().await.unwrap();
        let error = tx.commit().await.unwrap_err();
        assert!(
            matches!(
                error,
                crate::error::OrmError::Transaction(TransactionError::Closed)
            ),
            "got {error:?}"
        );
        pool.close().await;
    }

    /// Verifies a rolled-back row is absent.
    #[tokio::test]
    async fn rollback_discards_writes() {
        let pool = pool().await;
        let mut tx = Transaction::begin(&pool).await.unwrap();
        tx.execute_bind(
            "INSERT INTO t (id, name) VALUES ($1, $2)",
            &[Value::Int(2), Value::Text("b".into())],
        )
        .await
        .unwrap();
        tx.rollback().await.unwrap();

        let rows = pool.fetch_json("SELECT id FROM t", &[]).await.unwrap();
        assert!(rows.is_empty());
        pool.close().await;
    }
}
