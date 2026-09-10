//! Query-builder extensions: locks, scopes, soft deletes, pagination windows,
//! vector similarity, and DELETE/STRAIGHT_JOIN emission.

use super::{dialect, Condition, JoinClause, Lock, OrderDirection, QueryBuilder};
#[cfg(feature = "vector")]
use super::OrderBy;
use crate::error::{OrmError, Result, UpsertError};
use crate::scopes::ScopeRegistry;
use crate::types::Value;

#[cfg(feature = "vector")]
use crate::vector::VectorSimilarity;

impl QueryBuilder {
    /// Apply a pessimistic lock clause.
    pub fn lock(mut self, lock: Lock) -> Result<Self> {
        lock.to_sql(dialect())?; // validate early for sqlite
        self.lock = Some(lock);
        Ok(self)
    }

    /// Convenience: `FOR UPDATE`.
    pub fn for_update(self) -> Result<Self> {
        self.lock(Lock::ForUpdate)
    }

    /// Convenience: `FOR SHARE`.
    pub fn shared_lock(self) -> Result<Self> {
        self.lock(Lock::Shared)
    }

    /// Apply a named scope via a registry (composable scopes, S03-T03).
    pub fn with_scope(mut self, registry: &ScopeRegistry, name: &str) -> Self {
        let table = self.table.clone();
        registry.apply(&mut self, &table, name);
        self.scope_active.push(name.to_string());
        self
    }

    /// Apply a single scope closure to this builder.
    pub fn apply_scope<F>(mut self, mut scope: F) -> Self
    where
        F: FnMut(&mut QueryBuilder) + 'static,
    {
        scope(&mut self);
        self
    }

    /// Whether a named scope has already been applied (dedupe guard).
    pub fn has_scope(&self, name: &str) -> bool {
        self.scope_active.iter().any(|n| n == name)
    }

    /// Apply the soft-delete guard when the model uses `deleted_at`.
    ///
    /// Applies `deleted_at IS NULL` once; re-application is idempotent. With
    /// the model untouched this is a no-op — `Model::query()` gates queries.
    pub fn with_soft_deletes(mut self, uses: bool) -> Self {
        if uses && self.soft_delete_guard.is_none() {
            self.soft_delete_guard = Some(false);
            self.conditions.push(Condition {
                glue: "AND",
                sql: "deleted_at IS NULL".into(),
                bindings: Vec::new(),
            });
        }
        self
    }

    /// Include soft-deleted rows (no `deleted_at IS NULL` guard).
    ///
    /// Overrides any previously applied soft-delete guard.
    pub fn with_trashed(mut self) -> Self {
        if self.soft_delete_guard == Some(false) {
            self.conditions.retain(|c| c.sql != "deleted_at IS NULL");
        }
        self.soft_delete_guard = Some(true);
        self
    }

    /// Only soft-deleted rows (`deleted_at IS NOT NULL`).
    pub fn only_trashed(mut self) -> Self {
        if self.soft_delete_guard == Some(false) {
            self.conditions.retain(|c| c.sql != "deleted_at IS NULL");
        }
        self.soft_delete_guard = Some(true);
        self.conditions.push(Condition {
            glue: "AND",
            sql: "deleted_at IS NOT NULL".into(),
            bindings: Vec::new(),
        });
        self
    }

    /// Compute OFFSET pagination window (`page` is 1-based).
    pub fn for_page(self, page: u64, per_page: u64) -> Self {
        let offset = page.saturating_sub(1).saturating_mul(per_page);
        self.limit(per_page).offset(offset)
    }

    /// OFFSET-paginate alias of [`QueryBuilder::for_page`] — `forPage(page, perPage)`.
    ///
    /// Emits `LIMIT perPage OFFSET (page-1)*perPage`; the async `paginate`
    /// executor owns the COUNT + windowed SELECT and returns the full
    /// [`crate::execution::Paginator`] envelope.
    pub fn page_window(self, page: u64, per_page: u64) -> Self {
        self.for_page(page, per_page)
    }

    /// Cursor-paginate alias — `cursor(pageCursor, perPage)`.
    ///
    /// Windows the page by `per_page` rows (LIMIT) with the cursor filter
    /// applied; ORDER BY must be set beforehand so `cursor` knows the
    /// direction. Use [`QueryBuilder::cursor`] when a column + cursor value
    /// pair drives the window.
    pub fn cursor_page(self, page_cursor: Value, per_page: u64) -> Self {
        self.cursor("id", page_cursor).limit(per_page)
    }

