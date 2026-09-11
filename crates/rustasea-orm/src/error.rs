/// Typed errors for the ORM layer.
use thiserror::Error;

/// Alias for results produced by ORM operations.
pub type Result<T> = std::result::Result<T, OrmError>;

/// Top-level ORM error type.
#[derive(Debug, Error)]
pub enum OrmError {
    /// No row matched the query (`firstOrFail`, `findOrFail`).
    #[error("model not found")]
    NotFound,

    /// Strict upsert was called with an empty `uniqueBy` list.
    #[error("upsert rejected: {0}")]
    Upsert(#[from] UpsertError),

    /// Invalid builder state (e.g. paginating an unpaged query twice).
    #[error("invalid query state: {0}")]
    InvalidState(String),

    /// A value could not be bound to the query.
    #[error("invalid bind value: {0}")]
    InvalidValue(String),

    /// The configured driver is unsupported for this operation.
    #[error("unsupported driver: {0}")]
    UnsupportedDriver(String),

    /// Underlying storage/IO failure (reserved for sqlx wiring in S03-T01 follow-up).
    #[error("storage error: {0}")]
    Storage(String),

    /// A connection to the database could not be established.
    #[error("database connection error: {0}")]
    Connection(String),

    /// A SQL statement failed at the database.
    #[error("query error: {0}")]
    Query(String),

    /// The connection pool is closed or an acquisition timed out.
    #[error("database pool closed or timed out")]
    PoolClosed,

    /// Database configuration is missing or invalid.
    #[error("database configuration error: {0}")]
    Configuration(String),

    /// Generic database failure that does not fit a more specific variant.
    #[error("database error: {0}")]
    Database(String),

    /// Migration failure.
    #[error(transparent)]
    Migration(#[from] crate::migration::MigrationError),

    /// Vector extension failure (dimension mismatch, missing extension).
    #[error("vector error: {0}")]
    Vector(String),
}

impl From<sqlx::Error> for OrmError {
    /// Map a `sqlx` failure onto the closest typed ORM error.
    ///
    /// `RowNotFound` collapses to [`OrmError::NotFound`] so `fetch_one` keeps
    /// the same "missing row" contract as `firstOrFail`. Transport-level
    /// failures (`Io`, `Tls`) map to [`OrmError::Connection`] so callers can
    /// distinguish an unreachable host from a failed query.
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => OrmError::NotFound,
            sqlx::Error::PoolClosed | sqlx::Error::PoolTimedOut => OrmError::PoolClosed,
            sqlx::Error::Configuration(message) => OrmError::Configuration(message.to_string()),
            sqlx::Error::Database(message) => OrmError::Query(message.to_string()),
            sqlx::Error::Io(err) => OrmError::Connection(err.to_string()),
            sqlx::Error::Tls(err) => OrmError::Connection(err.to_string()),
            other => OrmError::Database(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Maps an I/O failure from `sqlx` onto the dedicated connection variant.
    #[test]
    fn io_error_maps_to_connection() {
        let io = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let error: OrmError = sqlx::Error::Io(io).into();
        match error {
            OrmError::Connection(message) => assert!(message.contains("connection refused")),
            other => panic!("expected OrmError::Connection, got {other:?}"),
        }
    }
}

/// Strict upsert errors — thrown before any round-trip.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UpsertError {
    /// `upsert(rows, unique_by: [])` — previously silent, now strict.
    #[error("uniqueBy must be non-empty")]
    EmptyUniqueBy,

    /// Upsert row count exceeded the configured batch limit.
    #[error("upsert batch too large: {0} rows")]
    BatchTooLarge(usize),
}
