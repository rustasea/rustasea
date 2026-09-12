//! Testcontainers helpers — isolated Postgres/Redis on random ports.
//!
//! The M5 harness provisions one Postgres (and optionally one Redis) per test
//! via the real `testcontainers` crate: the image is started on a random host
//! port, the image's readiness signal is awaited (bounded by [`HEALTH_TIMEOUT`]),
//! and the live container is registered under a `rustasea-test-*` name so the
//! shared teardown ([`teardown_all`]) removes every container this binary
//! spawned (FR-507, US-M5-04).
//!
//! Feature `containers` (default) gates this module. The image is configurable
//! via `RUSTASEA_TEST_PG_IMAGE` / `RUSTASEA_TEST_REDIS_IMAGE` so CI can reuse
//! warm images.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use testcontainers::core::error::{TestcontainersError, WaitContainerError};
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

use crate::TestError;

/// Default Postgres image used by the harness.
pub const PG_IMAGE: &str = "postgres:16-alpine";
/// Default Redis image used by the harness.
pub const REDIS_IMAGE: &str = "redis:7-alpine";
/// Container health timeout (contract: 30s → `TestError::ContainerTimeout`).
pub const HEALTH_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors produced by container provisioning.
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    /// Container startup exceeded the 30s health timeout.
    #[error("container failed to become healthy within 30s")]
    ContainerTimeout,

    /// No free port could be allocated for the container.
    #[error("no free port available: {0}")]
    PortAllocation(String),

    /// The docker daemon is unavailable or rejected the request.
    #[error("docker error: {0}")]
    Docker(String),

    /// Container removal during teardown failed.
    #[error("teardown failed for container {name}: {detail}")]
    Teardown { name: String, detail: String },
}

impl From<ContainerError> for TestError {
    /// Lift a container error into the shared test error.
    fn from(e: ContainerError) -> Self {
        match e {
            ContainerError::ContainerTimeout => TestError::ContainerTimeout,
            other => TestError::Setup(other.to_string()),
        }
    }
}

/// A provisioned container handle.
#[derive(Debug, Clone)]
pub struct ContainerHandle {
    /// Container name (`rustasea-test-<binary>-<suffix>`).
    pub name: String,
    /// Host port the container's primary port maps to.
    pub port: u16,
    /// Connection string for the mapped port.
    pub url: String,
}

/// A live container tracked for shared teardown.
struct RegisteredContainer {
    /// Docker container name assigned at creation.
    name: String,
    /// Live container; removed when drained by a teardown call.
    container: ContainerAsync<GenericImage>,
}

/// Process-wide registry of containers spawned by this test binary.
static CONTAINERS: OnceLock<Mutex<Vec<RegisteredContainer>>> = OnceLock::new();

/// Borrow the process-wide container registry.
fn registry() -> &'static Mutex<Vec<RegisteredContainer>> {
    CONTAINERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a started container for shared teardown.
fn register(name: String, container: ContainerAsync<GenericImage>) {
    registry()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .push(RegisteredContainer { name, container });
}

/// Provision an isolated Postgres container on a random port.
///
/// Returns a ready [`ContainerHandle`] (image pulled on first use, readiness
/// awaited via the Postgres "ready to accept connections" log line) or
/// [`ContainerError::ContainerTimeout`] when the 30s startup budget is exceeded.
pub async fn postgres_container(suffix: &str) -> Result<ContainerHandle, ContainerError> {
    let image = std::env::var("RUSTASEA_TEST_PG_IMAGE").unwrap_or_else(|_| PG_IMAGE.into());
    let (image_name, image_tag) = split_image(&image);
    let name = container_name(suffix);

    let container = GenericImage::new(image_name, image_tag)
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_DB", "rustasea_test")
        .with_container_name(&name)
        .with_startup_timeout(HEALTH_TIMEOUT)
        .start()
        .await
        .map_err(map_container_error)?;

    let port = container
        .get_host_port_ipv4(5432u16)
        .await
        .map_err(map_container_error)?;
    register(name.clone(), container);

    Ok(ContainerHandle {
        name,
        port,
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/rustasea_test"),
    })
}

