//! Async model persistence — create/save/update/delete/refresh over a [`DbPool`].
//!
//! [`ModelOps`] is blanket-implemented for every [`Model`]. Column values are
//! taken from the model's `serde` representation, so the operations stay generic
//! across derived models; `created_at`/`updated_at`/`deleted_at` are written as
//! dialect-aware current timestamps. Only the `sqlx` runtime API is used — never
//! the compile-time `query!` macros.

use crate::builder::dialect;
use crate::db::DbPool;
use crate::error::{OrmError, Result};
use crate::m2::UpsertBuilder;
use crate::model::Model;
use crate::types::Value;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

/// Fields that are managed by the ORM and never written as user columns.
const RESERVED_COLUMNS: &[&str] = &["id", "created_at", "updated_at", "deleted_at", "relations"];

/// Async persistence operations shared by every model.
#[allow(async_fn_in_trait)]
pub trait ModelOps: Model + Sized {
    /// Insert `data`, auto-assigning an id and the timestamp columns.
    ///
    /// Returns the persisted row re-read from the database.
    async fn create(pool: &DbPool, mut data: Self) -> Result<Self>
    where
        Self: Serialize + DeserializeOwned,
    {
        if data.primary_key() == Uuid::nil() {
            data.assign_id();
        }
        let id = data.primary_key();
        let (sql, bindings) = build_insert::<Self>(&data)?;
        pool.execute_bind(&sql, &bindings).await?;
        Ok(Self::refresh(pool, id).await?.unwrap_or(data))
    }

    /// Persist `data` with a single atomic upsert (insert-or-update).
    ///
    /// Replaces the former exists-then-branch check (a TOCTOU race): the
    /// database resolves insert-vs-update in one statement — Postgres/SQLite
    /// `INSERT … ON CONFLICT (id) DO UPDATE`, MySQL `INSERT … ON DUPLICATE KEY
    /// UPDATE`. The row is re-read with [`ModelOps::refresh`] afterwards.
    async fn save(pool: &DbPool, mut data: Self) -> Result<Self>
    where
        Self: Serialize + DeserializeOwned,
    {
        if data.primary_key() == Uuid::nil() {
            data.assign_id();
        }
        let id = data.primary_key();
        let (sql, bindings) = build_upsert::<Self>(&data)?;
        pool.execute_bind(&sql, &bindings).await?;
        Ok(Self::refresh(pool, id).await?.unwrap_or(data))
    }

    /// Update the row identified by the model's primary key.
    ///
    /// Bumps `updated_at` when the model tracks timestamps; returns the
    /// re-read persisted row.
    async fn update(pool: &DbPool, data: Self) -> Result<Self>
    where
        Self: Serialize + DeserializeOwned,
    {
        let id = data.primary_key();
        let (sql, bindings) = build_update::<Self>(&data)?;
        let affected = pool.execute_bind(&sql, &bindings).await?;
        if affected == 0 {
            return Err(OrmError::NotFound);
        }
        Ok(Self::refresh(pool, id).await?.unwrap_or(data))
    }

    /// Delete the row: soft delete when the model soft-deletes, else hard delete.
    async fn delete(pool: &DbPool, id: Uuid) -> Result<bool> {
        if Self::uses_soft_deletes() {
            Self::soft_delete(pool, id).await
        } else {
            Self::force_delete(pool, id).await
        }
    }

    /// Permanently remove the row, bypassing soft deletes.
    async fn force_delete(pool: &DbPool, id: Uuid) -> Result<bool> {
        let sql = Self::delete_sql(&Self::table_name());
        let affected = pool.execute_bind(&sql, &[Value::Uuid(id)]).await?;
        Ok(affected > 0)
    }

    /// Soft-delete the row by setting `deleted_at` to now.
    async fn soft_delete(pool: &DbPool, id: Uuid) -> Result<bool> {
        let mut bindings = vec![Value::Uuid(id)];
        let expr = match dialect() {
            "sqlite" => {
                bindings.push(timestamp_value(Utc::now()));
                "$2"
            }
            _ => "NOW()",
        };
        let sql = format!(
            "UPDATE {} SET deleted_at = {expr} WHERE id = $1",
            Self::table_name(),
        );
        let affected = pool.execute_bind(&sql, &bindings).await?;
        Ok(affected > 0)
    }

