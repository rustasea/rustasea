//! Config loader — layered `config/*.toml` discovery + environment overlay.
//!
//! [`ConfigLoader::load`] auto-discovers every top-level `*.toml` file in the
//! `config/` directory and merges them in a deterministic order, then applies
//! an environment-variable overlay on top. A missing `config/` directory is
//! tolerated: the loader simply yields an environment-only configuration.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use config::{Config, ConfigError, File};
use serde::de::DeserializeOwned;

/// Base file name always applied first, as the lowest-precedence layer.
const BASE_FILE_NAME: &str = "app.toml";

/// Load and provide typed config.
pub struct ConfigLoader {
    inner: Config,
}

impl ConfigLoader {
    /// Load every `config/*.toml` (optional) + env overlay.
    ///
    /// Precedence, lowest to highest:
    /// 1. `config/app.toml` — the base layer.
    /// 2. Remaining `config/*.toml`, sorted by file path ascending.
    /// 3. Process environment variables (nested keys use the `__` separator).
    ///
    /// A missing or unreadable `config/` directory is not an error.
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from_dir("config")
    }

    /// Load every top-level `*.toml` in `dir` (optional) + env overlay.
    ///
    /// Applies the same precedence rules as [`ConfigLoader::load`], using `dir`
    /// in place of `config/`. Intended for custom config locations and tests.
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();
        let mut builder = Config::builder();
        for path in discover_toml_files(dir.as_ref()) {
            builder = builder.add_source(File::from(path).required(false));
        }
        builder = builder.add_source(config::Environment::default().separator("__"));
        Ok(Self {
            inner: builder.build()?,
        })
    }

    /// Load from explicit paths + env overlay.
    ///
    /// Each entry is a path without the `.toml` extension, matching
    /// [`config::File::with_name`]. Sources are applied in slice order, so
    /// later entries override earlier ones, and the env overlay wins last.
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

/// Discover top-level `*.toml` files in `dir`, base file first.
///
/// Returns an empty vector when `dir` is missing or unreadable. Remaining files
/// are sorted by path so the merge order is deterministic across platforms.
fn discover_toml_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_toml_extension(path))
        .collect();
    files.sort();

    // `app.toml` is the base layer and must load before all other files.
    if let Some(index) = files
        .iter()
        .position(|path| path.file_name() == Some(OsStr::new(BASE_FILE_NAME)))
    {
        let base = files.remove(index);
        files.insert(0, base);
    }
    files
}

/// Return true when `path` has a `.toml` extension (case-insensitive).
fn has_toml_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    /// Serializes tests that read or write process-global environment variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Unique temporary directory removed when dropped.
    struct TempConfigDir {
        path: PathBuf,
    }

    impl TempConfigDir {
        /// Create an empty temp directory with a process-unique name.
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustasea-config-{}-{}",
                std::process::id(),
                unique
            ));
            fs::create_dir_all(&path).expect("create temp config dir");
            Self { path }
        }

        /// Write `contents` to `name` inside the temp directory.
        fn write(&self, name: &str, contents: &str) {
            fs::write(self.path.join(name), contents).expect("write temp config file");
        }

        /// Borrow the temp directory path.
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempConfigDir {
        /// Remove the temp directory and all of its contents.
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn loads_and_merges_multiple_files() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempConfigDir::new();
        dir.write("app.toml", "rustasea_config_test_app_name = \"RustaSea\"\n");
        dir.write(
            "database.toml",
            "[rustasea_config_test_database]\nurl = \"postgres://localhost/db\"\n",
        );
        dir.write(
            "queue.toml",
            "[rustasea_config_test_queue]\ndriver = \"sync\"\n",
        );

        let loader = ConfigLoader::load_from_dir(dir.path()).expect("load config");

        let name: String = loader
            .get_key("rustasea_config_test_app_name")
            .expect("app_name");
        let url: String = loader
            .get_key("rustasea_config_test_database.url")
            .expect("database.url");
        let driver: String = loader
            .get_key("rustasea_config_test_queue.driver")
            .expect("queue.driver");
        assert_eq!(name, "RustaSea");
        assert_eq!(url, "postgres://localhost/db");
        assert_eq!(driver, "sync");
    }

    #[test]
    fn app_toml_is_base_and_later_files_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempConfigDir::new();
        dir.write(
            "app.toml",
            "rustasea_config_test_shared = \"app\"\nrustasea_config_test_app_only = \"base\"\n",
        );
        dir.write("aaa.toml", "rustasea_config_test_shared = \"aaa\"\n");
        dir.write("zzz.toml", "rustasea_config_test_shared = \"zzz\"\n");

        let loader = ConfigLoader::load_from_dir(dir.path()).expect("load config");

        let shared: String = loader
            .get_key("rustasea_config_test_shared")
            .expect("shared");
        let app_only: String = loader
            .get_key("rustasea_config_test_app_only")
            .expect("app_only");
        assert_eq!(shared, "zzz", "alphabetically last file must win");
        assert_eq!(app_only, "base", "app.toml base values must remain");
    }

    #[test]
    fn env_overlay_wins_over_files() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let key = "rustasea_config_test_env_override";
        std::env::set_var(key, "from-env");

        let dir = TempConfigDir::new();
        dir.write("app.toml", &format!("{key} = \"from-file\"\n"));

        let loader = ConfigLoader::load_from_dir(dir.path()).expect("load config");
        let value: String = loader.get_key(key).expect("env override key");

        std::env::remove_var(key);
        assert_eq!(value, "from-env");
    }

    #[test]
    fn missing_directory_is_tolerated() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempConfigDir::new();
        let missing = dir.path().join("does-not-exist");

        let loader = ConfigLoader::load_from_dir(&missing).expect("missing dir is tolerated");
        let value: Option<String> = loader.get_key("rustasea_config_test_app_name").ok();
        assert!(value.is_none());
    }

    #[test]
    fn load_from_explicit_paths_still_works() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = TempConfigDir::new();
        dir.write("app.toml", "rustasea_config_test_app_name = \"Explicit\"\n");

        let base = dir.path().join("app");
        let base = base.to_str().expect("utf-8 temp path");
        let loader = ConfigLoader::load_from(&[base]).expect("load_from");
        let name: String = loader
            .get_key("rustasea_config_test_app_name")
            .expect("app_name");
        assert_eq!(name, "Explicit");
    }
}
