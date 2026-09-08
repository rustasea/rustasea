//! Config loader — layered config/*.toml + env overlay.

use config::{Config, ConfigError, File};
use serde::de::DeserializeOwned;

/// Load and provide typed config.
pub struct ConfigLoader {
    inner: Config,
}

impl ConfigLoader {
    /// Load from config/*.toml (optional) + env overlay.
    pub fn load() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();
        let builder = Config::builder()
            .add_source(File::with_name("config/app").required(false))
            .add_source(config::Environment::default().separator("__"));
        let inner = builder.build()?;
        Ok(Self { inner })
    }

    /// Load from explicit paths + env overlay.
    pub fn load_from(files: &[&str]) -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();
        let mut builder = Config::builder();
        for f in files {
            builder = builder.add_source(File::with_name(f).required(false));
        }
        builder = builder.add_source(config::Environment::default().separator("__"));
        Ok(Self {
            inner: builder.build()?,
        })
    }

    /// Deserialize entire config into T.
    pub fn get<T: DeserializeOwned>(&self) -> Result<T, ConfigError> {
        self.inner.clone().try_deserialize()
    }

    /// Get a single key as T.
    pub fn get_key<T: DeserializeOwned>(&self, key: &str) -> Result<T, ConfigError> {
        self.inner.get::<T>(key)
    }

    /// Access underlying config.
    pub fn inner(&self) -> &Config {
        &self.inner
    }
}
