//! M2 SQL-shape helpers — model scope entry points, insert/upsert builders and
//! raw-fragment emission used by the model helpers.
//!
//! Everything here emits SQL text; execution is wired by the driver crate.

use crate::builder::{QueryBuilder, SqlFragment};
use crate::error::{OrmError, Result};
use crate::types::Value;

/// Builder state for `insert_or_ignore_returning`.
#[derive(Debug, Clone)]
pub struct InsertBuilder {
    /// Target table.
    table: String,
    /// Column names to insert.
    columns: Vec<String>,
    /// Row values (column-major).
    values: Vec<Value>,
}

impl InsertBuilder {
    /// Begin an insert against `table`.
    pub fn table(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Add one column/value pair to the insert.
    pub fn column(mut self, column: &str, value: impl Into<Value>) -> Self {
        self.columns.push(column.to_string());
        self.values.push(value.into());
        self
    }

    /// The table targeted by this insert.
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Column names in insertion order.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Bind values in column order.
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Emit `INSERT INTO t (cols) VALUES ($1..$n)`.
    pub fn to_sql(&self) -> Result<String> {
        if self.columns.is_empty() {
            return Err(OrmError::InvalidState(
                "insert builder has no columns".into(),
            ));
        }
        let cols = self.columns.join(", ");
        let placeholders: Vec<String> = (1..=self.columns.len()).map(|i| format!("${i}")).collect();
        Ok(format!(
            "INSERT INTO {} ({cols}) VALUES ({})",
            self.table,
            placeholders.join(", ")
        ))
    }

    /// Emit `… ON CONFLICT (pk) DO NOTHING RETURNING id` (Postgres insertOrIgnore).
    pub fn on_conflict_returning_sql(&self, conflict: &[&str]) -> Result<String> {
        let base = self.to_sql()?;
        if conflict.is_empty() {
            return Err(OrmError::InvalidState(
                "insert-or-ignore requires a conflict column".into(),
            ));
        }
        Ok(format!(
            "{base} ON CONFLICT ({}) DO NOTHING RETURNING id",
            conflict.join(", ")
        ))
    }

    /// Emit `INSERT IGNORE INTO …` (MySQL insertOrIgnore).
    pub fn ignore_sql(&self) -> Result<String> {
        if self.columns.is_empty() {
            return Err(OrmError::InvalidState(
                "insert builder has no columns".into(),
            ));
        }
        let cols = self.columns.join(", ");
        let placeholders: Vec<String> = (1..=self.columns.len()).map(|i| format!("${i}")).collect();
        Ok(format!(
            "INSERT IGNORE INTO {} ({cols}) VALUES ({})",
            self.table,
            placeholders.join(", ")
        ))
    }

    /// Emit the dialect-specific insert-or-ignore shape (`saveOrIgnore`).
    ///
    /// Postgres uses `ON CONFLICT (id) DO NOTHING RETURNING id`, MySQL uses
    /// `INSERT IGNORE`; other dialects fall back to a plain INSERT so the SQL
    /// stays valid on drivers without conflict guards.
    pub fn save_or_ignore_sql(&self, dialect: &str) -> Result<String> {
        match dialect {
            "postgres" => self.on_conflict_returning_sql(&["id"]),
            "mysql" => self.ignore_sql(),
            _ => self.to_sql(),
        }
    }
}

/// Upsert builder — `INSERT … ON CONFLICT (uniqueBy) DO UPDATE SET …`.
#[derive(Debug, Clone)]
pub struct UpsertBuilder {
    /// Target table.
    table: String,
    /// Columns to write on conflict.
    columns: Vec<String>,
    /// Unique-by columns (conflict target).
    unique_by: Vec<String>,
    /// Columns excluded from the update set (skipped on conflict).
    exclude: Vec<String>,
}

impl UpsertBuilder {
    /// Begin an upsert against `table`.
    pub fn table(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            unique_by: Vec::new(),
            exclude: Vec::new(),
        }
    }

    /// Declare a column written by the upsert.
    pub fn column(mut self, column: &str) -> Self {
        self.columns.push(column.to_string());
        self
    }

    /// Set the strict `uniqueBy` list (empty is rejected at compile time).
    pub fn unique_by(mut self, unique_by: &[&str]) -> Result<Self> {
        if unique_by.is_empty() {
            return Err(OrmError::Upsert(crate::error::UpsertError::EmptyUniqueBy));
        }
        self.unique_by = unique_by.iter().map(|c| (*c).to_string()).collect();
        Ok(self)
    }

    /// Columns to exclude from the conflict update (e.g. `created_at`).
    pub fn exclude(mut self, columns: &[&str]) -> Self {
        self.exclude = columns.iter().map(|c| (*c).to_string()).collect();
        self
    }

