//! Real `sqlx` connection pools behind a dialect-neutral facade.
//!
//! [`DbPool`] wraps whichever driver was compiled in (Postgres, MySQL, SQLite)
//! and exposes the runtime query API only — no compile-time-checked macros, so
//! the workspace builds without a live `DATABASE_URL`. Rows are decoded into
//! the dialect-neutral [`super::DbRow`] so callers never depend on a concrete
//! driver.

use std::time::Duration;

use crate::error::{OrmError, Result};
use crate::types::Value;

use super::row::DbRow;
use super::PoolConfig;

#[cfg(feature = "mysql")]
use super::codec::{bind_mysql, decode_mysql_row};
#[cfg(feature = "postgres")]
use super::codec::{bind_pg, decode_pg_row};
#[cfg(feature = "sqlite")]
use super::codec::{bind_sqlite, decode_sqlite_row};

/// A database driver understood by [`DbPool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Driver {
    /// PostgreSQL (`postgres://`, `postgresql://`).
    Postgres,
    /// MySQL / MariaDB (`mysql://`, `mariadb://`).
    MySql,
    /// SQLite (`sqlite:` — file or `:memory:`).
    Sqlite,
}

/// A live connection pool for the compiled driver.
///
/// Variants exist only when the matching cargo feature is enabled, so
/// `--all-features` builds all three while a single-driver build stays lean.
#[derive(Debug, Clone)]
pub enum DbPool {
    /// PostgreSQL pool (`PgPool`).
    #[cfg(feature = "postgres")]
    Postgres(sqlx::PgPool),
    /// MySQL pool (`MySqlPool`).
    #[cfg(feature = "mysql")]
    MySql(sqlx::MySqlPool),
    /// SQLite pool (`SqlitePool`).
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::SqlitePool),
}

impl DbPool {
    /// Create a lazily-connected pool with default pool sizing.
    ///
    /// No connection is opened until the first query, so this never blocks on
    /// network I/O and is safe to call while binding into the container.
    pub fn connect_lazy(dsn: &str) -> Result<Self> {
        Self::connect_lazy_with(dsn, &PoolConfig::default())
    }

    /// Create a lazily-connected pool with explicit sizing.
    ///
    /// Returns [`OrmError::UnsupportedDriver`] when the DSN scheme has no
    /// compiled driver, or [`OrmError::Configuration`] for invalid pool bounds.
    pub fn connect_lazy_with(dsn: &str, config: &PoolConfig) -> Result<Self> {
        config.validate()?;
        match parse_driver(dsn)? {
            Driver::Postgres => connect_pg_lazy(dsn, config),
            Driver::MySql => connect_mysql_lazy(dsn, config),
            Driver::Sqlite => connect_sqlite_lazy(dsn, config),
        }
    }

    /// Eagerly connect a pool with default sizing.
    pub async fn connect(dsn: &str) -> Result<Self> {
        Self::connect_with(dsn, &PoolConfig::default()).await
    }

    /// Eagerly connect a pool with explicit sizing.
    pub async fn connect_with(dsn: &str, config: &PoolConfig) -> Result<Self> {
        config.validate()?;
        match parse_driver(dsn)? {
            Driver::Postgres => connect_pg(dsn, config).await,
            Driver::MySql => connect_mysql(dsn, config).await,
            Driver::Sqlite => connect_sqlite(dsn, config).await,
        }
    }

