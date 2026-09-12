//! Async query execution on [`QueryBuilder`] — real round-trips over a [`DbPool`]
//! or an open [`Transaction`].
//!
//! These methods render the existing parameterized SQL ([`QueryBuilder::to_sql`])
//! and hand it to the [`Executor`], which is either a connection pool or a live
//! transaction. Rows are returned as JSON objects so the executor stays
//! model-agnostic; typed callers deserialize with [`json_to_model`]. Only the
//! `sqlx` runtime API is used — never the compile-time `query!` macros.

use super::QueryBuilder;
use crate::db::DbPool;
use crate::error::{OrmError, Result};
use crate::execution::Paginator;
use crate::tx::Transaction;
use crate::types::Value;

/// Deserialize a JSON row into a concrete model or value type.
///
/// A shape mismatch (missing column, wrong type) surfaces as [`OrmError::Storage`].
pub(crate) fn json_to_model<T>(value: serde_json::Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|error| OrmError::Storage(format!("row decode failed: {error}")))
}

/// A query execution target — a connection pool or an open transaction.
///
/// Both variants expose the same `fetch_json`/`execute_bind` surface, so a
/// [`QueryBuilder`] runs unchanged whether it is handed a [`DbPool`] or a
/// [`Transaction`]. The transaction variant borrows the handle mutably so every
/// statement lands on the same connection and stays inside the transaction.
#[derive(Debug)]
pub enum Executor<'a> {
    /// Execute against a connection pool (autocommit).
    Pool(&'a DbPool),
    /// Execute against an open transaction (statements join the transaction).
    Transaction(&'a mut Transaction),
}

impl<'a> From<&'a DbPool> for Executor<'a> {
    /// Wrap a pool reference as an executor.
    fn from(pool: &'a DbPool) -> Self {
        Executor::Pool(pool)
    }
}

impl<'a> From<&'a mut Transaction> for Executor<'a> {
    /// Wrap an open transaction as an executor.
    fn from(tx: &'a mut Transaction) -> Self {
        Executor::Transaction(tx)
    }
}

impl<'a> Executor<'a> {
    /// Run a parameterized `SELECT`, decoding every row into a JSON object.
    ///
    /// The statement must use `$n` positional placeholders matching `bindings`;
    /// the driver rewrites them to the active dialect's native shape.
    pub async fn fetch_json(
        &mut self,
        sql: &str,
        bindings: &[Value],
    ) -> Result<Vec<serde_json::Value>> {
        match self {
            Executor::Pool(pool) => pool.fetch_json(sql, bindings).await,
            Executor::Transaction(tx) => tx.fetch_json(sql, bindings).await,
        }
    }

    /// Run a statement and return the number of affected rows.
    pub async fn execute_bind(&mut self, sql: &str, bindings: &[Value]) -> Result<u64> {
        match self {
            Executor::Pool(pool) => pool.execute_bind(sql, bindings).await,
            Executor::Transaction(tx) => tx.execute_bind(sql, bindings).await,
        }
    }

    /// Execute a `;`-separated SQL script with no bind values.
    ///
    /// Used by migration/seed bodies containing multiple statements; the script
    /// is sent verbatim — never interpolate user input.
    pub async fn execute_script(&mut self, sql: &str) -> Result<()> {
        match self {
            Executor::Pool(pool) => pool.execute_script(sql).await,
            Executor::Transaction(tx) => tx.execute_script(sql).await,
        }
    }

    /// Reborrow the executor so multi-statement methods can reuse it.
    fn reborrow(&mut self) -> Executor<'_> {
        match self {
            Executor::Pool(pool) => Executor::Pool(pool),
            Executor::Transaction(tx) => Executor::Transaction(tx),
        }
    }
}

