//! Bind values and JSON filter primitives shared by builder and model layers.

use crate::error::{OrmError, Result};
use serde::{Deserialize, Serialize};

/// A single bind value for a prepared statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// NULL.
    Null,
    /// Boolean.
    Bool(bool),
    /// 64-bit integer.
    Int(i64),
    /// 64-bit float (never used for monetary values — `Decimal` in M4).
    Float(f64),
    /// UTF-8 text.
    Text(String),
    /// UUID as text.
    Uuid(uuid::Uuid),
    /// JSON value.
    Json(serde_json::Value),
    /// Vector of floats (pgvector embedding).
    #[cfg(feature = "vector")]
    Vector(Vec<f32>),
}

impl Value {
    /// Render the value as a literal for `toRawSql` diagnostics.
    ///
    /// Always parameterized in real execution — raw SQL is display-only.
    pub fn to_literal(&self) -> String {
        match self {
            Value::Null => "NULL".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{f}"),
            Value::Text(s) => format!("'{}'", s.replace('\'', "''")),
            Value::Uuid(u) => format!("'{u}'"),
            Value::Json(v) => {
                format!(
                    "'{}'",
                    serde_json::to_string(v)
                        .unwrap_or_default()
                        .replace('\'', "''")
                )
            }
            #[cfg(feature = "vector")]
            Value::Vector(v) => {
                let parts: Vec<String> = v.iter().map(|f| f.to_string()).collect();
                format!("[{}]", parts.join(","))
            }
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<uuid::Uuid> for Value {
    fn from(u: uuid::Uuid) -> Self {
        Value::Uuid(u)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

/// Filter operator applied to a JSON path expression.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonFilter {
    /// `path @> $n` — JSON contains the given value.
    Contains(serde_json::Value),
    /// `path ->> key = $n` — key equality against text.
    PathEquals(String, String),
    /// `json_exists(path)` / `path ? key` — key exists.
    KeyExists(String),
}

impl JsonFilter {
    /// Bind value for this filter, if any.
    ///
    /// `Contains` binds the contained JSON; `PathEquals` binds the expected text
    /// at the path; `KeyExists` has no bind value.
    pub fn bind_value(&self) -> Option<Value> {
        match self {
            JsonFilter::Contains(value) => Some(Value::Json(value.clone())),
            JsonFilter::PathEquals(_, value) => Some(Value::Text(value.clone())),
            JsonFilter::KeyExists(_) => None,
        }
    }

    /// Compile the filter to a SQL fragment for the given driver dialect.
    pub fn to_sql(&self, column: &str, dialect: &str) -> Result<String> {
        match self {
            JsonFilter::Contains(_) => match dialect {
                "postgres" => Ok(format!("{column} @> {{}}")),
                "mysql" => Ok(format!("JSON_CONTAINS({column}, {{}})")),
                "sqlite" => Err(OrmError::UnsupportedDriver(
                    "sqlite has no native JSON contains operator".into(),
                )),
                other => Err(OrmError::UnsupportedDriver(other.to_string())),
            },
            JsonFilter::PathEquals(key, _) => match dialect {
                "postgres" => Ok(format!("{column} ->> '{{{key}}}' = {{}}")),
                "mysql" => Ok(format!(
                    "JSON_UNQUOTE(JSON_EXTRACT({column}, '$.{key}')) = {{}}"
                )),
                "sqlite" => Ok(format!("json_extract({column}, '$.{key}') = {{}}")),
                other => Err(OrmError::UnsupportedDriver(other.to_string())),
            },
            JsonFilter::KeyExists(key) => match dialect {
                "postgres" => Ok(format!("{column} ? '{key}'")),
                "mysql" => Ok(format!("JSON_CONTAINS_PATH({column}, 'one', '$.{key}')")),
                "sqlite" => Ok(format!("json_type({column}, '$.{key}') IS NOT NULL")),
                other => Err(OrmError::UnsupportedDriver(other.to_string())),
            },
        }
    }
}

/// Schema column type used by Blueprint/migration helpers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// UUID primary key.
    Uuid,
    /// Variable-length text.
    Text,
    /// 64-bit integer.
    BigInt,
    /// `NUMERIC(15,2)` — mandated for money/quantities.
    Decimal,
    /// Boolean flag.
    Boolean,
    /// `JSONB` (Postgres) / `JSON` (MySQL) / `TEXT` (SQLite).
    Json,
    /// Timestamp with time zone.
    TimestampTz,
    /// pgvector embedding column of fixed dimension.
    #[cfg(feature = "vector")]
    Vector(u32),
}

impl ColumnType {
    /// Emit the dialect-specific SQL type for this column.
    pub fn to_sql(&self, dialect: &str) -> Result<String> {
        match self {
            ColumnType::Uuid => Ok(match dialect {
                "postgres" => "UUID".to_string(),
                "mysql" => "CHAR(36)".to_string(),
                "sqlite" => "TEXT".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            ColumnType::Text => Ok(match dialect {
                "postgres" => "TEXT".to_string(),
                "mysql" => "VARCHAR(255)".to_string(),
                "sqlite" => "TEXT".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            ColumnType::BigInt => Ok(match dialect {
                "postgres" => "BIGINT".to_string(),
                "mysql" => "BIGINT".to_string(),
                "sqlite" => "INTEGER".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            ColumnType::Decimal => Ok(match dialect {
                "postgres" => "NUMERIC(15,2)".to_string(),
                "mysql" => "DECIMAL(15,2)".to_string(),
                "sqlite" => "NUMERIC(15,2)".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            ColumnType::Boolean => Ok(match dialect {
                "postgres" => "BOOLEAN".to_string(),
                "mysql" => "TINYINT(1)".to_string(),
                "sqlite" => "INTEGER".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            ColumnType::Json => Ok(match dialect {
                "postgres" => "JSONB".to_string(),
                "mysql" => "JSON".to_string(),
                "sqlite" => "TEXT".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            ColumnType::TimestampTz => Ok(match dialect {
                "postgres" => "TIMESTAMPTZ".to_string(),
                "mysql" => "DATETIME".to_string(),
                "sqlite" => "TEXT".to_string(),
                other => return Err(OrmError::UnsupportedDriver(other.to_string())),
            }),
            #[cfg(feature = "vector")]
            ColumnType::Vector(dim) => match dialect {
                "postgres" => Ok(format!("VECTOR({dim})")),
                "mysql" => Ok(format!("VECTOR({dim})")),
                other => Err(OrmError::UnsupportedDriver(format!(
                    "{other} does not support vector columns"
                ))),
            },
        }
    }
}
