//! Fluent SQL query builder (in-memory SQL emission; sqlx execution wired in S03-T01).

use crate::error::{OrmError, Result};
use crate::types::{JsonFilter, Value};

pub use crate::clause::{Lock, OrderDirection, Raw, SqlFragment};

mod exec;
mod ext;

pub(crate) use exec::json_to_model;
pub use exec::Executor;

mod eager;

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

/// A `JOIN` clause carried by the builder (`straight_join` hint).
#[derive(Debug, Clone)]
struct JoinClause {
    sql: String,
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
    joins: Vec<JoinClause>,
    orders: Vec<OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
    lock: Option<Lock>,
    /// Runtime lock dialect (`lock_with_dialect`); `None` = compile-time [`dialect`].
    lock_dialect: Option<String>,
    scope_active: Vec<String>,
    bindings: Vec<Value>,
    /// Soft-delete guard state: `None` = not applied, `true` = include trashed,
    /// `false` = active rows only (`deleted_at IS NULL`).
    soft_delete_guard: Option<bool>,
    /// Relation names requested for eager loading via `with(&[...])`.
    eager: Vec<String>,
    /// Declared relation metadata used to resolve `eager` names.
    eager_declared: Vec<crate::model::Relation>,
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

    /// Add an OR equality clause on the primary key (`orWhereKey`).
    pub fn or_where_key(self, column: &str, value: impl Into<Value>) -> Self {
        self.or_where_eq(column, value)
    }