    /// Emit the full upsert statement (Postgres dialect).
    pub fn to_sql(&self) -> Result<String> {
        if self.columns.is_empty() {
            return Err(OrmError::InvalidState(
                "upsert builder has no columns".into(),
            ));
        }
        if self.unique_by.is_empty() {
            return Err(OrmError::Upsert(crate::error::UpsertError::EmptyUniqueBy));
        }
        let cols = self.columns.join(", ");
        let placeholders: Vec<String> = (1..=self.columns.len()).map(|i| format!("${i}")).collect();
        let update: Vec<String> = self
            .columns
            .iter()
            .filter(|c| !self.unique_by.iter().any(|u| &u == c))
            .filter(|c| !self.exclude.iter().any(|e| &e == c))
            .cloned()
            .map(|c| format!("{c} = EXCLUDED.{c}"))
            .collect();
        if update.is_empty() {
            return Err(OrmError::InvalidState(
                "upsert has no non-unique columns to update".into(),
            ));
        }
        Ok(format!(
            "INSERT INTO {} ({cols}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
            self.table,
            placeholders.join(", "),
            self.unique_by.join(", "),
            update.join(", ")
        ))
    }
}

/// Model scope entry points — the SQL shapes a model's `query()` helper emits.
///
/// Each free function builds on [`QueryBuilder`] fragments so `to_sql` output
/// is verifiable end-to-end.
pub struct ModelScopes;

impl ModelScopes {
    /// `INSERT INTO t (cols) VALUES (…) ON CONFLICT DO NOTHING` fragment.
    pub fn insert_sql(builder: &InsertBuilder, dialect: &str) -> Result<String> {
        let base = builder.to_sql()?;
        match dialect {
            "postgres" => builder.on_conflict_returning_sql(&["id"]),
            "mysql" => builder.ignore_sql(),
            _ => Ok(base),
        }
    }

    /// Refresh a row for update: `SELECT * FROM t WHERE id = ? FOR UPDATE`.
    pub fn refresh_sql(table: &str, id: uuid::Uuid) -> Result<String> {
        QueryBuilder::table(table)
            .where_eq("id", Value::Uuid(id))
            .for_update()
            .map(|qb| qb.to_sql().unwrap_or_default())
    }

    /// Update timestamps clause: `SET updated_at = now()`.
    pub fn touch_clause(columns: &[&str]) -> String {
        if columns.is_empty() {
            "updated_at = now()".to_string()
        } else {
            columns
                .iter()
                .map(|c| format!("{c} = now()"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// Raw SQL fragment constructor (raw_sql helper).
pub fn raw_sql(fragment: impl Into<String>) -> SqlFragment {
    SqlFragment {
        sql: fragment.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies insert emission is parameterized and ordered.
    #[test]
    fn insert_builder_emits_parameterized_sql() {
        let b = InsertBuilder::table("users")
            .column("name", "ada")
            .column("email", "ada@example.test");
        assert_eq!(
            b.to_sql().unwrap(),
            "INSERT INTO users (name, email) VALUES ($1, $2)"
        );
        assert_eq!(b.values().len(), 2);
    }

    /// Verifies the Postgres ON CONFLICT shape carries RETURNING id.
    #[test]
    fn insert_or_ignore_returning_shape() {
        let b = InsertBuilder::table("users").column("email", "ada@example.test");
        assert_eq!(
            b.on_conflict_returning_sql(&["id"]).unwrap(),
            "INSERT INTO users (email) VALUES ($1) ON CONFLICT (id) DO NOTHING RETURNING id"
        );
        assert!(b.on_conflict_returning_sql(&[]).is_err());
    }

    /// Verifies MySQL INSERT IGNORE shape.
    #[test]
    fn insert_ignore_shape() {
        let b = InsertBuilder::table("users").column("email", "ada@example.test");
        assert_eq!(
            b.ignore_sql().unwrap(),
            "INSERT IGNORE INTO users (email) VALUES ($1)"
        );
    }

    /// Verifies upsert emits EXCLUDED updates and rejects empty uniqueBy.
    #[test]
    fn upsert_emits_excluded_updates() {
        let u = UpsertBuilder::table("users")
            .column("email")
            .column("name")
            .column("created_at")
            .unique_by(&["email"])
            .unwrap()
            .exclude(&["created_at"]);
        assert_eq!(
            u.to_sql().unwrap(),
            "INSERT INTO users (email, name, created_at) VALUES ($1, $2, $3) \
             ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name"
        );
        assert!(UpsertBuilder::table("users")
            .column("email")
            .unique_by(&[])
            .is_err());
    }

    /// Verifies ModelScopes delegates to the dialect-specific insert shape.
    #[test]
    fn model_scopes_insert_shapes() {
        let b = InsertBuilder::table("users").column("email", "a@b.c");
        assert!(ModelScopes::insert_sql(&b, "postgres")
            .unwrap()
            .contains("ON CONFLICT (id) DO NOTHING RETURNING id"));
        assert!(ModelScopes::insert_sql(&b, "mysql")
            .unwrap()
            .starts_with("INSERT IGNORE INTO"));
    }

    /// Verifies saveOrIgnore switches shape per dialect.
    #[test]
    fn save_or_ignore_is_dialect_aware() {
        let b = InsertBuilder::table("users").column("email", "a@b.c");
        let postgres = b.save_or_ignore_sql("postgres").unwrap();
        assert!(postgres.contains("ON CONFLICT (id) DO NOTHING RETURNING id"));
        assert!(b
            .save_or_ignore_sql("mysql")
            .unwrap()
            .starts_with("INSERT IGNORE INTO"));
        assert_eq!(b.save_or_ignore_sql("sqlite").unwrap(), b.to_sql().unwrap());
    }
}