    /// The active driver name (`postgres`, `mysql`, or `sqlite`).
    pub fn driver(&self) -> &'static str {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(_) => "postgres",
            #[cfg(feature = "mysql")]
            DbPool::MySql(_) => "mysql",
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(_) => "sqlite",
        }
    }

    /// Whether the pool has been closed.
    pub fn is_closed(&self) -> bool {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => pool.is_closed(),
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => pool.is_closed(),
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => pool.is_closed(),
        }
    }

    /// Current number of live connections.
    pub fn size(&self) -> u32 {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => pool.size(),
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => pool.size(),
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => pool.size(),
        }
    }

    /// Execute a statement and return the number of affected rows.
    pub async fn execute(&self, sql: &str, bindings: &[Value]) -> Result<u64> {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let query = bind_pg(sqlx::query(sql), bindings)?;
                Ok(query.execute(pool).await?.rows_affected())
            }
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => {
                let query = bind_mysql(sqlx::query(sql), bindings)?;
                Ok(query.execute(pool).await?.rows_affected())
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let query = bind_sqlite(sqlx::query(sql), bindings)?;
                Ok(query.execute(pool).await?.rows_affected())
            }
        }
    }

    /// Fetch every row produced by a statement.
    pub async fn fetch_all(&self, sql: &str, bindings: &[Value]) -> Result<Vec<DbRow>> {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let rows = bind_pg(sqlx::query(sql), bindings)?.fetch_all(pool).await?;
                Ok(rows.iter().map(decode_pg_row).collect())
            }
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => {
                let rows = bind_mysql(sqlx::query(sql), bindings)?
                    .fetch_all(pool)
                    .await?;
                Ok(rows.iter().map(decode_mysql_row).collect())
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let rows = bind_sqlite(sqlx::query(sql), bindings)?
                    .fetch_all(pool)
                    .await?;
                Ok(rows.iter().map(decode_sqlite_row).collect())
            }
        }
    }

    /// Fetch exactly one row, mapping an empty result to [`OrmError::NotFound`].
    pub async fn fetch_one(&self, sql: &str, bindings: &[Value]) -> Result<DbRow> {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = bind_pg(sqlx::query(sql), bindings)?.fetch_one(pool).await?;
                Ok(decode_pg_row(&row))
            }
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => {
                let row = bind_mysql(sqlx::query(sql), bindings)?
                    .fetch_one(pool)
                    .await?;
                Ok(decode_mysql_row(&row))
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = bind_sqlite(sqlx::query(sql), bindings)?
                    .fetch_one(pool)
                    .await?;
                Ok(decode_sqlite_row(&row))
            }
        }
    }

    /// Fetch zero or one row.
    pub async fn fetch_optional(&self, sql: &str, bindings: &[Value]) -> Result<Option<DbRow>> {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => {
                let row = bind_pg(sqlx::query(sql), bindings)?
                    .fetch_optional(pool)
                    .await?;
                Ok(row.as_ref().map(decode_pg_row))
            }
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => {
                let row = bind_mysql(sqlx::query(sql), bindings)?
                    .fetch_optional(pool)
                    .await?;
                Ok(row.as_ref().map(decode_mysql_row))
            }
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => {
                let row = bind_sqlite(sqlx::query(sql), bindings)?
                    .fetch_optional(pool)
                    .await?;
                Ok(row.as_ref().map(decode_sqlite_row))
            }
        }
    }

    /// Liveness probe used by the health check endpoint.
    pub async fn ping(&self) -> Result<()> {
        self.execute("SELECT 1", &[]).await.map(|_| ())
    }

    /// Close the pool, draining all idle connections.
    pub async fn close(&self) {
        match self {
            #[cfg(feature = "postgres")]
            DbPool::Postgres(pool) => pool.close().await,
            #[cfg(feature = "mysql")]
            DbPool::MySql(pool) => pool.close().await,
            #[cfg(feature = "sqlite")]
            DbPool::Sqlite(pool) => pool.close().await,
        }
    }
}

/// Resolve the driver name for a DSN without opening a connection.
///
/// Shared with [`super::DatabaseConfig`] so the configured `driver` can be
/// checked against the DSN scheme before any pool is built.
pub(crate) fn dsn_driver_name(dsn: &str) -> Result<&'static str> {
    Ok(match parse_driver(dsn)? {
        Driver::Postgres => "postgres",
        Driver::MySql => "mysql",
        Driver::Sqlite => "sqlite",
    })
}

/// Classify a DSN by its URI scheme.
fn parse_driver(dsn: &str) -> Result<Driver> {
    let trimmed = dsn.trim();
    if trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://") {
        return Ok(Driver::Postgres);
    }
    if trimmed.starts_with("mysql://") || trimmed.starts_with("mariadb://") {
        return Ok(Driver::MySql);
    }
    if trimmed.starts_with("sqlite:") {
        return Ok(Driver::Sqlite);
    }
    Err(OrmError::UnsupportedDriver(format!(
        "unrecognized database DSN scheme: `{trimmed}`"
    )))
}

