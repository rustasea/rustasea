//! Async query execution on [`QueryBuilder`] — real round-trips over a [`DbPool`].
//!
//! These methods render the existing parameterized SQL ([`QueryBuilder::to_sql`])
//! and hand it to the driver. Rows are returned as JSON objects so the executor
//! stays model-agnostic; typed callers deserialize with [`json_to_model`]. Only
//! the `sqlx` runtime API is used — never the compile-time `query!` macros.

use super::QueryBuilder;
use crate::db::DbPool;
use crate::error::{OrmError, Result};
use crate::execution::Paginator;

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

impl QueryBuilder {
    /// Execute the query and return every matching row as a JSON object.
    pub async fn get(self, pool: &DbPool) -> Result<Vec<serde_json::Value>> {
        let sql = self.to_sql()?;
        let bindings = self.bindings().to_vec();
        pool.fetch_json(&sql, &bindings).await
    }

    /// Execute the query with `LIMIT 1` and return the first row, if any.
    pub async fn first(self, pool: &DbPool) -> Result<Option<serde_json::Value>> {
        let rows = self.limit(1).get(pool).await?;
        Ok(rows.into_iter().next())
    }

    /// Execute the query with `LIMIT 1`, returning [`OrmError::NotFound`] on empty.
    pub async fn first_or_fail(self, pool: &DbPool) -> Result<serde_json::Value> {
        self.first(pool).await?.ok_or(OrmError::NotFound)
    }

    /// Run the `COUNT(*)` projection over the current filters.
    pub async fn count(self, pool: &DbPool) -> Result<u64> {
        let sql = crate::execution::count_sql(&self)?;
        let bindings = self.bindings().to_vec();
        let rows = pool.fetch_json(&sql, &bindings).await?;
        Ok(scalar_count(rows.first()))
    }

    /// Whether at least one row matches the current filters.
    pub async fn exists(self, pool: &DbPool) -> Result<bool> {
        Ok(self.first(pool).await?.is_some())
    }

    /// Offset-paginate the query, running the COUNT and the windowed SELECT.
    ///
    /// Returns a [`Paginator`] carrying `items`/`data`, `meta`, and generated
    /// `links`; `page` and `per_page` are floored at 1.
    pub async fn paginate(
        self,
        pool: &DbPool,
        page: u64,
        per_page: u64,
    ) -> Result<Paginator<serde_json::Value>> {
        let page = page.max(1);
        let per_page = per_page.max(1);

        let count_sql = crate::execution::count_sql(&self)?;
        let count_bindings = self.bindings().to_vec();
        let count_rows = pool.fetch_json(&count_sql, &count_bindings).await?;
        let total = scalar_count(count_rows.first());

        let offset = page.saturating_sub(1).saturating_mul(per_page);
        let window = self.limit(per_page).offset(offset);
        let items = window.get(pool).await?;

        Ok(Paginator::new(items, page, per_page, total).with_links())
    }

    /// Execute the query in `size`-row windows, invoking `callback` per window.
    ///
    /// Iterates with LIMIT/OFFSET until a short page is returned, so every window
    /// is bounded and no full-table load occurs. Returns the processed row count.
    pub async fn chunk_by<F>(
        self,
        pool: &DbPool,
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
        let mut offset = self.offset.unwrap_or(0);
        let mut processed: u64 = 0;
        loop {
            let window = self.clone().limit(size).offset(offset);
            let rows = window.get(pool).await?;
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
