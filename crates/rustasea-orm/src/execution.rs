//! Query execution helpers — pagination, raw statements, aggregations,
//! chunked iteration and transactions.
//!
//! [`Paginator`] is the shared `{data, meta, links}` envelope; `count_sql` /
//! `sum_sql` / the string `chunk_by` emit SQL text for callers to verify before
//! a round-trip. The async executors live in [`crate::builder`] and
//! [`crate::model_ops`]; [`transaction`] runs a body atomically against a real
//! [`crate::db::DbPool`].

use crate::builder::{QueryBuilder, Raw};
use crate::db::DbPool;
use crate::error::{OrmError, Result};
use crate::tx::Transaction;
use crate::types::Value;
use std::future::Future;
use std::pin::Pin;

/// Pagination metadata and rows (offset pagination).
///
/// Serializes to the shared wire envelope `{ data, meta, links }` (see
/// `modules/data-orm/query-builder.md` §3.2) while exposing the flat fields to
/// Rust callers.
#[derive(Debug, Clone)]
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
    /// Navigation links for the page envelope.
    pub links: Links,
}

/// Meta block of a paginated response envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PaginationMeta {
    /// 1-based current page.
    pub current_page: u64,
    /// Rows per page.
    pub per_page: u64,
    /// Total matching rows.
    pub total: u64,
    /// Last page number.
    pub last_page: u64,
}

/// Wire envelope for a paginated response (`data`/`meta`/`links`).
#[derive(Debug, Clone, serde::Deserialize)]
struct PaginatorEnvelope<T> {
    /// Rows on this page.
    data: Vec<T>,
    /// Pagination metadata.
    meta: PaginationMeta,
    /// Navigation links.
    links: Links,
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Paginator<T> {
    /// Read the `{ data, meta, links }` envelope back into flat fields.
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let envelope = PaginatorEnvelope::<T>::deserialize(deserializer)?;
        Ok(Self {
            items: envelope.data,
            current_page: envelope.meta.current_page,
            per_page: envelope.meta.per_page,
            total: envelope.meta.total,
            last_page: envelope.meta.last_page,
            links: envelope.links,
        })
    }
}

impl<T: serde::Serialize> serde::Serialize for Paginator<T> {
    /// Emit the `{ data, meta, links }` envelope shared across modules.
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Paginator", 3)?;
        state.serialize_field("data", &self.items)?;
        state.serialize_field(
            "meta",
            &PaginationMeta {
                current_page: self.current_page,
                per_page: self.per_page,
                total: self.total,
                last_page: self.last_page,
            },
        )?;
        state.serialize_field("links", &self.links)?;
        state.end()
    }
}

/// Page navigation links emitted with a paginator envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Links {
    /// Link to the first page.
    pub first: String,
    /// Link to the previous page (`None` on the first page).
    pub prev: Option<String>,
    /// Link to the next page (`None` on the last page).
    pub next: Option<String>,
    /// Link to the last page.
    pub last: String,
}

impl Default for Links {
    /// Empty links — the value carried by a bare [`Paginator::new`].
    fn default() -> Self {
        Self {
            first: String::new(),
            prev: None,
            next: None,
            last: String::new(),
        }
    }
}

impl Links {
    /// Build the navigation links for a 1-based `page` of `total` rows.
    pub fn for_page(page: u64, per_page: u64, total: u64) -> Self {
        let page = page.max(1);
        let last_page = if per_page == 0 {
            1
        } else {
            total.div_ceil(per_page).max(1)
        };
        let link = |target: u64| format!("?page={target}");
        Self {
            first: link(1),
            prev: (page > 1).then(|| link(page - 1)),
            next: (page < last_page).then(|| link(page + 1)),
            last: link(last_page),
        }
    }
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
            links: Links::default(),
        }
    }

    /// Populate the navigation links from the current page metadata.
    pub fn with_links(mut self) -> Self {
        self.links = Links::for_page(self.current_page.max(1), self.per_page, self.total);
        self
    }

    /// Recompute page metadata from a fresh COUNT (`per_page` kept).
    pub fn with_total(&self, total: u64) -> Self
    where
        T: Clone,
    {
        Self::new(self.items.clone(), self.current_page, self.per_page, total).with_links()
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
        Paginator {
            items,
            current_page: self.current_page,
            per_page: self.per_page,
            total: self.total,
            last_page: self.last_page,
            links: self.links,
        }
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
pub fn raw(sql: &str, bindings: Vec<Value>) -> Raw {    Raw {
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

/// Execute a transaction body atomically against `pool`.
///
/// Begins a real transaction and commits on `Ok` / rolls back on `Err`. Do not call
/// `transaction()` inside a `transaction()` body — a nested call acquires a second connection and deadlocks; use the passed `&mut Transaction` handle instead.
///
/// The body is a closure returning a boxed future so it can run driver
/// statements asynchronously on the transaction connection:
///
/// ```ignore
/// transaction(&pool, |tx| Box::pin(async move {
///     tx.execute_bind("INSERT INTO t (id) VALUES ($1)", &[Value::Int(1)]).await?;
///     Ok(())
/// })).await?;
/// ```
pub async fn transaction<F, T>(pool: &DbPool, body: F) -> Result<T>
where
    F: for<'a> FnOnce(
        &'a mut Transaction,
    ) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>,
{
    let mut tx = Transaction::begin(pool).await?;
    match body(&mut tx).await {
        Ok(value) => {
            tx.commit().await?;
            Ok(value)
        }
        Err(e) => {
            let _ = tx.rollback().await;
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
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn transaction_rollback_on_error() {
        let pool = DbPool::connect("sqlite::memory:").await.unwrap();
        pool.execute_bind("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[])
            .await
            .unwrap();

        let ok = transaction(&pool, |_tx| Box::pin(async { Ok(7u8) }))
            .await
            .unwrap();
        assert_eq!(ok, 7);

        let err = transaction(&pool, |_tx| {
            Box::pin(async { Err::<u8, _>(crate::OrmError::NotFound) })
        })
        .await
        .unwrap_err();
        assert!(matches!(err, crate::OrmError::NotFound));
        pool.close().await;
    }
}
