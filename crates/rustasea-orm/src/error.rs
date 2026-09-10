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

    /// Underlying storage/IO failure surfaced by the driver.
    #[error("storage error: {0}")]
    Storage(String),

    /// A query referenced a table that does not exist (or was not created).
    #[error("missing table: {table}")]
    MissingTable {
        /// Name of the missing table.
        table: String,
    },

    /// Connection-pool setup or acquisition failure (`connect`, `ping`).
    #[error("connection pool error: {0}")]
    Pool(String),

    /// Migration failure.
    #[error(transparent)]
    Migration(#[from] crate::migration::MigrationError),

    /// Vector dimension mismatch: the column expects `expected` dimensions but
    /// the supplied embedding has `actual`.
    #[error("vector dimension mismatch: expected {expected}, got {actual}")]
    VectorDimensionMismatch {
        /// Expected column dimension.
        expected: usize,
        /// Actual embedding length.
        actual: usize,
    },
}

impl From<sqlx::Error> for OrmError {
    /// Map a raw `sqlx` failure onto the ORM's storage error.
    fn from(error: sqlx::Error) -> Self {
        OrmError::Storage(error.to_string())
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
