//! Bind-value conversion and row decoding over the `sqlx` runtime API.
//!
//! Values from the query builder ([`Value`]) are bound onto `sqlx` queries with
//! [`bind_all`]; result rows are turned into JSON with [`row_to_json`] or into a
//! concrete type with [`decode`]. Everything here uses the runtime API — the
//! compile-time `query!` macros are intentionally not used.

use crate::error::Result;
pub use crate::types::{JsonFilter, Value};
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::query::Query;
use sqlx::{Column, ColumnIndex, Database, Decode, Encode, Row, Type, TypeInfo};

/// Bind a slice of [`Value`]s onto a `sqlx` query, in order.
///
/// `Null` binds as a typed `NULL`; JSON, UUID and timestamps bind natively (the
/// `json`/`uuid`/`chrono` sqlx features are enabled). A vector value binds as its
/// pgvector text literal (`[a,b,c]`) — callers cast it (`$n::vector`) on the
/// Postgres side.
pub fn bind_all<'q, DB>(
    mut query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    values: &[Value],
) -> Query<'q, DB, <DB as Database>::Arguments<'q>>
where
    DB: Database,
    i64: Encode<'q, DB> + Type<DB>,
    f64: Encode<'q, DB> + Type<DB>,
    bool: Encode<'q, DB> + Type<DB>,
    String: Encode<'q, DB> + Type<DB>,
    uuid::Uuid: Encode<'q, DB> + Type<DB>,
    DateTime<Utc>: Encode<'q, DB> + Type<DB>,
    serde_json::Value: Encode<'q, DB> + Type<DB>,
    Option<i64>: Encode<'q, DB> + Type<DB>,
{
    for value in values {
        query = bind_one(query, value.clone());
    }
    query
}

/// Bind a single [`Value`] onto a `sqlx` query.
///
/// A vector value has no native `sqlx` encoding, so it is bound as text.
pub fn bind_one<'q, DB>(
    query: Query<'q, DB, <DB as Database>::Arguments<'q>>,
    value: Value,
) -> Query<'q, DB, <DB as Database>::Arguments<'q>>
where
    DB: Database,
    i64: Encode<'q, DB> + Type<DB>,
    f64: Encode<'q, DB> + Type<DB>,
    bool: Encode<'q, DB> + Type<DB>,
    String: Encode<'q, DB> + Type<DB>,
    uuid::Uuid: Encode<'q, DB> + Type<DB>,
    DateTime<Utc>: Encode<'q, DB> + Type<DB>,
    serde_json::Value: Encode<'q, DB> + Type<DB>,
    Option<i64>: Encode<'q, DB> + Type<DB>,
{
    match value {
        Value::Null => query.bind(Option::<i64>::None),
        Value::Bool(v) => query.bind(v),
        Value::Int(v) => query.bind(v),
        Value::Float(v) => query.bind(v),
        Value::Text(v) => query.bind(v),
        Value::Uuid(v) => query.bind(v),
        Value::Timestamp(v) => query.bind(v),
        Value::Json(v) => query.bind(v),
        #[cfg(feature = "vector")]
        Value::Vector(v) => query.bind(crate::vector::to_vector_literal(&v)),
    }
}

/// Decode one column of a row into a concrete type by index or name.
///
/// The index may be a `usize` ordinal or a `&str` column name. Type mismatches
/// surface as `OrmError::Storage` via the `sqlx::Error` conversion.
pub fn decode<'r, DB, T, I>(row: &'r DB::Row, index: I) -> Result<T>
where
    DB: Database,
    I: ColumnIndex<DB::Row>,
    T: Decode<'r, DB> + Type<DB>,
{
    Ok(row.try_get::<T, _>(index)?)
}

/// Convert a full row into a JSON object keyed by column name.
///
/// Column values are decoded using the column's declared type name as a hint,
/// falling back across integer/float/bool/text/JSON. Unknown or NULL values
/// become `null`.
pub fn row_to_json<DB>(row: &DB::Row) -> Result<serde_json::Value>
where
    DB: Database,
    usize: ColumnIndex<DB::Row>,
    for<'a> i64: Decode<'a, DB> + Type<DB>,
    for<'a> f64: Decode<'a, DB> + Type<DB>,
    for<'a> bool: Decode<'a, DB> + Type<DB>,
    for<'a> String: Decode<'a, DB> + Type<DB>,
    for<'a> serde_json::Value: Decode<'a, DB> + Type<DB>,
    for<'a> uuid::Uuid: Decode<'a, DB> + Type<DB>,
    for<'a> DateTime<Utc>: Decode<'a, DB> + Type<DB>,
    for<'a> Option<i64>: Decode<'a, DB> + Type<DB>,
{
    let mut object = serde_json::Map::new();
    for (index, column) in row.columns().iter().enumerate() {
        let value = decode_column_json::<DB>(row, index, column.type_info().name());
        object.insert(column.name().to_string(), value);
    }
    Ok(serde_json::Value::Object(object))
}

