//! `TestCase` harness — per-binary setup, `.env.testing` overlay, teardown.
//!
//! A test struct implements [`TestCase`] and overrides [`TestCase::setup`] to
//! provision its isolated stores. The harness guarantees:
//!
//! - `.env.testing` is loaded once per process (process env wins over file).
//! - Factory sequences are reset before each `setup` so parallel tests never
//!   leak `Str` counters (NFR-Rel-03).
//! - Container-backed stores register their names for the shared teardown,
//!   which kills every `rustavel-test-*` container when `teardown_all` runs.
//!
//! Containers themselves are feature-gated (feature `containers`, default);
//! `setup` bodies that only touch in-memory stores compile without it.

use std::sync::OnceLock;

use crate::factory;

/// Environment configuration for a test run.
///
/// `env_testing_path` points at the package `.env.testing` file; the loader
/// overlays it under the current process environment (process env wins), then
/// exposes the merged values through typed accessors.
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Path to the `.env.testing` file (default `.env.testing`).
    pub env_testing_path: std::path::PathBuf,
}

impl Default for TestConfig {
    /// Default config: `.env.testing` in the current directory.
    fn default() -> Self {
        Self {
            env_testing_path: std::path::PathBuf::from(".env.testing"),
        }
    }
}

impl TestConfig {
    /// Load `.env.testing` into the process environment once.
    ///
    /// Existing process variables are never overwritten (process env > file).
    /// The load is intentionally idempotent per process — repeated `setup`
    /// calls in one binary must not re-apply the file.
    pub fn load_env_testing(&self) -> crate::Result<()> {
        static LOADED: OnceLock<()> = OnceLock::new();
        LOADED.get_or_init(|| {
            if self.env_testing_path.exists() {
                #[cfg(feature = "containers")]
                {
                    let _ = dotenvy::from_path(&self.env_testing_path);
                }
            }
        });
        Ok(())
    }
}

/// A runnable feature test.
///
/// Implementors provide `setup`; the harness calls `reset_factory_sequences`
/// before it so every test starts from sequence 1 (`user1@example.com`, not
/// `user11`).
pub trait TestCase: Send {
    /// Provision isolated stores for one test.
    ///
    /// The default implementation loads `.env.testing` and resets factory
    /// sequences — override and call `super::...` or `self.harness_setup()`
    /// first when spinning up containers.
    fn setup(&mut self) {
        self.harness_setup();
    }

    /// Run the shared harness pre-test hooks (env overlay + sequence reset).
    ///
    /// Overrides that provision containers should call this first, then spawn
    /// their stores on random ports.
    fn harness_setup(&mut self) {
        let config = TestConfig::default();
        let _ = config.load_env_testing();
        factory::reset_factory_sequences();
    }
}

/// Track container names spawned by this test binary for shared teardown.
pub(crate) fn register_container(name: String) {
    let registry = CONTAINERS.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    registry
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(name);
}

/// Process-wide container registry (`rustavel-test-*` names).
static CONTAINERS: OnceLock<std::sync::Mutex<Vec<String>>> = OnceLock::new();

/// Kill every container this binary spawned (teardown hook).
///
/// Best-effort: each container is stopped/removed individually and failures
/// are logged to stderr rather than aborting the remaining cleanup. Call from
/// a `#[tokio::test]` finalizer or the binary's `Drop`.
pub async fn teardown_all() {
    let names: Vec<String> = CONTAINERS
        .get()
        .map(|c| c.lock().unwrap_or_else(|p| p.into_inner()).clone())
        .unwrap_or_default();
    for name in names {
        if let Err(err) = teardown_container(&name).await {
            eprintln!("rustavel-testing: failed to remove container {name}: {err}");
        }
    }
}

/// Remove one named container via the docker CLI (best-effort).
async fn teardown_container(name: &str) -> crate::Result<()> {
    let status = tokio::process::Command::new("docker")
        .args(["rm", "-f", name])
        .status()
        .await
        .map_err(crate::TestError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(crate::TestError::Setup(format!(
            "docker rm -f {name} exited with {status}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies sequence reset runs before each setup.
    #[test]
    fn setup_resets_factory_sequences() {
        crate::factory::bump_sequence_for_test();
        assert!(crate::factory::current_sequence_for_test() > 0);

        let mut test = EmptyTest;
        test.setup();
        assert_eq!(crate::factory::current_sequence_for_test(), 0);
    }

    /// Minimal test double.
    struct EmptyTest;
    impl TestCase for EmptyTest {}
}
