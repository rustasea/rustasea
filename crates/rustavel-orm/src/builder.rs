//! Fluent SQL query builder (in-memory SQL emission; sqlx execution wired in S03-T01).

use crate::error::{OrmError, Result, UpsertError};
use crate::scopes::ScopeRegistry;
use crate::types::{JsonFilter, Value};

#[cfg(feature = "vector")]
use crate::vector::VectorSimilarity;

/// Driver dialect selected via cargo features (Postgres default).
pub fn dialect() -> &'static str {
    #[cfg(feature = "postgres")]
    {
        "postgres"
    }
    #[cfg(all(not(feature = "postgres"), feature = "mysql"))]
    {
        "mysql"
    }
    #[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
    {
        "sqlite"
    }
    #[cfg(not(any(feature = "postgres", feature = "mysql", feature = "sqlite")))]
    {
        "unknown"
    }
}

/// Pessimistic lock clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    /// `FOR UPDATE` — blocks concurrent writers.
    ForUpdate,
    /// `FOR SHARE` — blocks writers, allows readers.
    Shared,
}

impl Lock {
    /// Emit the lock clause for the active dialect.
    pub fn to_sql(&self, dialect: &str) -> Result<String> {
        match dialect {
            "postgres" => match self {
                Lock::ForUpdate => Ok("FOR UPDATE".into()),
                Lock::Shared => Ok("FOR SHARE".into()),
            },
            "mysql" => match self {
                Lock::ForUpdate => Ok("FOR UPDATE".into()),
                Lock::Shared => Ok("LOCK IN SHARE MODE".into()),
            },
            "sqlite" => Err(OrmError::UnsupportedDriver(
                "sqlite has no row locks".into(),
            )),
            other => Err(OrmError::UnsupportedDriver(other.to_string())),
        }
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

impl OrderDirection {
    /// SQL keyword.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

/// A raw SQL fragment with bound values.
#[derive(Debug, Clone)]
pub struct Raw {
    /// SQL text with `$1`-style placeholders.
    pub sql: String,
    /// Bind values.
    pub bindings: Vec<Value>,
}

/// Transaction handle stub — real `BEGIN/COMMIT/ROLLBACK` wired in S03-T01.
#[derive(Debug)]
pub struct TransactionStub {
    /// Whether the transaction is still open.
    pub open: bool,
}

impl TransactionStub {
    /// Begin a transaction (stub).
    pub fn begin() -> Self {
        Self { open: true }
    }

    /// Commit the transaction (stub).
    pub fn commit(mut self) -> Result<()> {
        self.open = false;
        Ok(())
    }

    /// Roll back the transaction (stub).
    pub fn rollback(mut self) -> Result<()> {
        self.open = false;
        Ok(())
    }
}

/// A single WHERE condition.
#[derive(Clone)]
struct Condition {
    glue: &'static str, // "AND" | "OR"
    sql: String,
    /// Per-condition bind values (reserved for named-scope composition in S03-T03).
    #[allow(dead_code)]
    bindings: Vec<Value>,
}

/// A column ordering.
#[derive(Clone)]
struct OrderBy {
    column: String,
    direction: OrderDirection,
}

/// Fluent query builder — builds parameterized SQL without touching the DB.
#[derive(Default, Clone)]
pub struct QueryBuilder {
    table: String,
    columns: Vec<String>,
    conditions: Vec<Condition>,
    orders: Vec<OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
    lock: Option<Lock>,
    scope_active: Vec<String>,
    bindings: Vec<Value>,
}

impl QueryBuilder {
    /// The table this query targets.
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Start a query against `table`.
    pub fn table(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            ..Default::default()
        }
    }

    /// Select specific columns (default `*`).
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|c| (*c).to_string()).collect();
        self
    }

