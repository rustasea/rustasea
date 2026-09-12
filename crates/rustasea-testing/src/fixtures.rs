//! Isolated Postgres fixtures — the testcontainers-backed equivalent of
//! `#[sqlx::test]`.
//!
//! `#[sqlx::test]` provisions a throwaway database on a server reachable via
//! `DATABASE_URL`; this harness instead starts a dedicated Postgres container
//! per fixture (reusing [`crate::containers::postgres_container`]) so a suite
//! needs no external database or management server. Typical usage:
//!
//! ```rust,ignore
//! use rustasea_testing::fixtures::PostgresTestDb;
//!
//! #[tokio::test]
//! #[ignore = "requires docker"]
//! async fn feature_round_trip() {
//!     let db = PostgresTestDb::start("feature-users").await.unwrap();
//!     db.pool()
//!         .execute_script("CREATE TABLE users (id BIGINT PRIMARY KEY)")
//!         .await
//!         .unwrap();
//!     // ... drive the application against `db.pool()` ...
//!     db.shutdown().await.unwrap();
//! }
//! ```
//!
//! Feature `postgres` gates this module.

use rustasea_orm::{DbPool, OrmError};

use crate::containers::{postgres_container, teardown, ContainerError, ContainerHandle};

/// Errors produced while provisioning an isolated database fixture.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// The Postgres container could not be started or torn down.
    #[error(transparent)]
    Container(#[from] ContainerError),

    /// The pool could not connect to the isolated database.
    #[error("database error: {0}")]
    Database(#[from] OrmError),
}

/// An isolated Postgres database provisioned for a single test.
///
/// The fixture owns a dedicated container, exposes a connected [`DbPool`], and
/// tears both down via [`PostgresTestDb::shutdown`]. Call `shutdown` (or
/// [`crate::containers::teardown_all`]) so containers do not outlive the suite.
#[derive(Debug)]
pub struct PostgresTestDb {
    /// Container handle backing this fixture (name/port/url).
    handle: ContainerHandle,
    /// Pool bound to the isolated database.
    pool: DbPool,
}

impl PostgresTestDb {
    /// Provision a dedicated Postgres container and connect a pool to it.
    ///
    /// `suffix` names the container (`rustasea-test-<pid>-<suffix>`) so parallel
    /// suites never collide. Startup failure surfaces as
    /// [`FixtureError::Container`]; a failed connection as
    /// [`FixtureError::Database`].
    pub async fn start(suffix: &str) -> Result<Self, FixtureError> {
        let handle = postgres_container(suffix).await?;
        let pool = DbPool::connect(&handle.url).await?;
        Ok(Self { handle, pool })
    }

    /// The live pool bound to the isolated database.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// The connection URL of the isolated database.
    pub fn url(&self) -> &str {
        &self.handle.url
    }

    /// The container name backing this fixture.
    pub fn name(&self) -> &str {
        &self.handle.name
    }

    /// Close the pool and remove the backing container.
    pub async fn shutdown(self) -> Result<(), FixtureError> {
        self.pool.close().await;
        teardown(&self.handle.name).await?;
        Ok(())
    }
}
