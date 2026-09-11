//! Per-driver value binding and row decoding.
//!
//! Isolates the `sqlx` `bind`/`try_get` glue from [`super::pool::DbPool`] so the
//! pool facade stays driver-neutral. Every helper is compiled only for its
//! matching cargo feature.

use crate::error::{OrmError, Result};
use crate::types::Value;

use super::row::DbRow;

/// PostgreSQL query type alias.
#[cfg(feature = "postgres")]
pub(super) type PgQuery<'q> = sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>;

/// MySQL query type alias.
#[cfg(feature = "mysql")]
pub(super) type MySqlQuery<'q> = sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>;

/// SQLite query type alias.
#[cfg(feature = "sqlite")]
pub(super) type SqliteQuery<'q> =
    sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>;

/// Bind one [`Value`], encoding UUIDs in the driver's native form.
///
/// Returns [`OrmError::InvalidValue`] for [`Value::Unsupported`], which is a
/// decode-only marker and must never be written back to the database.
#[cfg(feature = "postgres")]
macro_rules! bind_native_uuid {
    ($query:expr, $value:expr) => {
        match $value {
            Value::Null => Ok($query.bind(Option::<String>::None)),
            Value::Bool(value) => Ok($query.bind(*value)),
            Value::Int(value) => Ok($query.bind(*value)),
            Value::Float(value) => Ok($query.bind(*value)),
            Value::Text(value) => Ok($query.bind(value.clone())),
            Value::Uuid(value) => Ok($query.bind(*value)),
            Value::Json(value) => Ok($query.bind(sqlx::types::Json(value.clone()))),
            #[cfg(feature = "vector")]
            Value::Vector(value) => Ok($query.bind(vector_literal(value))),
            Value::Unsupported(type_name) => Err(OrmError::InvalidValue(format!(
                "cannot bind an unsupported decode marker (driver type `{type_name}`)"
            ))),
        }
    };
}

/// Bind one [`Value`], encoding UUIDs as text for CHAR(36)/TEXT columns.
///
/// Returns [`OrmError::InvalidValue`] for [`Value::Unsupported`], which is a
/// decode-only marker and must never be written back to the database.
#[cfg(any(feature = "mysql", feature = "sqlite"))]
macro_rules! bind_text_uuid {
    ($query:expr, $value:expr) => {
        match $value {
            Value::Null => Ok($query.bind(Option::<String>::None)),
            Value::Bool(value) => Ok($query.bind(*value)),
            Value::Int(value) => Ok($query.bind(*value)),
            Value::Float(value) => Ok($query.bind(*value)),
            Value::Text(value) => Ok($query.bind(value.clone())),
            Value::Uuid(value) => Ok($query.bind(value.to_string())),
            Value::Json(value) => Ok($query.bind(sqlx::types::Json(value.clone()))),
            #[cfg(feature = "vector")]
            Value::Vector(value) => Ok($query.bind(vector_literal(value))),
            Value::Unsupported(type_name) => Err(OrmError::InvalidValue(format!(
                "cannot bind an unsupported decode marker (driver type `{type_name}`)"
            ))),
        }
    };
}

/// Bind all values onto a PostgreSQL query in order.
#[cfg(feature = "postgres")]
pub(super) fn bind_pg<'q>(mut query: PgQuery<'q>, bindings: &[Value]) -> Result<PgQuery<'q>> {
    for value in bindings {
        query = bind_native_uuid!(query, value)?;
    }
    Ok(query)
}

/// Bind all values onto a MySQL query in order.
#[cfg(feature = "mysql")]
pub(super) fn bind_mysql<'q>(
    mut query: MySqlQuery<'q>,
    bindings: &[Value],
) -> Result<MySqlQuery<'q>> {
    for value in bindings {
        query = bind_text_uuid!(query, value)?;
    }
    Ok(query)
}

/// Bind all values onto a SQLite query in order.
#[cfg(feature = "sqlite")]
pub(super) fn bind_sqlite<'q>(
    mut query: SqliteQuery<'q>,
    bindings: &[Value],
) -> Result<SqliteQuery<'q>> {
    for value in bindings {
        query = bind_text_uuid!(query, value)?;
    }
    Ok(query)
}

/// Render a vector as the pgvector text literal `[a,b,c]`.
#[cfg(feature = "vector")]
fn vector_literal(values: &[f32]) -> String {
    let parts: Vec<String> = values.iter().map(|value| value.to_string()).collect();
    format!("[{}]", parts.join(","))
}

/// Decide the neutral value for a cell that no typed decoder matched.
///
/// A genuine SQL `NULL` becomes [`Value::Null`]; a non-NULL value whose driver
/// type has no decoder is preserved as [`Value::Unsupported`] carrying the type
/// name, so callers never mistake lost data for a real null.
fn fallback_value(is_null: bool, type_name: impl Into<String>) -> Value {
    if is_null {
        Value::Null
    } else {
        Value::Unsupported(type_name.into())
    }
}