/// PostgreSQL: connect lazily with the configured pool sizing.
#[cfg(feature = "postgres")]
fn connect_pg_lazy(dsn: &str, config: &PoolConfig) -> Result<DbPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(config.min)
        .max_connections(config.max)
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect_lazy(dsn)?;
    Ok(DbPool::Postgres(pool))
}

/// PostgreSQL: report the driver as unavailable when the feature is off.
#[cfg(not(feature = "postgres"))]
fn connect_pg_lazy(_dsn: &str, _config: &PoolConfig) -> Result<DbPool> {
    Err(OrmError::UnsupportedDriver(
        "postgres driver feature is not enabled".into(),
    ))
}

/// PostgreSQL: connect eagerly with the configured pool sizing.
#[cfg(feature = "postgres")]
async fn connect_pg(dsn: &str, config: &PoolConfig) -> Result<DbPool> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(config.min)
        .max_connections(config.max)
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect(dsn)
        .await?;
    Ok(DbPool::Postgres(pool))
}

/// PostgreSQL: report the driver as unavailable when the feature is off.
#[cfg(not(feature = "postgres"))]
async fn connect_pg(_dsn: &str, _config: &PoolConfig) -> Result<DbPool> {
    Err(OrmError::UnsupportedDriver(
        "postgres driver feature is not enabled".into(),
    ))
}

/// MySQL: connect lazily with the configured pool sizing.
#[cfg(feature = "mysql")]
fn connect_mysql_lazy(dsn: &str, config: &PoolConfig) -> Result<DbPool> {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .min_connections(config.min)
        .max_connections(config.max)
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect_lazy(dsn)?;
    Ok(DbPool::MySql(pool))
}

/// MySQL: report the driver as unavailable when the feature is off.
#[cfg(not(feature = "mysql"))]
fn connect_mysql_lazy(_dsn: &str, _config: &PoolConfig) -> Result<DbPool> {
    Err(OrmError::UnsupportedDriver(
        "mysql driver feature is not enabled".into(),
    ))
}

/// MySQL: connect eagerly with the configured pool sizing.
#[cfg(feature = "mysql")]
async fn connect_mysql(dsn: &str, config: &PoolConfig) -> Result<DbPool> {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .min_connections(config.min)
        .max_connections(config.max)
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect(dsn)
        .await?;
    Ok(DbPool::MySql(pool))
}

/// MySQL: report the driver as unavailable when the feature is off.
#[cfg(not(feature = "mysql"))]
async fn connect_mysql(_dsn: &str, _config: &PoolConfig) -> Result<DbPool> {
    Err(OrmError::UnsupportedDriver(
        "mysql driver feature is not enabled".into(),
    ))
}

/// SQLite: connect lazily with the configured pool sizing.
#[cfg(feature = "sqlite")]
fn connect_sqlite_lazy(dsn: &str, config: &PoolConfig) -> Result<DbPool> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .min_connections(config.min)
        .max_connections(config.max)
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect_lazy(dsn)?;
    Ok(DbPool::Sqlite(pool))
}

/// SQLite: report the driver as unavailable when the feature is off.
#[cfg(not(feature = "sqlite"))]
fn connect_sqlite_lazy(_dsn: &str, _config: &PoolConfig) -> Result<DbPool> {
    Err(OrmError::UnsupportedDriver(
        "sqlite driver feature is not enabled".into(),
    ))
}

/// SQLite: connect eagerly with the configured pool sizing.
#[cfg(feature = "sqlite")]
async fn connect_sqlite(dsn: &str, config: &PoolConfig) -> Result<DbPool> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .min_connections(config.min)
        .max_connections(config.max)
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .connect(dsn)
        .await?;
    Ok(DbPool::Sqlite(pool))
}

/// SQLite: report the driver as unavailable when the feature is off.
#[cfg(not(feature = "sqlite"))]
async fn connect_sqlite(_dsn: &str, _config: &PoolConfig) -> Result<DbPool> {
    Err(OrmError::UnsupportedDriver(
        "sqlite driver feature is not enabled".into(),
    ))
}