    /// Cursor pagination window: rows strictly after `cursor` on the order column.
    pub fn cursor(mut self, column: &str, cursor: Value) -> Self {
        self.bindings.push(cursor);
        let idx = self.bindings.len();
        let dir = self
            .orders
            .iter()
            .find(|o| o.column == column)
            .map(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);
        let op = match dir {
            OrderDirection::Asc => ">",
            OrderDirection::Desc => "<",
        };
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("{column} {op} ${idx}"),
            bindings: Vec::new(),
        });
        self
    }

    /// Validate an `upsert` call — strict `uniqueBy` enforcement (FS-M2-04).
    pub fn assert_upsert(unique_by: &[&str], rows: usize) -> Result<()> {
        if unique_by.is_empty() {
            return Err(UpsertError::EmptyUniqueBy.into());
        }
        if rows > 1000 {
            return Err(UpsertError::BatchTooLarge(rows).into());
        }
        Ok(())
    }

    /// Add a vector similarity clause (`ORDER BY col <=> $n LIMIT k`) — requires `vector` feature.
    #[cfg(feature = "vector")]
    pub fn where_vector_similar_to(
        mut self,
        column: &str,
        embedding: &[f32],
        limit: u32,
    ) -> Result<Self> {
        let sim = VectorSimilarity::new(embedding.to_vec())?;
        self.bindings.push(Value::Vector(sim.embedding.clone()));
        let idx = self.bindings.len();
        self.orders = Vec::new(); // similarity ordering replaces explicit orders
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("{column} IS NOT NULL AND {column} <=> ${idx}"),
            bindings: Vec::new(),
        });
        self.orders.push(OrderBy {
            column: format!("{column} <=> ${idx}"),
            direction: OrderDirection::Asc,
        });
        self.limit = Some(limit as u64);
        Ok(self)
    }
}