    /// Re-read the row by primary key, including soft-deleted rows.
    async fn refresh(pool: &DbPool, id: Uuid) -> Result<Option<Self>>
    where
        Self: DeserializeOwned,
    {
        let row = Self::query_with_trashed()
            .where_key(id)
            .first(pool)
            .await?;
        match row {
            Some(value) => Ok(Some(crate::builder::json_to_model(value)?)),
            None => Ok(None),
        }
    }

    /// Load the row `FOR UPDATE`, returning [`OrmError::NotFound`] when absent.
    ///
    /// The pessimistic lock is driver-gated: SQLite has no row locks and
    /// surfaces [`OrmError::UnsupportedDriver`].
    async fn first_for_update(pool: &DbPool, id: Uuid) -> Result<Self>
    where
        Self: DeserializeOwned,
    {
        let row = <Self as Model>::first_for_update(id)?.first(pool).await?;
        match row {
            Some(value) => crate::builder::json_to_model(value),
            None => Err(OrmError::NotFound),
        }
    }
}

impl<T: Model> ModelOps for T {}

/// Collect the insertable `(column, value)` pairs for a model instance.
///
/// User columns come from the model's `serde` object (reserved fields skipped);
/// the primary key is first and `created_at`/`updated_at` are appended as the
/// current timestamp. Shared by [`build_insert`] and [`build_upsert`].
fn insert_columns_and_bindings<T: Model + Serialize>(data: &T) -> Result<(Vec<String>, Vec<Value>)> {
    let columns = user_columns(data)?;
    let timestamps = T::insert_columns();

    let mut names: Vec<String> = Vec::with_capacity(columns.len() + timestamps.len() + 1);
    let mut bindings: Vec<Value> = Vec::with_capacity(columns.len() + timestamps.len() + 1);
    names.push("id".to_string());
    bindings.push(Value::Uuid(data.primary_key()));
    for (column, value) in &columns {
        names.push(column.clone());
        bindings.push(value.clone());
    }
    if !timestamps.is_empty() {
        let now = timestamp_value(Utc::now());
        for ts in &timestamps {
            names.push((*ts).to_string());
            bindings.push(now.clone());
        }
    }
    Ok((names, bindings))
}

/// Build the INSERT statement and bindings for a model instance.
fn build_insert<T: Model + Serialize>(data: &T) -> Result<(String, Vec<Value>)> {
    let table = T::table_name();
    let (names, bindings) = insert_columns_and_bindings(data)?;
    let placeholders: Vec<String> = (1..=bindings.len()).map(|i| format!("${i}")).collect();
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        names.join(", "),
        placeholders.join(", ")
    );
    Ok((sql, bindings))
}

/// Build the atomic dialect-aware upsert statement and bindings for a model.
///
/// Postgres/SQLite route through the [`Model::upsert_sql`] path, emitting
/// `INSERT … ON CONFLICT (id) DO UPDATE SET …`; MySQL emits
/// `INSERT … ON DUPLICATE KEY UPDATE …`. `created_at` is preserved on conflict
/// (excluded from the update set) while `updated_at` is refreshed. Resolving
/// insert-vs-update in one statement removes the previous check-then-act race.
fn build_upsert<T: Model + Serialize>(data: &T) -> Result<(String, Vec<Value>)> {
    let table = T::table_name();
    let (names, bindings) = insert_columns_and_bindings(data)?;

    let mut builder = UpsertBuilder::table(table.clone()).unique_by(&["id"])?;
    for name in &names {
        builder = builder.column(name);
    }
    builder = builder.exclude(&["created_at"]);

    let sql = match dialect() {
        "mysql" => mysql_upsert_sql(&table, &names),
        _ => T::upsert_sql(&builder)?,
    };
    Ok((sql, bindings))
}

/// Emit the MySQL upsert shape: `INSERT … ON DUPLICATE KEY UPDATE col = VALUES(col)`.
///
/// The primary key and `created_at` are preserved on conflict; every other
/// inserted column is refreshed from the incoming row.
fn mysql_upsert_sql(table: &str, columns: &[String]) -> String {
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${i}")).collect();
    let updates: Vec<String> = columns
        .iter()
        .filter(|column| column.as_str() != "id" && column.as_str() != "created_at")
        .map(|column| format!("{column} = VALUES({column})"))
        .collect();
    format!(
        "INSERT INTO {table} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
        columns.join(", "),
        placeholders.join(", "),
        updates.join(", ")
    )
}