    /// Add a `WHERE column = ?` on a raw byte string (`whereBinary`).
    pub fn where_binary(mut self, column: &str, value: &[u8]) -> Self {
        self.bindings.push(Value::Text(
            value.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        ));
        let idx = self.bindings.len();
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("encode({column}, 'hex') = ${idx}"),
            bindings: Vec::new(),
        });
        self
    }

    /// Add an arbitrary operator clause: `where("age", ">", 18)`.
    pub fn where_op(
        mut self,
        column: &str,
        operator: &str,
        value: impl Into<Value>,
    ) -> Result<Self> {
        let allowed = ["=", "!=", "<>", ">", ">=", "<", "<=", "LIKE", "NOT LIKE"];
        if !allowed.contains(&operator) {
            return Err(OrmError::InvalidState(format!(
                "unsupported operator `{operator}`"
            )));
        }
        let value = value.into();
        self.bindings.push(value);
        let idx = self.bindings.len();
        self.conditions.push(Condition {
            glue: "AND",
            sql: format!("{column} {operator} ${idx}"),
            bindings: Vec::new(),
        });
        Ok(self)
    }

    /// Splice a raw SQL fragment into the WHERE clause (unparameterized).
    pub fn where_raw(mut self, fragment: impl Into<SqlFragment>) -> Self {
        self.conditions.push(Condition {
            glue: "AND",
            sql: fragment.into().sql,
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

    /// Bind values in positional order.
    pub fn bindings(&self) -> &[Value] {
        &self.bindings
    }

    /// Whether any rows would match (used by `first` semantics).
    pub fn has_limit(&self) -> bool {
        self.limit.is_some()
    }

    /// Whether the query currently filters on the primary key column.
    pub fn has_where_key(&self) -> bool {
        self.conditions
            .iter()
            .any(|c| c.sql.starts_with("id = $") || c.sql.starts_with("id IN ("))
    }

    /// Whether the builder has conditions applied.
    pub fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }

    /// The active ORDER BY clause (used by `delete_sql` for MySQL JOIN deletes).
    pub fn order_clause(&self) -> Option<String> {
        if self.orders.is_empty() {
            return None;
        }
        let parts: Vec<String> = self
            .orders
            .iter()
            .map(|o| format!("{} {}", o.column, o.direction.as_str()))
            .collect();
        Some(parts.join(", "))
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

    /// Render the SELECT statement with `$n` placeholders (never interpolates values).
    pub fn to_sql(&self) -> Result<String> {
        let columns = if self.columns.is_empty() {
            "*".to_string()
        } else {
            self.columns.join(", ")
        };
        let mut sql = format!("SELECT {columns} FROM {}", self.table);
        for join in &self.joins {
            sql.push(' ');
            sql.push_str(&join.sql);
        }
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
            let lock_dialect = self.lock_dialect.as_deref().unwrap_or(dialect());
            sql.push_str(&lock.to_sql(lock_dialect)?);
        }
        Ok(sql)
    }

    /// Render the raw-SQL form: the table fragment with filters applied.
    ///
    /// Display-only diagnostic for `raw_sql` helpers; the SELECT projection
    /// never matters for count/delete/update emission.
    pub fn to_raw_sql_clause(&self) -> Result<String> {
        let mut sql = String::from("FROM ");
        sql.push_str(&self.table);
        for join in &self.joins {
            sql.push(' ');
            sql.push_str(&join.sql);
        }
        if let Some(clause) = self.where_clause() {
            sql.push_str(" WHERE ");
            sql.push_str(&clause);
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
        assert!(matches!(
            err,
            OrmError::Upsert(crate::error::UpsertError::EmptyUniqueBy)
        ));
    }

    /// Verifies pagination window math.
    #[test]
    fn paginator_computes_last_page() {
        let p: crate::execution::Paginator<u8> =
            crate::execution::Paginator::new(vec![1, 2, 3], 2, 3, 10);
        assert_eq!(p.last_page, 4);
    }

    /// Verifies where_json binds the filter value and emits dialect-shaped SQL.
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
        match dialect() {
            "postgres" => assert!(raw.contains("settings ->> '{theme}' = 'dark'"), "{raw}"),
            "mysql" => assert!(
                raw.contains("JSON_UNQUOTE(JSON_EXTRACT(settings, '$.theme')) = 'dark'"),
                "{raw}"
            ),
            "sqlite" => assert!(
                raw.contains("json_extract(settings, '$.theme') = 'dark'"),
                "{raw}"
            ),
            other => panic!("unexpected dialect {other}"),
        }

        // `Contains` is Postgres/MySQL-only; SQLite has no native operator.
        let contains = QueryBuilder::table("users").where_json(
            "settings",
            JsonFilter::Contains(serde_json::json!({"a": 1})),
        );
        match dialect() {
            "postgres" => {
                let contains = contains.unwrap();
                assert_eq!(contains.bindings().len(), 1);
                assert!(matches!(contains.bindings()[0], Value::Json(_)));
                assert!(contains.to_raw_sql().unwrap().contains("@> '{\"a\":1}'"));
            }
            "mysql" => {
                let contains = contains.unwrap();
                assert_eq!(contains.bindings().len(), 1);
                assert!(contains
                    .to_raw_sql()
                    .unwrap()
                    .contains("JSON_CONTAINS(settings"));
            }
            "sqlite" => {
                assert!(matches!(contains, Err(OrmError::UnsupportedDriver(_))));
            }
            other => panic!("unexpected dialect {other}"),
        }

        // KeyExists needs no placeholder.
        let key = QueryBuilder::table("users")
            .where_json("settings", JsonFilter::KeyExists("theme".into()))
            .unwrap();
        assert_eq!(key.bindings(), &[] as &[Value]);
        let key_raw = key.to_raw_sql().unwrap();
        match dialect() {
            "postgres" => assert!(key_raw.contains("settings ? 'theme'"), "{key_raw}"),
            "mysql" => assert!(
                key_raw.contains("JSON_CONTAINS_PATH(settings, 'one', '$.theme')"),
                "{key_raw}"
            ),
            "sqlite" => assert!(
                key_raw.contains("json_type(settings, '$.theme') IS NOT NULL"),
                "{key_raw}"
            ),
            other => panic!("unexpected dialect {other}"),
        }
    }
}
