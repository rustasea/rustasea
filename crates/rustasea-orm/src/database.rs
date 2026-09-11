//! Database handle, configuration, and foundation container binding.
//!
//! [`Database`] is the ergonomic facade over a real [`DbPool`]; it can be
//! created from a DSN or from `config/database.toml` (with `DATABASE_URL`
//! overriding the file URL). [`DatabaseServiceProvider`] binds the handle into
//! the foundation container so application code resolves it by key.

use serde::{Deserialize, Serialize};

use rustasea_foundation::{Application, ServiceProvider};

use crate::error::{OrmError, Result};
use crate::types::Value;

mod codec;
mod pool;
mod row;

pub use pool::DbPool;
pub use row::DbRow;

/// Container key under which the [`Database`] handle is bound.
pub const DATABASE_BINDING: &str = "database";

/// Connection pool sizing sourced from `config.database.pool`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of idle connections kept warm.
    #[serde(default = "default_pool_min")]
    pub min: u32,
    /// Maximum number of connections the pool will open.
    #[serde(default = "default_pool_max")]
    pub max: u32,
    /// Seconds before an idle connection is reaped.
    #[serde(default = "default_pool_idle_timeout")]
    pub idle_timeout: u64,
}

impl Default for PoolConfig {
    /// Default pool sizing (`min=1`, `max=10`, `idle_timeout=600`).
    fn default() -> Self {
        Self {
            min: default_pool_min(),
            max: default_pool_max(),
            idle_timeout: default_pool_idle_timeout(),
        }
    }
}

impl PoolConfig {
    /// Validate pool bounds before a pool is constructed.
    pub fn validate(&self) -> Result<()> {
        if self.max == 0 {
            return Err(OrmError::Configuration(
                "database.pool.max must be greater than zero".into(),
            ));
        }
        if self.min > self.max {
            return Err(OrmError::Configuration(format!(
                "database.pool.min ({}) must not exceed database.pool.max ({})",
                self.min, self.max
            )));
        }
        Ok(())
    }
}

/// Default minimum pool connections.
fn default_pool_min() -> u32 {
    1
}

/// Default maximum pool connections.
fn default_pool_max() -> u32 {
    10
}

/// Default idle timeout in seconds.
fn default_pool_idle_timeout() -> u64 {
    600
}

/// Typed view of the `[database]` table in `config/database.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Optional driver name (`postgres`, `mysql`, `sqlite`); inferred from the DSN when omitted.
    #[serde(default)]
    pub driver: Option<String>,
    /// Connection string; overridden by the `DATABASE_URL` environment variable.
    #[serde(default)]
    pub url: Option<String>,
    /// Connection pool sizing.
    #[serde(default)]
    pub pool: PoolConfig,
}

impl DatabaseConfig {
    /// Parse a configuration from an inline TOML document containing `[database]`.
    pub fn from_toml_str(toml: &str) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from_str(toml, config::FileFormat::Toml))
            .build()
            .map_err(|error| OrmError::Configuration(error.to_string()))?;
        config
            .get::<DatabaseConfig>("database")
            .map_err(|error| OrmError::Configuration(error.to_string()))
    }

    /// Parse a configuration from a TOML file path (extension optional).
    pub fn from_file(path: &str) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(path).required(true))
            .build()
            .map_err(|error| OrmError::Configuration(error.to_string()))?;
        config
            .get::<DatabaseConfig>("database")
            .map_err(|error| OrmError::Configuration(error.to_string()))
    }

    /// Load `config/database.toml` (optional) with a `DATABASE_URL` override.
    ///
    /// Missing files are tolerated: the resulting config simply has no URL and
    /// [`DatabaseConfig::dsn`] reports a typed configuration error.
    pub fn load() -> Result<Self> {
        let mut parsed = config::Config::builder()
            .add_source(config::File::with_name("config/database").required(false))
            .add_source(config::Environment::default().separator("__"))
            .build()
            .map_err(|error| OrmError::Configuration(error.to_string()))?
            .get::<DatabaseConfig>("database")
            .unwrap_or_default();
        if let Ok(url) = std::env::var("DATABASE_URL") {
            if !url.trim().is_empty() {
                parsed.url = Some(url);
            }
        }
        Ok(parsed)
    }

    /// The configured DSN, or a typed error when none is set.
    pub fn dsn(&self) -> Result<&str> {
        self.url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| {
                OrmError::Configuration(
                    "no database DSN configured (set database.url or DATABASE_URL)".into(),
                )
            })
    }

    /// Validate pool bounds and that any explicit driver matches the DSN scheme.
    pub fn validate(&self) -> Result<()> {
        self.pool.validate()?;
        if let (Some(driver), Some(url)) = (self.driver.as_deref(), self.url.as_deref()) {
            let configured = normalize_driver(driver)?;
            let from_dsn = pool::dsn_driver_name(url)?;
            if configured != from_dsn {
                return Err(OrmError::Configuration(format!(
                    "database.driver `{driver}` does not match DSN scheme `{from_dsn}`"
                )));
            }
        }
        Ok(())
    }
}

