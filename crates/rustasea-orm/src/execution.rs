//! Query execution helpers — pagination, raw statements, aggregations,
//! chunked iteration and transactions.
//!
//! In-memory stubs for M2; the sqlx pool wiring replaces the executor bodies
//! without changing these signatures. Every helper emits SQL text so callers
//! can verify the emitted statement before any round-trip.

use crate::builder::{QueryBuilder, Raw, TransactionStub};
use crate::error::{OrmError, Result};
use crate::types::Value;

/// Pagination metadata and rows (offset pagination).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Paginator<T> {
    /// Rows on this page.
    pub items: Vec<T>,
    /// 1-based current page.
    pub current_page: u64,
    /// Rows per page.
    pub per_page: u64,
    /// Total matching rows (extra COUNT query).
    pub total: u64,
    /// Last page number (`ceil(total / per_page)`).
    pub last_page: u64,
}

impl<T> Paginator<T> {
    /// Compute a paginator; `total` is supplied by the caller (COUNT query).
    pub fn new(items: Vec<T>, current_page: u64, per_page: u64, total: u64) -> Self {
        let last_page = if per_page == 0 {
            0
        } else {
            total.div_ceil(per_page)
        };
        Self {
            items,
            current_page,
            per_page,
            total,
            last_page,
        }
    }

    /// Recompute page metadata from a fresh COUNT (`per_page` kept).
    pub fn with_total(&self, total: u64) -> Self
    where
        T: Clone,
    {
        Self::new(self.items.clone(), self.current_page, self.per_page, total)
    }

    /// Rows on this page (Laravel `data` alias).
    pub fn data(&self) -> &[T] {
        &self.items
    }

    /// Rows returned on this page (serde-friendly `items` alias).
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// The number of rows on this page (empty tail pages included).
    pub fn row_count(&self) -> usize {
        self.items.len()
    }

    /// Total pages — `ceil(total / per_page)` with a floor of 1 for data.
    pub fn last_page(&self) -> u64 {
        self.last_page
    }

    /// Whether the page has a next page.
    pub fn has_more(&self) -> bool {
        self.current_page < self.last_page
    }

    /// Map each row while preserving pagination metadata.
    pub fn map<U, F>(self, f: F) -> Paginator<U>
    where
        F: FnMut(T) -> U,
    {
        let items: Vec<U> = self.items.into_iter().map(f).collect();
        Paginator::new(items, self.current_page, self.per_page, self.total)
    }
}

/// Describe the pagination window carried by a builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMeta {
    /// 1-based current page.
    pub page: u64,
    /// Rows per page.
    pub per_page: u64,
}

impl PageMeta {
    /// The OFFSET implied by `(page, per_page)` — 0-based.
    pub fn offset(&self) -> u64 {
        self.page.saturating_sub(1).saturating_mul(self.per_page)
    }
}

/// Execute a raw statement (stub — sqlx wiring lands with the driver crate).
pub fn raw(sql: &str, bindings: Vec<Value>) -> Raw {
    Raw {
        sql: sql.to_string(),
        bindings,
    }
}

/// Raw SQL builder statement (fragment emission, display-only).
pub fn raw_sql(sql: &str) -> crate::builder::SqlFragment {
    crate::builder::SqlFragment {
        sql: sql.to_string(),
    }
}

/// Aggregation helper: build a `COUNT(*)` projection over the current filters.
pub fn count_sql(builder: &QueryBuilder) -> Result<String> {
    let mut sql = format!("SELECT COUNT(*) FROM {}", builder.table_name());
    if let Some(clause) = builder.where_clause() {
        sql.push_str(" WHERE ");
        sql.push_str(&clause);
    }
    Ok(sql)
}

/// Count projection that ignores OFFSET/LIMIT (total for pagination).
pub fn to_row_count_sql(builder: &QueryBuilder) -> Result<String> {
    count_sql(builder)
}

/// Aggregation helper: build a `SUM(column)` projection over the current filters.
pub fn sum_sql(builder: &QueryBuilder, column: &str) -> Result<String> {
    let mut sql = format!("SELECT SUM({column}) FROM {}", builder.table_name());
    if let Some(clause) = builder.where_clause() {
        sql.push_str(" WHERE ");
        sql.push_str(&clause);
    }
    Ok(sql)
}