/// Decode one column to JSON, guided by its declared type name.
fn decode_column_json<DB>(row: &DB::Row, index: usize, type_name: &str) -> serde_json::Value
where
    DB: Database,
    usize: ColumnIndex<DB::Row>,
    for<'a> i64: Decode<'a, DB> + Type<DB>,
    for<'a> f64: Decode<'a, DB> + Type<DB>,
    for<'a> bool: Decode<'a, DB> + Type<DB>,
    for<'a> String: Decode<'a, DB> + Type<DB>,
    for<'a> serde_json::Value: Decode<'a, DB> + Type<DB>,
    for<'a> uuid::Uuid: Decode<'a, DB> + Type<DB>,
    for<'a> DateTime<Utc>: Decode<'a, DB> + Type<DB>,
    for<'a> Option<i64>: Decode<'a, DB> + Type<DB>,
{
    if let Ok(None) = row.try_get::<Option<i64>, _>(index) {
        return serde_json::Value::Null;
    }

    let upper = type_name.to_ascii_uppercase();
    // UUID is decoded before the numeric fallbacks: SQLite stores it as a
    // 16-byte BLOB and MySQL as BINARY(16), which the numeric fallbacks would
    // otherwise misread.
    if upper.contains("UUID") || upper.contains("BLOB") || upper.contains("BINARY") {
        if let Ok(v) = row.try_get::<uuid::Uuid, _>(index) {
            return serde_json::Value::String(v.to_string());
        }
    }
    // Datetimes are decoded to RFC3339 so models deserialize consistently
    // across drivers (SQLite stores them as TEXT).
    if upper.contains("DATE") || upper.contains("TIME") {
        if let Ok(v) = row.try_get::<DateTime<Utc>, _>(index) {
            return serde_json::Value::String(v.to_rfc3339_opts(SecondsFormat::Micros, true));
        }
    }
    if upper.contains("BOOL") {
        if let Ok(v) = row.try_get::<bool, _>(index) {
            return serde_json::Value::Bool(v);
        }
    }
    if upper.contains("JSON") {
        if let Ok(v) = row.try_get::<serde_json::Value, _>(index) {
            return v;
        }
    }
    if upper.contains("INT") {
        if let Ok(v) = row.try_get::<i64, _>(index) {
            return serde_json::Value::from(v);
        }
    }
    if upper.contains("REAL")
        || upper.contains("FLOAT")
        || upper.contains("DOUBLE")
        || upper.contains("NUMERIC")
        || upper.contains("DECIMAL")
    {
        if let Ok(v) = row.try_get::<f64, _>(index) {
            return serde_json::Value::from(v);
        }
    }

    if let Ok(v) = row.try_get::<i64, _>(index) {
        return serde_json::Value::from(v);
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return serde_json::Value::from(v);
    }
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.try_get::<serde_json::Value, _>(index) {
        return v;
    }
    serde_json::Value::Null
}

/// Whether a [`Value`] can be bound natively without a dialect cast.
///
/// Vector values are bound as text and therefore require a `$n::vector` cast.
pub fn is_natively_bindable(value: &Value) -> bool {
    match value {
        #[cfg(feature = "vector")]
        Value::Vector(_) => false,
        _ => true,
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    /// Verifies scalar values bind and round-trip through SQLite.
    #[tokio::test]
    async fn binds_scalars_and_decodes_row() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let values = vec![
            Value::Int(7),
            Value::Text("hello".into()),
            Value::Bool(true),
            Value::Null,
        ];
        let query = bind_all::<sqlx::Sqlite>(sqlx::query("SELECT ?1, ?2, ?3, ?4"), &values);
        let row = query.fetch_one(&pool).await.unwrap();

        assert_eq!(decode::<sqlx::Sqlite, i64, _>(&row, 0).unwrap(), 7);
        assert_eq!(
            decode::<sqlx::Sqlite, String, _>(&row, 1).unwrap(),
            "hello".to_string()
        );
        assert!(decode::<sqlx::Sqlite, bool, _>(&row, 2).unwrap());
        assert!(decode::<sqlx::Sqlite, Option<i64>, _>(&row, 3)
            .unwrap()
            .is_none());

        let json = row_to_json::<sqlx::Sqlite>(&row).unwrap();
        assert!(json.is_object());
        pool.close().await;
    }
}