/// Provision an isolated Redis container on a random port.
///
/// Returns a ready [`ContainerHandle`] (readiness awaited via the Redis "Ready
/// to accept connections" log line) or [`ContainerError::ContainerTimeout`]
/// when the 30s startup budget is exceeded.
pub async fn redis_container(suffix: &str) -> Result<ContainerHandle, ContainerError> {
    let image = std::env::var("RUSTASEA_TEST_REDIS_IMAGE").unwrap_or_else(|_| REDIS_IMAGE.into());
    let (image_name, image_tag) = split_image(&image);
    let name = container_name(suffix);

    let container = GenericImage::new(image_name, image_tag)
        .with_exposed_port(ContainerPort::Tcp(6379))
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
        .with_container_name(&name)
        .with_startup_timeout(HEALTH_TIMEOUT)
        .start()
        .await
        .map_err(map_container_error)?;

    let port = container
        .get_host_port_ipv4(6379u16)
        .await
        .map_err(map_container_error)?;
    register(name.clone(), container);

    Ok(ContainerHandle {
        name,
        port,
        url: format!("redis://127.0.0.1:{port}"),
    })
}

/// Kill every container this binary spawned (teardown hook).
///
/// Drains the process-wide registry and force-removes each container. Removal
/// is best-effort: a failed removal is logged to stderr and the remaining
/// containers are still removed.
pub async fn teardown_all() {
    let containers: Vec<RegisteredContainer> = {
        let mut guard = registry()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        guard.drain(..).collect()
    };
    for registered in containers {
        if let Err(error) = registered.container.rm().await {
            eprintln!(
                "rustasea-testing: failed to remove container {}: {error}",
                registered.name
            );
        }
    }
}

/// Force-remove one registered container by name.
///
/// Returns `Ok(())` when the name is not registered (already torn down).
pub async fn teardown(name: &str) -> Result<(), ContainerError> {
    let registered = {
        let mut guard = registry()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        guard
            .iter()
            .position(|entry| entry.name == name)
            .map(|index| guard.remove(index))
    };
    match registered {
        Some(entry) => entry
            .container
            .rm()
            .await
            .map_err(|error| ContainerError::Teardown {
                name: name.to_string(),
                detail: error.to_string(),
            }),
        None => Ok(()),
    }
}

/// Build the deterministic container name for a test binary + suffix.
fn container_name(suffix: &str) -> String {
    format!("rustasea-test-{}-{suffix}", std::process::id())
}

/// Split `name:tag` on the tag separator, ignoring a registry port.
///
/// `postgres:16-alpine` → `("postgres", "16-alpine")`; a bare image defaults to
/// the `latest` tag; a registry port (`registry:5000/img`) is not a tag.
fn split_image(image: &str) -> (&str, &str) {
    let last_slash = image.rfind('/');
    match image.rfind(':') {
        Some(colon) if last_slash.is_none_or(|slash| colon > slash) => {
            (&image[..colon], &image[colon + 1..])
        }
        _ => (image, "latest"),
    }
}

/// Map a testcontainers failure onto the harness's typed error.
///
/// A readiness wait that exceeds the configured startup budget is a
/// [`ContainerError::ContainerTimeout`]; every other failure is surfaced as
/// [`ContainerError::Docker`] with the original message.
fn map_container_error(error: TestcontainersError) -> ContainerError {
    match error {
        TestcontainersError::WaitContainer(WaitContainerError::StartupTimeout) => {
            ContainerError::ContainerTimeout
        }
        other => ContainerError::Docker(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies a readiness startup timeout maps to the typed timeout error.
    #[test]
    fn startup_timeout_maps_to_container_timeout() {
        let error = map_container_error(TestcontainersError::WaitContainer(
            WaitContainerError::StartupTimeout,
        ));
        assert!(matches!(error, ContainerError::ContainerTimeout));
    }

    /// Verifies other testcontainers failures map to the docker variant.
    #[test]
    fn docker_failure_maps_to_docker_error() {
        let error = map_container_error(TestcontainersError::WaitContainer(
            WaitContainerError::Unhealthy,
        ));
        assert!(matches!(error, ContainerError::Docker(_)));
    }

    /// Verifies image strings split into name/tag without breaking registry ports.
    #[test]
    fn split_image_handles_tags_and_registry_ports() {
        assert_eq!(split_image("postgres:16-alpine"), ("postgres", "16-alpine"));
        assert_eq!(split_image("postgres"), ("postgres", "latest"));
        assert_eq!(
            split_image("registry:5000/postgres"),
            ("registry:5000/postgres", "latest")
        );
        assert_eq!(
            split_image("registry:5000/postgres:16"),
            ("registry:5000/postgres", "16")
        );
    }

    /// Verifies container names stay deterministic per process + suffix.
    #[test]
    fn container_name_includes_process_and_suffix() {
        let name = container_name("unit");
        assert!(name.starts_with("rustasea-test-"));
        assert!(name.ends_with("-unit"));
    }
}