/// Decode a single cell into the dialect-neutral [`Value`].
///
/// Tries the common scalar types in widening order. A genuine SQL `NULL` stays
/// [`Value::Null`]; a non-NULL cell with no matching decoder is preserved as
/// [`Value::Unsupported`] with its driver type name instead of silently
/// collapsing to a false null (see [`fallback_value`]).
macro_rules! decode_cell {
    ($row:expr, $index:expr) => {{
        use sqlx::{Row, TypeInfo, ValueRef};
        let row = $row;
        let index = $index;
        if let Ok(value) = row.try_get::<Option<i64>, _>(index) {
            value.map(Value::Int).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<i32>, _>(index) {
            value.map(|v| Value::Int(v as i64)).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<i16>, _>(index) {
            value.map(|v| Value::Int(v as i64)).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<bool>, _>(index) {
            value.map(Value::Bool).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<i8>, _>(index) {
            value.map(|v| Value::Int(v as i64)).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<f64>, _>(index) {
            value.map(Value::Float).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<f32>, _>(index) {
            value.map(|v| Value::Float(v as f64)).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<serde_json::Value>, _>(index) {
            value.map(Value::Json).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<String>, _>(index) {
            value.map(Value::Text).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<uuid::Uuid>, _>(index) {
            value.map(Value::Uuid).unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(index) {
            value
                .map(|v| Value::Text(v.to_rfc3339()))
                .unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<chrono::NaiveDateTime>, _>(index) {
            value
                .map(|v| Value::Text(v.to_string()))
                .unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<chrono::NaiveDate>, _>(index) {
            value
                .map(|v| Value::Text(v.to_string()))
                .unwrap_or(Value::Null)
        } else if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(index) {
            value
                .map(|bytes| {
                    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                    Value::Text(hex)
                })
                .unwrap_or(Value::Null)
        } else {
            // No typed decoder matched. Distinguish a real SQL NULL from a
            // non-NULL value of an unmapped type; the latter must not be lost.
            let is_null = row
                .try_get_raw(index)
                .map(|raw| raw.is_null())
                .unwrap_or(false);
            let type_name = row
                .try_get_raw(index)
                .map(|raw| raw.type_info().name().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            fallback_value(is_null, type_name)
        }
    }};
}

/// Walk a driver row's columns and build a [`DbRow`].
macro_rules! decode_row {
    ($row:expr) => {{
        use sqlx::{Column, Row};
        let row = $row;
        let mut columns = Vec::with_capacity(row.len());
        let mut values = Vec::with_capacity(row.len());
        for (index, column) in row.columns().iter().enumerate() {
            columns.push(column.name().to_string());
            values.push(decode_cell!(row, index));
        }
        DbRow::new(columns, values)
    }};
}

/// Decode a PostgreSQL row into a [`DbRow`].
#[cfg(feature = "postgres")]
pub(super) fn decode_pg_row(row: &sqlx::postgres::PgRow) -> DbRow {
    decode_row!(row)
}

/// Decode a MySQL row into a [`DbRow`].
#[cfg(feature = "mysql")]
pub(super) fn decode_mysql_row(row: &sqlx::mysql::MySqlRow) -> DbRow {
    decode_row!(row)
}

/// Decode a SQLite row into a [`DbRow`].
#[cfg(feature = "sqlite")]
pub(super) fn decode_sqlite_row(row: &sqlx::sqlite::SqliteRow) -> DbRow {
    decode_row!(row)
}

#[cfg(test)]
mod tests {
    use super::fallback_value;
    use crate::types::Value;

    /// A non-NULL cell of an unmapped type must never be reported as SQL NULL.
    ///
    /// SQLite's decoder derives the type from the runtime storage class (always
    /// INTEGER, REAL, TEXT, or BLOB), so the unsupported branch is unreachable
    /// through an in-memory SQLite query; the fallback decision is exercised
    /// directly here.
    #[test]
    fn non_null_unmapped_cell_is_not_null() {
        let value = fallback_value(false, "NUMERIC");
        assert_ne!(value, Value::Null);
        assert_eq!(value, Value::Unsupported("NUMERIC".to_string()));
    }

    /// A genuine SQL NULL must still decode to `Value::Null`.
    #[test]
    fn genuine_null_cell_stays_null() {
        assert_eq!(fallback_value(true, "NUMERIC"), Value::Null);
    }

    /// An unsupported decode marker must be rejected, never written as data.
    #[cfg(feature = "sqlite")]
    #[test]
    fn binding_unsupported_marker_is_rejected() {
        let result = super::bind_sqlite(
            sqlx::query("SELECT 1"),
            &[Value::Unsupported("NUMERIC".to_string())],
        );
        assert!(matches!(
            result,
            Err(crate::error::OrmError::InvalidValue(_))
        ));
    }
}
