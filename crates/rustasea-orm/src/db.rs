//! Real database connection foundation backed by `sqlx`.
//!
//! [`DbPool`] wraps one live `sqlx` pool per supported driver. Callers
//! [`DbPool::connect`] a URL, run [`DbPool::ping`] as a health check, and hand
//! the typed pool to the query executor. Only drivers compiled in are
//! constructible; the URL scheme selects the driver.

use crate::error::{OrmError, Result};
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
use std::time::Duration;

mod exec;

/// Maximum connections held by a pool (SQLite in-memory is capped at one).
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

/// Time to wait for a connection before `connect`/`acquire` fails.
#[cfg(any(feature = "sqlite", feature = "postgres", feature = "mysql"))]
const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);

/// A live connection pool for one supported database driver.
#[derive(Debug, Clone)]
pub enum DbPool {
    /// SQLite pool (file-backed or `:memory:`).
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
    /// PostgreSQL pool.
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
    /// MySQL pool.
    #[cfg(feature = "mysql")]
    MySql(sqlx::MySqlPool),
}

impl DbPool {
    /// Connect to `url`, selecting the driver from its scheme.
    ///
    /// `sqlite::memory:` yields a single-connection pool so the in-memory
    /// database is shared across statements. Unknown schemes and driver
    /// failures surface as [`OrmError::Pool`]; a recognised scheme whose driver
    /// feature is not compiled in surfaces as [`OrmError::UnsupportedDriver`].
    pub async fn connect(url: &str) -> Result<Self> {
        let scheme = url.split(':').next().unwrap_or_default();
        match scheme {
            "sqlite" => Self::connect_sqlite(url).await,
            "postgres" | "postgresql" => Self::connect_postgres(url).await,
            "mysql" => Self::connect_mysql(url).await,
            other => Err(OrmError::Pool(format!(
                "invalid database URL `{url}`: unknown scheme `{other}`"
            ))),
        }
    }

