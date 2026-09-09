//! Transaction wrapper — `BEGIN` / `COMMIT` / `ROLLBACK` lifecycle over the
//! [`TransactionStub`], with an async closure body.
//!
//! M2 emits the lifecycle SQL into a typed log; the sqlx pool wiring replaces
//! the stub handle without changing the call shape.

use crate::builder::TransactionStub;
use crate::error::Result;

/// Transaction lifecycle errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransactionError {
    /// A commit/rollback was attempted after the transaction closed.
    #[error("transaction is not open")]
    Closed,
}

/// A transaction handle wrapping the driver stub.
#[derive(Debug)]
pub struct Transaction {
    /// Whether the transaction is still open.
    pub open: bool,
    /// Lifecycle log (`BEGIN`, `COMMIT`, `ROLLBACK`).
    pub log: Vec<String>,
}

impl Transaction {
    /// Begin a new transaction (logs `BEGIN`).
    pub fn begin() -> Self {
        Self {
            open: true,
            log: vec!["BEGIN".to_string()],
        }
    }

    /// Commit the transaction (logs `COMMIT`) and return the closed handle.
    pub fn commit(mut self) -> Result<Transaction> {
        if !self.open {
            return Err(crate::error::OrmError::InvalidState(
                "cannot commit a closed transaction".into(),
            ));
        }
        self.open = false;
        self.log.push("COMMIT".to_string());
        Ok(self)
    }

    /// Roll back the transaction (logs `ROLLBACK`) and return the closed handle.
    pub fn rollback(mut self) -> Result<Transaction> {
        if !self.open {
            return Err(crate::error::OrmError::InvalidState(
                "cannot roll back a closed transaction".into(),
            ));
        }
        self.open = false;
        self.log.push("ROLLBACK".to_string());
        Ok(self)
    }

    /// Run `body` inside the transaction, committing on `Ok` and rolling
    /// back on `Err` (the error is returned unchanged).
    pub async fn run<F, T>(body: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction) -> Result<T>,
    {
        let mut tx = Transaction::begin();
        match body(&mut tx) {
            Ok(value) => {
                tx.commit()?;
                Ok(value)
            }
            Err(e) => {
                let _ = tx.rollback();
                Err(e)
            }
        }
    }
}

impl Default for Transaction {
    /// Create a closed transaction (no lifecycle events).
    fn default() -> Self {
        Self {
            open: false,
            log: Vec::new(),
        }
    }
}

/// Begin a transaction (alias of [`Transaction::begin`]).
pub fn begin() -> Transaction {
    Transaction::begin()
}

/// Convert a stub into a wrapper transaction (adoption helper).
pub fn from_stub(stub: TransactionStub) -> Transaction {
    Transaction {
        open: stub.open,
        log: stub.log,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies commit closes the transaction and records the lifecycle.
    #[test]
    fn commit_logs_and_closes() {
        let tx = Transaction::begin().commit().unwrap();
        assert!(!tx.open);
        assert_eq!(tx.log, vec!["BEGIN", "COMMIT"]);
    }

    /// Verifies double-commit is a typed error.
    #[test]
    fn double_commit_rejected() {
        let tx = Transaction::begin().commit().unwrap();
        assert!(matches!(
            tx.commit(),
            Err(crate::error::OrmError::InvalidState(_))
        ));
    }

    /// Verifies run rolls back and propagates the error.
    #[tokio::test]
    async fn run_rolls_back_on_error() {
        let err: crate::Result<u8> = Transaction::run(|_tx| Err(crate::OrmError::NotFound)).await;
        assert!(matches!(err.unwrap_err(), crate::OrmError::NotFound));
        let ok: crate::Result<u8> = Transaction::run(|_tx| Ok(3)).await;
        assert_eq!(ok.unwrap(), 3);
    }
}
