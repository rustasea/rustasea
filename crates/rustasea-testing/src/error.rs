//! Shared test error type and result alias (harness contract).
//!
//! Mirrors the error catalogue from `testing-harness.md` §5: container
//! startup that exceeds 30s is a [`ContainerTimeout`](TestError::ContainerTimeout);
//! rendering a paginator without its data rows is a
//! [`PaginatorMissing`](TestError::PaginatorMissing).

/// Shared test error type for the harness.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    /// A testcontainers-backed store did not become healthy within 30s.
    #[error("container did not become healthy within the 30s timeout")]
    ContainerTimeout,

    /// The paginator view was asked to render without any rows.
    #[error("paginator has no data rows to render")]
    PaginatorMissing,

    /// The test environment could not be provisioned.
    #[error("test setup failed: {0}")]
    Setup(String),

    /// An io error while loading `.env.testing` or fixtures.
    #[error("io error: {0}")]
    Io(std::io::Error),
}

/// Alias for results produced by the test harness.
pub type Result<T> = std::result::Result<T, TestError>;

impl From<std::io::Error> for TestError {
    /// Lift an io error into the typed test error.
    fn from(e: std::io::Error) -> Self {
        TestError::Io(e)
    }
}