/// Normalize a configured driver alias to its canonical name.
fn normalize_driver(driver: &str) -> Result<&'static str> {
    match driver.trim().to_ascii_lowercase().as_str() {
        "postgres" | "postgresql" => Ok("postgres"),
        "mysql" | "mariadb" => Ok("mysql"),
        "sqlite" => Ok("sqlite"),
        other => Err(OrmError::UnsupportedDriver(other.to_string())),
    }
}

/// Ergonomic handle over a real [`DbPool`].
#[derive(Debug, Clone)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    /// Create a lazily-connected handle with default pool sizing.
    pub fn connect_lazy(dsn: &str) -> Result<Self> {
        Self::connect_lazy_with(dsn, &PoolConfig::default())
    }

    /// Create a lazily-connected handle with explicit pool sizing.
    pub fn connect_lazy_with(dsn: &str, config: &PoolConfig) -> Result<Self> {
        Ok(Self {
            pool: DbPool::connect_lazy_with(dsn, config)?,
        })
    }

    /// Eagerly connect a handle with default pool sizing.
    pub async fn connect(dsn: &str) -> Result<Self> {
        Ok(Self {
            pool: DbPool::connect(dsn).await?,
        })
    }

    /// Eagerly connect a handle with explicit pool sizing.
    pub async fn connect_with(dsn: &str, config: &PoolConfig) -> Result<Self> {
        Ok(Self {
            pool: DbPool::connect_with(dsn, config).await?,
        })
    }

    /// Build a handle from `config/database.toml` + `DATABASE_URL` override.
    pub fn from_config() -> Result<Self> {
        let config = DatabaseConfig::load()?;
        Self::from_config_with(&config)
    }

    /// Build a handle from an already-resolved [`DatabaseConfig`].
    pub fn from_config_with(config: &DatabaseConfig) -> Result<Self> {
        config.validate()?;
        Self::connect_lazy_with(config.dsn()?, &config.pool)
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Consume the handle and return the pool.
    pub fn into_pool(self) -> DbPool {
        self.pool
    }

    /// The active driver name (`postgres`, `mysql`, or `sqlite`).
    pub fn driver(&self) -> &'static str {
        self.pool.driver()
    }

    /// Whether the pool has been closed.
    pub fn is_closed(&self) -> bool {
        self.pool.is_closed()
    }

    /// Current number of live connections.
    pub fn size(&self) -> u32 {
        self.pool.size()
    }

    /// Execute a statement and return the number of affected rows.
    pub async fn execute(&self, sql: &str, bindings: &[Value]) -> Result<u64> {
        self.pool.execute(sql, bindings).await
    }

    /// Fetch every row produced by a statement.
    pub async fn fetch_all(&self, sql: &str, bindings: &[Value]) -> Result<Vec<DbRow>> {
        self.pool.fetch_all(sql, bindings).await
    }

    /// Fetch exactly one row, mapping an empty result to [`OrmError::NotFound`].
    pub async fn fetch_one(&self, sql: &str, bindings: &[Value]) -> Result<DbRow> {
        self.pool.fetch_one(sql, bindings).await
    }

    /// Fetch zero or one row.
    pub async fn fetch_optional(&self, sql: &str, bindings: &[Value]) -> Result<Option<DbRow>> {
        self.pool.fetch_optional(sql, bindings).await
    }

    /// Liveness probe used by the health check endpoint.
    pub async fn ping(&self) -> Result<()> {
        self.pool.ping().await
    }

    /// Close the pool, draining all idle connections.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Bind a [`Database`] handle into the foundation container.
pub fn bind_database(app: &mut Application, database: Database) {
    app.container.instance(DATABASE_BINDING, database);
}

/// Foundation provider that binds a [`Database`] from `config/database.toml`.
///
/// Registration is best-effort: when no DSN is configured the binding is
/// skipped so the container resolves `None` instead of failing the boot DAG.
pub struct DatabaseServiceProvider;

