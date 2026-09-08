//! SQL clause primitives shared by the query builder and execution helpers.

use crate::error::{OrmError, Result};
use crate::types::Value;

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

/// A raw SQL fragment emitted inline (never parameterized).
#[derive(Debug, Clone)]
pub struct SqlFragment {
    /// SQL text spliced verbatim into the statement.
    pub sql: String,
}

impl From<&str> for SqlFragment {
    /// Wrap a static fragment.
    fn from(sql: &str) -> Self {
        Self {
            sql: sql.to_string(),
        }
    }
}

impl From<String> for SqlFragment {
    /// Wrap an owned fragment.
    fn from(sql: String) -> Self {
        Self { sql }
    }
}

/// Transaction handle stub — real `BEGIN/COMMIT/ROLLBACK` wired in S03-T01.
#[derive(Debug)]
pub struct TransactionStub {
    /// Whether the transaction is still open.
    pub open: bool,
    /// Human-readable log of the lifecycle events observed so far.
    pub log: Vec<String>,
}

impl TransactionStub {
    /// Begin a transaction (stub — logs `BEGIN`).
    pub fn begin() -> Self {
        Self {
            open: true,
            log: vec!["BEGIN".to_string()],
        }
    }

    /// Commit the transaction (stub — logs `COMMIT`).
    pub fn commit(mut self) -> Result<()> {
        if !self.open {
            return Err(OrmError::InvalidState(
                "cannot commit a closed transaction".into(),
            ));
        }
        self.open = false;
        self.log.push("COMMIT".to_string());
        Ok(())
    }

    /// Roll back the transaction (stub — logs `ROLLBACK`).
    pub fn rollback(mut self) -> Result<()> {
        if !self.open {
            return Err(OrmError::InvalidState(
                "cannot roll back a closed transaction".into(),
            ));
        }
        self.open = false;
        self.log.push("ROLLBACK".to_string());
        Ok(())
    }
}