impl QueryBuilder {
    /// Execute the query and return every matching row as a JSON object.
    ///
    /// Works against a [`DbPool`] (`.get(&pool)`) or an open [`Transaction`]
    /// (`.get(&mut tx)`); the two are unified by [`Executor`].
    pub async fn get<'a>(
        self,
        executor: impl Into<Executor<'a>>,
    ) -> Result<Vec<serde_json::Value>> {
        let sql = self.to_sql()?;
        let bindings = self.bindings().to_vec();
        executor.into().fetch_json(&sql, &bindings).await
    }

    /// Execute the query with `LIMIT 1` and return the first row, if any.
    pub async fn first<'a>(
        self,
        executor: impl Into<Executor<'a>>,
    ) -> Result<Option<serde_json::Value>> {
        let rows = self.limit(1).get(executor).await?;
        Ok(rows.into_iter().next())
    }

    /// Execute the query with `LIMIT 1`, returning [`OrmError::NotFound`] on empty.
    pub async fn first_or_fail<'a>(
        self,
        executor: impl Into<Executor<'a>>,
    ) -> Result<serde_json::Value> {
        self.first(executor).await?.ok_or(OrmError::NotFound)
    }

    /// Run the `COUNT(*)` projection over the current filters.
    pub async fn count<'a>(self, executor: impl Into<Executor<'a>>) -> Result<u64> {
        let sql = crate::execution::count_sql(&self)?;
        let bindings = self.bindings().to_vec();
        let rows = executor.into().fetch_json(&sql, &bindings).await?;
        Ok(scalar_count(rows.first()))
    }

    /// Whether at least one row matches the current filters.
    pub async fn exists<'a>(self, executor: impl Into<Executor<'a>>) -> Result<bool> {
        Ok(self.first(executor).await?.is_some())
    }

    /// Offset-paginate the query, running the COUNT and the windowed SELECT.
    ///
    /// Returns a [`Paginator`] carrying `items`/`data`, `meta`, and generated
    /// `links`; `page` and `per_page` are floored at 1.
    pub async fn paginate<'a>(
        self,
        executor: impl Into<Executor<'a>>,
        page: u64,
        per_page: u64,
    ) -> Result<Paginator<serde_json::Value>> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let mut executor = executor.into();

        let count_sql = crate::execution::count_sql(&self)?;
        let count_bindings = self.bindings().to_vec();
        let count_rows = executor.fetch_json(&count_sql, &count_bindings).await?;
        let total = scalar_count(count_rows.first());

        let offset = page.saturating_sub(1).saturating_mul(per_page);
        let window = self.limit(per_page).offset(offset);
        let items = window.get(executor.reborrow()).await?;

        Ok(Paginator::new(items, page, per_page, total).with_links())
    }

    /// Execute the query in `size`-row windows, invoking `callback` per window.
    ///
    /// Iterates with LIMIT/OFFSET until a short page is returned, so every window
    /// is bounded and no full-table load occurs. Returns the processed row count.
    pub async fn chunk_by<'a, F>(
        self,
        executor: impl Into<Executor<'a>>,
        size: u64,
        mut callback: F,
    ) -> Result<u64>
    where
        F: FnMut(Vec<serde_json::Value>) -> Result<()>,
    {
        if size == 0 {
            return Err(OrmError::InvalidState(
                "chunk_by size must be greater than zero".into(),
            ));
        }
        let mut executor = executor.into();
        let mut offset = self.offset.unwrap_or(0);
        let mut processed: u64 = 0;
        loop {
            let window = self.clone().limit(size).offset(offset);
            let rows = window.get(executor.reborrow()).await?;
            let fetched = rows.len() as u64;
            if fetched == 0 {
                break;
            }
            callback(rows)?;
            processed += fetched;
            if fetched < size {
                break;
            }
            offset += size;
        }
        Ok(processed)
    }
}

/// Read a scalar count from the first column of a COUNT row.
///
/// Drivers name the aggregate column differently (`COUNT(*)`, `count`), so the
/// first column value is read regardless of key.
fn scalar_count(row: Option<&serde_json::Value>) -> u64 {
    row.and_then(|value| value.as_object())
        .and_then(|object| object.values().next())
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}
