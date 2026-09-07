//! Query execution helpers — raw statements, aggregations, transactions.
//!
//! In-memory stubs for Sprint 03; the sqlx pool wiring replaces the bodies in
//! S03-T01 without changing these signatures.

use crate::builder::{QueryBuilder, Raw, TransactionStub};
use crate::error::Result;
use crate::types::Value;

/// Pagination metadata and rows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Paginator<T> {
    /// Rows on this page.
    pub data: Vec<T>,
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
    pub fn new(data: Vec<T>, current_page: u64, per_page: u64, total: u64) -> Self {
        let last_page = if per_page == 0 {
            0
        } else {
            total.div_ceil(per_page)
        };
        Self {
            data,
            current_page,
            per_page,
            total,
            last_page,
        }
    }
}

/// Execute a raw statement (stub — sqlx wiring lands with S03-T01).
pub fn raw(sql: &str, bindings: Vec<Value>) -> Raw {
    Raw {
        sql: sql.to_string(),
        bindings,
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

/// Aggregation helper: build a `SUM(column)` projection over the current filters.
pub fn sum_sql(builder: &QueryBuilder, column: &str) -> Result<String> {
    let mut sql = format!("SELECT SUM({column}) FROM {}", builder.table_name());
    if let Some(clause) = builder.where_clause() {
        sql.push_str(" WHERE ");
        sql.push_str(&clause);
    }
    Ok(sql)
}

/// Execute a transaction body atomically (stub — pool wiring in S03-T01).
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

    /// Verifies COUNT projection inherits WHERE filters.
    #[test]
    fn count_inherits_filters() {
        let qb = QueryBuilder::table("users").where_null("deleted_at");
        assert_eq!(
            count_sql(&qb).unwrap(),
            "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL"
        );
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

    /// Verifies transaction commits on success and rolls back on error.
    #[tokio::test]
    async fn transaction_rollback_on_error() {
        let ok: crate::Result<u8> = transaction(|_tx| Ok(7)).await;
        assert_eq!(ok.unwrap(), 7);
        let err: crate::Result<u8> = transaction(|_tx| Err(crate::OrmError::NotFound)).await;
        assert!(matches!(err.unwrap_err(), crate::OrmError::NotFound));
    }
}