/// Build the UPDATE statement and bindings for a model instance.
///
/// The primary key is `$1`; `updated_at` is appended as a bound timestamp.
fn build_update<T: Model + Serialize>(data: &T) -> Result<(String, Vec<Value>)> {
    let table = T::table_name();
    let columns = user_columns(data)?;

    let mut bindings: Vec<Value> = vec![Value::Uuid(data.primary_key())];
    let mut assignments: Vec<String> = Vec::with_capacity(columns.len() + 1);
    for (column, value) in &columns {
        bindings.push(value.clone());
        assignments.push(format!("{column} = ${}", bindings.len()));
    }
    if let Some(updated) = T::updated_column() {
        bindings.push(timestamp_value(Utc::now()));
        assignments.push(format!("{updated} = ${}", bindings.len()));
    }

    let sql = format!(
        "UPDATE {table} SET {} WHERE id = $1",
        assignments.join(", ")
    );
    Ok((sql, bindings))
}

/// Extract the writable `(column, value)` pairs from a model's serde object.
fn user_columns<T: Serialize>(data: &T) -> Result<Vec<(String, Value)>> {
    let value = serde_json::to_value(data)
        .map_err(|error| OrmError::Storage(format!("model serialization failed: {error}")))?;
    let object = value.as_object().ok_or_else(|| {
        OrmError::InvalidValue("model must serialize to a JSON object".into())
    })?;

    let mut columns = Vec::new();
    for (column, value) in object {
        if RESERVED_COLUMNS.contains(&column.as_str()) {
            continue;
        }
        columns.push((column.clone(), json_to_value(column, value)?));
    }
    Ok(columns)
}

/// Whether `column` holds a UUID and therefore must bind natively as one.
///
/// The primary key (`id`) and foreign keys/UUID columns (`*_id`, `*_uuid`) are
/// covered; every other string column stays text.
fn is_uuid_column(column: &str) -> bool {
    column == "id" || column.ends_with("_id") || column.ends_with("_uuid")
}

/// Convert a JSON field value into a bind [`Value`].
///
/// The primary key and `*_id`/`*_uuid` columns are decoded to a UUID so they
/// bind against UUID columns; a string that fails to parse in one of those
/// columns is a typed [`OrmError::InvalidValue`] rather than a silent text bind.
fn json_to_value(column: &str, value: &serde_json::Value) -> Result<Value> {
    Ok(match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(int) => Value::Int(int),
            None => Value::Float(number.as_f64().unwrap_or_default()),
        },
        serde_json::Value::String(text) => {
            if is_uuid_column(column) {
                match Uuid::parse_str(text) {
                    Ok(id) => return Ok(Value::Uuid(id)),
                    Err(_) => {
                        return Err(OrmError::InvalidValue(format!(
                            "column `{column}` expects a UUID, got `{text}`"
                        )))
                    }
                }
            }
            Value::Text(text.clone())
        }
        other => Value::Json(other.clone()),
    })
}

/// The current timestamp as a bind value for the active dialect.
///
/// SQLite compares TEXT datetimes lexicographically, so the stored form is a
/// fixed-width RFC3339 string; Postgres/MySQL receive a native `DateTime<Utc>`
/// bound against their `timestamp`/`datetime` columns.
fn timestamp_value(now: DateTime<Utc>) -> Value {
    match dialect() {
        "sqlite" => Value::Text(now.to_rfc3339_opts(SecondsFormat::Micros, true)),
        _ => Value::Timestamp(now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed UUID string used across the type-fidelity assertions.
    const UUID_TEXT: &str = "f3c1f3c1-0000-4000-8000-000000000000";

    /// Verifies `*_id`/`*_uuid` strings bind as native UUIDs, not text.
    #[test]
    fn uuid_columns_bind_native_uuid() {
        for column in ["id", "user_id", "owner_uuid"] {
            let value = json_to_value(column, &serde_json::Value::String(UUID_TEXT.into())).unwrap();
            assert_eq!(value, Value::Uuid(Uuid::parse_str(UUID_TEXT).unwrap()));
        }
    }

    /// Verifies ordinary text columns keep a UUID-looking string as text.
    #[test]
    fn non_uuid_columns_keep_text() {
        let value = json_to_value("nickname", &serde_json::Value::String(UUID_TEXT.into())).unwrap();
        assert_eq!(value, Value::Text(UUID_TEXT.into()));
    }

    /// Verifies a non-UUID string in a UUID column is a typed error, not a bind.
    #[test]
    fn invalid_uuid_column_value_is_typed_error() {
        let error = json_to_value("user_id", &serde_json::Value::String("not-a-uuid".into()))
            .expect_err("must reject non-UUID in a UUID column");
        assert!(matches!(error, OrmError::InvalidValue(_)), "got {error:?}");
    }
}