/// DML emission helpers — all methods return SQL text only.
impl QueryBuilder {
    /// Primary-key name assumed by `insert_or_ignore`/`refresh_for_update`.
    pub fn primary_key() -> &'static str {
        "id"
    }

    /// Set the select limit (shared by LIMIT/OFFSET pagination).
    pub fn set_limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set the select offset (shared by LIMIT/OFFSET pagination).
    pub fn set_offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

    /// Emit a DELETE with the current filters as a fragment (`delete_sql`).
    ///
    /// Driver-safe emission; MySQL DELETE-JOIN (`straight_join`) handled by
    /// [`QueryBuilder::delete_join_sql`] callers.
    pub fn to_delete_clause(&self) -> Result<String> {
        let mut sql = String::from("DELETE FROM ");
        sql.push_str(&self.table);
        if let Some(clause) = self.where_clause() {
            sql.push_str(" WHERE ");
            sql.push_str(&clause);
        }
        Ok(sql)
    }

    /// Emit a DELETE joining a second table (MySQL `DELETE JOIN`).
    ///
    /// `DELETE <target> FROM <target> JOIN <join_table> ON <on> WHERE …`.
    pub fn delete_join_sql(&self, join_table: &str, on: &str) -> Result<String> {
        if dialect() != "mysql" {
            return Err(OrmError::UnsupportedDriver(format!(
                "{} does not support DELETE JOIN",
                dialect()
            )));
        }
        let mut sql = format!(
            "DELETE {} FROM {} JOIN {join_table} ON {on}",
            self.table, self.table
        );
        if let Some(clause) = self.where_clause() {
            sql.push_str(" WHERE ");
            sql.push_str(&clause);
        }
        Ok(sql)
    }

    /// Add a `STRAIGHT_JOIN` hint (MySQL `straight_join()`).
    pub fn straight_join(mut self, join_table: &str, on: &str) -> Result<Self> {
        if dialect() != "mysql" {
            return Err(OrmError::UnsupportedDriver(format!(
                "{} does not support STRAIGHT_JOIN",
                dialect()
            )));
        }
        self.joins.push(JoinClause {
            sql: format!("STRAIGHT_JOIN {join_table} ON {on}"),
        });
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scopes::ScopeRegistry;

    /// Verifies with_soft_deletes gates on deleted_at and with_trashed removes it.
    #[test]
    fn soft_delete_guard_toggles() {
        let active = QueryBuilder::table("users").with_soft_deletes(true);
        assert_eq!(
            active.to_sql().unwrap(),
            "SELECT * FROM users WHERE deleted_at IS NULL"
        );

        let trashed = QueryBuilder::table("users")
            .with_soft_deletes(true)
            .with_trashed();
        assert_eq!(trashed.to_sql().unwrap(), "SELECT * FROM users");

        let only = QueryBuilder::table("users").only_trashed();
        assert_eq!(
            only.to_sql().unwrap(),
            "SELECT * FROM users WHERE deleted_at IS NOT NULL"
        );
    }

    /// Verifies no-op when the model does not use soft deletes.
    #[test]
    fn soft_delete_noop_when_unused() {
        let qb = QueryBuilder::table("users").with_soft_deletes(false);
        assert_eq!(qb.to_sql().unwrap(), "SELECT * FROM users");
    }

    /// Verifies for_page computes the OFFSET window.
    #[test]
    fn for_page_window() {
        let qb = QueryBuilder::table("users").for_page(3, 25);
        assert_eq!(qb.limit, Some(25));
        assert_eq!(qb.offset, Some(50));
    }

    /// Verifies page_window aliases for_page and cursor_page binds a LIMIT.
    #[test]
    fn paginate_aliases_for_page() {
        let qb = QueryBuilder::table("users").page_window(2, 15);
        assert_eq!(qb.limit, Some(15));
        assert_eq!(qb.offset, Some(15));

        let qb = QueryBuilder::table("users")
            .order_by("id", OrderDirection::Asc)
            .cursor_page(Value::Int(40), 15);
        assert_eq!(qb.limit, Some(15));
        assert!(qb.to_sql().unwrap().contains("id > $1"));
    }

    /// Verifies apply_scope mutates the builder through a closure.
    #[test]
    fn apply_scope_closure() {
        let qb = QueryBuilder::table("users").apply_scope(|q| {
            *q = std::mem::take(q).where_eq("active", true);
        });
        assert!(qb.to_sql().unwrap().contains("active = $1"));
    }

    /// Verifies named-scope dedupe bookkeeping.
    #[test]
    fn named_scope_tracks_active() {
        let mut reg = ScopeRegistry::default();
        reg.register("users", "active", |q| {
            *q = std::mem::take(q).where_eq("active", true);
        });
        let qb = QueryBuilder::table("users").with_scope(&reg, "active");
        assert!(qb.has_scope("active"));
        assert!(qb.to_sql().unwrap().contains("active = $1"));
    }

    /// Verifies cursor pagination direction derives from ORDER BY.
    #[test]
    fn cursor_window_uses_order_direction() {
        let asc = QueryBuilder::table("users")
            .order_by("id", OrderDirection::Asc)
            .cursor("id", Value::Int(10));
        assert!(asc.to_sql().unwrap().contains("id > $1"));

        let desc = QueryBuilder::table("users")
            .order_by("id", OrderDirection::Desc)
            .cursor("id", Value::Int(10));
        assert!(desc.to_sql().unwrap().contains("id < $1"));
    }

    /// Verifies where_binary hex-encodes bytes (postgres).
    #[test]
    fn where_binary_hex_encode() {
        let qb = QueryBuilder::table("files").where_binary("hash", &[0xde, 0xad]);
        assert!(qb.to_sql().unwrap().contains("encode(hash, 'hex') = $1"));
        assert_eq!(qb.bindings(), &[Value::Text("dead".into())]);
    }

    /// Verifies lock emission is dialect-aware.
    #[test]
    fn lock_clause_emission() {
        if dialect() == "postgres" {
            let qb = QueryBuilder::table("users").for_update().unwrap();
            assert!(qb.to_sql().unwrap().ends_with("FOR UPDATE"));
            let qb = QueryBuilder::table("users").shared_lock().unwrap();
            assert!(qb.to_sql().unwrap().ends_with("FOR SHARE"));
        }
    }

    /// Verifies vector similarity ordering (requires vector feature).
    #[cfg(feature = "vector")]
    #[test]
    fn vector_similar_to_shapes_query() {
        let qb = QueryBuilder::table("documents")
            .where_vector_similar_to("embedding", &[0.1, 0.2, 0.3], 10)
            .unwrap();
        let sql = qb.to_sql().unwrap();
        assert!(sql.contains("embedding IS NOT NULL AND embedding <=> $1"));
        assert!(sql.contains("ORDER BY embedding <=> $1 ASC"));
        assert!(sql.contains("LIMIT 10"));
    }
}