impl ServiceProvider for DatabaseServiceProvider {
    /// Bind a lazily-connected database when configuration is available.
    fn register(&self, app: &mut Application) {
        if let Ok(database) = Database::from_config() {
            bind_database(app, database);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canonical `config/database.toml` shape used across the tests.
    fn sample_toml() -> &'static str {
        r#"
[database]
driver = "postgres"
url = "postgres://user:pass@localhost:5432/rustasea"

[database.pool]
min = 1
max = 5
idle_timeout = 30
"#
    }

    /// Verifies the TOML table maps onto the typed config.
    #[test]
    fn parses_database_config_from_toml() {
        let config = DatabaseConfig::from_toml_str(sample_toml()).unwrap();
        assert_eq!(config.driver.as_deref(), Some("postgres"));
        assert_eq!(
            config.url.as_deref(),
            Some("postgres://user:pass@localhost:5432/rustasea")
        );
        assert_eq!(config.pool.min, 1);
        assert_eq!(config.pool.max, 5);
        assert_eq!(config.pool.idle_timeout, 30);
        config.validate().unwrap();
    }

    /// Verifies invalid pool bounds are rejected with a typed error.
    #[test]
    fn rejects_pool_bounds_with_min_above_max() {
        let config = DatabaseConfig {
            pool: PoolConfig {
                min: 5,
                max: 2,
                idle_timeout: 30,
            },
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(OrmError::Configuration(_))));
    }

    /// Verifies a zero maximum is rejected.
    #[test]
    fn rejects_zero_max_connections() {
        let config = DatabaseConfig {
            pool: PoolConfig {
                min: 0,
                max: 0,
                idle_timeout: 30,
            },
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(OrmError::Configuration(_))));
    }

    /// Verifies a missing DSN surfaces a typed configuration error.
    #[test]
    fn rejects_missing_dsn() {
        let config = DatabaseConfig::default();
        assert!(matches!(config.dsn(), Err(OrmError::Configuration(_))));
    }

    /// Verifies a driver/DSN scheme mismatch is rejected.
    #[test]
    fn rejects_driver_dsn_mismatch() {
        let config = DatabaseConfig {
            driver: Some("postgres".into()),
            url: Some("mysql://localhost/rustasea".into()),
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(OrmError::Configuration(_))));
    }

    /// Verifies an unknown driver alias is rejected.
    #[test]
    fn rejects_unknown_driver_alias() {
        let config = DatabaseConfig {
            driver: Some("oracle".into()),
            url: Some("oracle://localhost/db".into()),
            ..Default::default()
        };
        assert!(matches!(
            config.validate(),
            Err(OrmError::UnsupportedDriver(_))
        ));
    }

    /// Verifies lazy construction selects the Postgres driver without connecting.
    ///
    /// `sqlx` lazily spawns pool maintenance, which needs an active Tokio
    /// runtime even though no connection is opened here.
    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn connect_lazy_selects_postgres_driver() {
        let database =
            Database::connect_lazy("postgres://user:pass@localhost:5432/rustasea").unwrap();
        assert_eq!(database.driver(), "postgres");
        assert!(!database.is_closed());
    }

    /// Verifies an unrecognized DSN scheme is rejected.
    #[test]
    fn connect_lazy_rejects_unknown_scheme() {
        let error = Database::connect_lazy("oracle://localhost/db").unwrap_err();
        assert!(matches!(error, OrmError::UnsupportedDriver(_)));
    }

    /// Verifies a DSN whose driver feature is off is rejected.
    #[cfg(not(feature = "sqlite"))]
    #[test]
    fn connect_lazy_rejects_disabled_driver() {
        let error = Database::connect_lazy("sqlite::memory:").unwrap_err();
        assert!(matches!(error, OrmError::UnsupportedDriver(_)));
    }

    /// Verifies a bound database resolves back out of the foundation container.
    ///
    /// Runs under Tokio because binding a lazily-connected pool requires a
    /// runtime, mirroring the lazy-construction test above.
    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn container_resolves_bound_database() {
        let mut app = Application::new();
        let database =
            Database::connect_lazy("postgres://user:pass@localhost:5432/rustasea").unwrap();
        bind_database(&mut app, database);
        let resolved = app.container.get::<Database>(DATABASE_BINDING);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().driver(), "postgres");
    }

    /// Verifies an unbound database resolves to `None`, not a panic.
    #[test]
    fn container_missing_binding_is_none() {
        let app = Application::new();
        assert!(app.container.get::<Database>(DATABASE_BINDING).is_none());
    }
}