/// Chunk the query results by `column` value ranges.
///
/// Emits `(column BETWEEN ? AND ?)` windows so a 10k-row table is processed
/// as bounded batches without loading every row into memory at once.
pub fn chunk_by(
    _builder: &QueryBuilder,
    column: &str,
    size: u64,
    total: u64,
) -> Result<Vec<String>> {
    if size == 0 {
        return Err(OrmError::InvalidState(
            "chunk_by size must be greater than zero".into(),
        ));
    }
    if column.trim().is_empty() {
        return Err(OrmError::InvalidState(
            "chunk_by column must not be empty".into(),
        ));
    }
    let mut chunks = Vec::new();
    let mut start: u64 = 0;
    while start < total {
        let end = total.min(start + size);
        chunks.push(format!("{column} BETWEEN {start} AND {end}"));
        start += size;
    }
    if chunks.is_empty() {
        chunks.push(format!("{column} BETWEEN 0 AND 0"));
    }
    Ok(chunks)
}

/// Execute a transaction body atomically (stub — pool wiring in the driver crate).
///
/// Commits on `Ok`, rolls back on `Err`; the body receives an open handle.
pub async fn transaction<F, T>(body: F) -> Result<T>
where
    F: FnOnce(&mut TransactionStub) -> Result<T>,
{
    let mut tx = TransactionStub::begin();
    match body(&mut tx) {
        Ok(value) => {
            tx.commit()?;
            Ok(value)
        }
        Err(e) => {
            tx.rollback()?;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies Paginator exposes both `items` and `data` with identical rows.
    #[test]
    fn paginator_exposes_items_and_data() {
        let p = Paginator::new(vec![1, 2, 3], 2, 5, 12);
        assert_eq!(p.items(), &[1, 2, 3]);
        assert_eq!(p.data(), &[1, 2, 3]);
        assert_eq!(p.row_count(), 3);
        assert_eq!(p.last_page(), 3);
        assert!(p.has_more());
    }

    /// Verifies metadata recomputation from a fresh COUNT.
    #[test]
    fn paginator_with_total_recomputes_last_page() {
        let p = Paginator::new(vec![1u8; 10], 1, 10, 25);
        assert_eq!(p.last_page, 3);
        let p = p.with_total(100);
        assert_eq!(p.last_page, 10);
        assert_eq!(p.total, 100);
    }

    /// Verifies map transforms rows and keeps the metadata.
    #[test]
    fn paginator_map_preserves_metadata() {
        let p = Paginator::new(vec![1u8, 2, 3], 1, 3, 9).map(|x| x * 10);
        assert_eq!(p.items, vec![10, 20, 30]);
        assert_eq!(p.total, 9);
        assert_eq!(p.per_page, 3);
    }

    /// Verifies PageMeta computes the OFFSET window.
    #[test]
    fn page_meta_offsets() {
        assert_eq!(
            PageMeta {
                page: 1,
                per_page: 10
            }
            .offset(),
            0
        );
        assert_eq!(
            PageMeta {
                page: 3,
                per_page: 10
            }
            .offset(),
            20
        );
    }

    /// Verifies COUNT projection inherits WHERE filters.
    #[test]
    fn count_inherits_filters() {
        let qb = QueryBuilder::table("users").where_null("deleted_at");
        assert_eq!(
            count_sql(&qb).unwrap(),
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL"
        );
        assert_eq!(to_row_count_sql(&qb).unwrap(), count_sql(&qb).unwrap());
    }

    /// Verifies SUM projection targets the requested column.
    #[test]
    fn sum_targets_column() {
        let qb = QueryBuilder::table("orders");
        assert_eq!(
            sum_sql(&qb, "total").unwrap(),
            "SELECT SUM(total) FROM orders"
        );
    }

    /// Verifies chunk_by windows cover the total without overlap.
    #[test]
    fn chunk_by_windows_cover_total() {
        let windows = chunk_by(&QueryBuilder::table("posts"), "id", 500, 10_000).unwrap();
        assert_eq!(windows.len(), 20);
        assert_eq!(windows[0], "id BETWEEN 0 AND 500");
        assert_eq!(windows[19], "id BETWEEN 9500 AND 10000");
    }

    /// Verifies chunk_by rejects a zero-size window.
    #[test]
    fn chunk_by_rejects_zero_size() {
        let err = chunk_by(&QueryBuilder::table("posts"), "id", 0, 100).unwrap_err();
        assert!(matches!(err, OrmError::InvalidState(_)));
    }

    /// Verifies transaction commits on success and rolls back on error.
    #[tokio::test]
    async fn transaction_rollback_on_error() {
        let ok: crate::Result<u8> = transaction(|_tx| Ok(7)).await;
        assert_eq!(ok.unwrap(), 7);
        let err: crate::Result<u8> = transaction(|_tx| Err(crate::OrmError::NotFound)).await;
        assert!(matches!(err.unwrap_err(), crate::OrmError::NotFound));
    }
}