    /// Add an equality WHERE clause: `where("email", value)`.
    pub fn where_eq(mut self, column: &str, value: impl Into<Value>) -> Self {
        let value = value.into();
        self.bindings.push(value);
        let idx = self.bindings.len();
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("{column} = ${idx}"),
            bindings: Vec::new(),
        });
        self
    }

    /// Add an OR equality WHERE clause.
    pub fn or_where_eq(mut self, column: &str, value: impl Into<Value>) -> Self {
        let value = value.into();
        self.bindings.push(value);
        let idx = self.bindings.len();
        self.conditions.push(Condition {
            glue: "OR",
            sql: format!("{column} = ${idx}"),
            bindings: Vec::new(),
        });
        self
    }

    /// Add a `WHERE column IN (...)` clause.
    pub fn where_in(mut self, column: &str, values: Vec<Value>) -> Self {
        let placeholders: Vec<String> = values
            .iter()
            .map(|v| {
                self.bindings.push(v.clone());
                format!("${}", self.bindings.len())
            })
            .collect();
        if placeholders.is_empty() {
            self.conditions.push(Condition {
                glue: "AND",
                sql: "1 = 0".into(),
                bindings: Vec::new(),
            });
            return self;
        }
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("{column} IN ({})", placeholders.join(", ")),
            bindings: Vec::new(),
        });
        self
    }

    /// Add a `WHERE column IS NULL` clause (soft-delete guard uses this).
    pub fn where_null(mut self, column: &str) -> Self {
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("{column} IS NULL"),
            bindings: Vec::new(),
        });
        self
    }

    /// Add a JSON filter clause (`@>`, path equality, or key exists).
    ///
    /// Filters that compare against a value (`Contains`, `PathEquals`) bind that
    /// value; `KeyExists` needs no placeholder.
    pub fn where_json(mut self, column: &str, filter: JsonFilter) -> Result<Self> {
        let placeholder = filter.bind_value().map(|value| {
            self.bindings.push(value);
            format!("${}", self.bindings.len())
        });
        let fragment = filter.to_sql(column, dialect())?;
        let sql = match placeholder {
            Some(placeholder) => fragment.replace("{}", &placeholder),
            None => fragment,
        };
        self.conditions.push(Condition {
            glue: "AND",
            sql,
            bindings: Vec::new(),
        });
        Ok(self)
    }

    /// Add a `WHERE id = $n` clause (findOrFail / whereKey).
    pub fn where_key(self, id: uuid::Uuid) -> Self {
        self.where_eq("id", Value::Uuid(id))
    }

    /// Order results by a column.
    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.orders.push(OrderBy {
            column: column.to_string(),
            direction,
        });
        self
    }

    /// Limit returned rows.
    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Skip rows (OFFSET pagination).
    pub fn offset(mut self, n: u64) -> Self {
        self.offset = Some(n);
        self
    }

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

    /// Apply the soft-delete guard when the model uses `deleted_at`.
    pub fn with_soft_deletes(self, uses: bool) -> Self {
        if uses {
            self.where_null("deleted_at")
        } else {
            self
        }
    }

    /// Include soft-deleted rows (no `deleted_at IS NULL` guard).
    pub fn with_trashed(self) -> Self {
        self
    }

    /// Compute OFFSET pagination window (`page` is 1-based).
    pub fn for_page(self, page: u64, per_page: u64) -> Self {
        let offset = page.saturating_sub(1).saturating_mul(per_page);
        self.limit(per_page).offset(offset)
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

    /// Render the SELECT statement with `$n` placeholders (never interpolates values).
    pub fn to_sql(&self) -> Result<String> {
        let columns = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };
        let mut sql = format!("SELECT {columns} FROM {}", self.table);
        if let Some(clause) = self.where_clause() {
            sql.push_str(" WHERE ");
            sql.push_str(&clause);
        }
        if !self.orders.is_empty() {
            let parts: Vec<String> = self
                .orders
                .iter()
                .map(|o| format!("{} {}", o.column, o.direction.as_str()))
                .collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&parts.join(", "));
        }
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {offset}"));
        }
        if let Some(lock) = self.lock {
            sql.push(' ');
            sql.push_str(&lock.to_sql(dialect())?);
        }
        Ok(sql)
    }

    /// Render the statement with inlined literals — **display only, never executed**.
    pub fn to_raw_sql(&self) -> Result<String> {
        let sql = self.to_sql()?;
        let mut out = sql;
        for (i, value) in self.bindings.iter().enumerate() {
            out = out.replace(&format!("${}", i + 1), &value.to_literal());
        }
        Ok(out)
    }

    /// Bind values in positional order.
    pub fn bindings(&self) -> &[Value] {
        &self.bindings
    }

    /// Whether any rows would match (used by `first` semantics).
    pub fn has_limit(&self) -> bool {
        self.limit.is_some()
    }

    /// Render the WHERE clause, or `None` when no conditions exist.
    pub fn where_clause(&self) -> Option<String> {
        if self.conditions.is_empty() {
            return None;
        }
        let mut out = String::new();
        for (i, cond) in self.conditions.iter().enumerate() {
            if i > 0 {
                out.push(' ');
                out.push_str(cond.glue);
                out.push(' ');
            }
            out.push_str(&cond.sql);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies toSql emits parameterized SQL for a basic filtered query.
    #[test]
    fn builds_parameterized_sql() {
        let qb = QueryBuilder::table("users")
            .where_eq("email", "a@b.c")
            .where_null("deleted_at")
            .order_by("created_at", OrderDirection::Desc)
            .limit(10);
        assert_eq!(
            qb.to_sql().unwrap(),
            "SELECT * FROM users WHERE email = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 10"
        );
    }

    /// Verifies toRawSql inlines literals and toSql never does.
    #[test]
    fn raw_sql_inlines_literals() {
        let qb = QueryBuilder::table("users").where_eq("name", "O'Brien");
        assert!(qb.to_sql().unwrap().contains("$1"));
        assert!(qb.to_raw_sql().unwrap().contains("'O''Brien'"));
    }

    /// Verifies strict upsert rejects empty uniqueBy before any round-trip.
    #[test]
    fn upsert_rejects_empty_unique_by() {
        let err = QueryBuilder::assert_upsert(&[], 1).unwrap_err();
        assert!(matches!(err, OrmError::Upsert(UpsertError::EmptyUniqueBy)));
    }

    /// Verifies pagination window math.
    #[test]
    fn paginator_computes_last_page() {
        let p: crate::execution::Paginator<u8> =
            crate::execution::Paginator::new(vec![1, 2, 3], 2, 3, 10);
        assert_eq!(p.last_page, 4);
    }

    /// Verifies where_json binds the filter value, not Null.
    #[test]
    fn where_json_binds_filter_value() {
        let qb = QueryBuilder::table("users")
            .where_json(
                "settings",
                JsonFilter::PathEquals("theme".into(), "dark".into()),
            )
            .unwrap();
        assert_eq!(qb.bindings(), &[Value::Text("dark".into())]);
        let raw = qb.to_raw_sql().unwrap();
        assert!(raw.contains("settings ->> '{theme}' = 'dark'"), "{raw}");

        let contains = QueryBuilder::table("users")
            .where_json(
                "settings",
                JsonFilter::Contains(serde_json::json!({"a": 1})),
            )
            .unwrap();
        assert_eq!(contains.bindings().len(), 1);
        assert!(matches!(contains.bindings()[0], Value::Json(_)));
        assert!(contains.to_raw_sql().unwrap().contains("@> '{\"a\":1}'"));

        // KeyExists needs no placeholder.
        let key = QueryBuilder::table("users")
            .where_json("settings", JsonFilter::KeyExists("theme".into()))
            .unwrap();
        assert_eq!(key.bindings(), &[] as &[Value]);
        assert!(key.to_raw_sql().unwrap().contains("settings ? 'theme'"));
    }
}