    /// The dialect name (`sqlite` / `postgres` / `mysql`).
    pub fn dialect(&self) -> &'static str {
        match self {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(_) => "sqlite",
            #[cfg(feature = "postgres")]
            DbPool::Postgres(_) => "postgres",
            #[cfg(feature = "mysql")]
            DbPool::MySql(_) => "mysql",
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => "unknown",
        }
    }

    /// Execute `SELECT 1` to verify the pool can reach the database.
    pub async fn ping(&self) -> Result<()> {
        match self {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => {}
        }
        Ok(())
    }

    /// Gracefully close every connection in the pool.
    pub async fn close(&self) {
        match self {
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => pool.close().await,
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => pool.close().await,
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => pool.close().await,
            #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
            _ => {}
        }
    }

    /// The inner SQLite pool, when this handle is SQLite.
    #[cfg(feature = "sqlite")]
    pub fn sqlite(&self) -> Option<&sqlx::SqlitePool> {
        match self {
            DbPool::Sqlite(pool) => Some(pool),
            #[cfg(feature = "postgres")]
            DbPool::Postgres(_) => None,
            #[cfg(feature = "mysql")]
            DbPool::MySql(_) => None,
        }
    }

    /// The inner Postgres pool, when this handle is Postgres.
    #[cfg(feature = "postgres")]
    pub fn postgres(&self) -> Option<&sqlx::PgPool> {
        match self {
            DbPool::Postgres(pool) => Some(pool),
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(_) => None,
            #[cfg(feature = "mysql")]
            DbPool::MySql(_) => None,
        }
    }

    /// The inner MySQL pool, when this handle is MySQL.
    #[cfg(feature = "mysql")]
    pub fn mysql(&self) -> Option<&sqlx::MySqlPool> {
        match self {
            DbPool::MySql(pool) => Some(pool),
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(_) => None,
            #[cfg(feature = "postgres")]
            DbPool::Postgres(_) => None,
        }
    }

    /// Connect a SQLite pool; in-memory URLs are pinned to one connection.
    #[cfg(feature = "sqlite")]
    async fn connect_sqlite(url: &str) -> Result<Self> {
        use std::str::FromStr;

        let options = sqlx::sqlite::SqliteConnectOptions::from_str(url)
            .map_err(|error| OrmError::Pool(error.to_string()))?
            .create_if_missing(true);

        let mut pool_options = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .acquire_timeout(DEFAULT_ACQUIRE_TIMEOUT);
        if sqlite_is_memory(url) {
            pool_options = pool_options
                .max_connections(1)
                .idle_timeout(None)
                .max_lifetime(None);
        }

        let pool = pool_options
            .connect_with(options)
            .await
            .map_err(|error| OrmError::Pool(error.to_string()))?;
        Ok(DbPool::Sqlite(pool))
    }

    /// Report that the SQLite driver is not compiled in.
    #[cfg(not(feature = "sqlite"))]
    async fn connect_sqlite(_url: &str) -> Result<Self> {
        Err(OrmError::UnsupportedDriver("sqlite".to_string()))
    }

    /// Connect a Postgres pool.
    #[cfg(feature = "postgres")]
    async fn connect_postgres(url: &str) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .acquire_timeout(DEFAULT_ACQUIRE_TIMEOUT)
            .connect(url)
            .await
            .map_err(|error| OrmError::Pool(error.to_string()))?;
        Ok(DbPool::Postgres(pool))
    }

    /// Report that the Postgres driver is not compiled in.
    #[cfg(not(feature = "postgres"))]
    async fn connect_postgres(_url: &str) -> Result<Self> {
        Err(OrmError::UnsupportedDriver("postgres".to_string()))
    }

    /// Connect a MySQL pool.
    #[cfg(feature = "mysql")]
    async fn connect_mysql(url: &str) -> Result<Self> {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .acquire_timeout(DEFAULT_ACQUIRE_TIMEOUT)
            .connect(url)
            .await
            .map_err(|error| OrmError::Pool(error.to_string()))?;
        Ok(DbPool::MySql(pool))
    }

    /// Report that the MySQL driver is not compiled in.
    #[cfg(not(feature = "mysql"))]
    async fn connect_mysql(_url: &str) -> Result<Self> {
        Err(OrmError::UnsupportedDriver("mysql".to_string()))
    }
}

/// Whether a SQLite URL addresses an in-memory database.
#[cfg(feature = "sqlite")]
fn sqlite_is_memory(url: &str) -> bool {
    url.contains(":memory:") || url.contains("mode=memory")
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;

    /// Verifies an in-memory SQLite pool connects and answers a ping.
    #[tokio::test]
    async fn connects_and_pings_sqlite_memory() {
        let pool = DbPool::connect("sqlite::memory:").await.expect("connect");
        assert_eq!(pool.dialect(), "sqlite");
        assert!(pool.sqlite().is_some());
        pool.ping().await.expect("ping");
        pool.close().await;
    }

    /// Verifies in-memory pools are pinned to a single shared connection.
    #[tokio::test]
    async fn sqlite_memory_pool_is_single_connection() {
        let pool = DbPool::connect("sqlite::memory:").await.unwrap();
        match &pool {
            DbPool::Sqlite(inner) => assert_eq!(inner.options().get_max_connections(), 1),
            #[cfg(feature = "postgres")]
            DbPool::Postgres(_) => {}
            #[cfg(feature = "mysql")]
            DbPool::MySql(_) => {}
        }
        pool.close().await;
    }

    /// Verifies an invalid URL maps to a typed pool error.
    #[tokio::test]
    async fn invalid_url_maps_to_pool_error() {
        let error = DbPool::connect("nonsense://localhost/none")
            .await
            .unwrap_err();
        assert!(matches!(error, OrmError::Pool(_)), "got {error:?}");
    }
}
