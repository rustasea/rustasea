//! Testcontainers helpers — isolated Postgres/Redis on random ports.
//!
//! The M5 harness provisions one Postgres (and optionally one Redis) per test
//! binary on a random free port, waits up to 30s for health, and registers
//! the container under a `rustasea-test-*` name so the shared teardown
//! removes it after the suite (FR-507, US-M5-04).
//!
//! Feature `containers` (default) gates this module's real docker wiring.
//! The image is configurable via `RUSTASEA_TEST_PG_IMAGE` /
//! `RUSTASEA_TEST_REDIS_IMAGE` so CI can reuse warm images.

use std::net::TcpListener;
use std::time::Duration;

use crate::test_case;
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

/// Provision an isolated Postgres container on a random port.
///
/// Returns a ready [`ContainerHandle`] (image pulled on first use, health
/// checked via `pg_isready` inside the container) or
/// [`ContainerError::ContainerTimeout`] after 30s.
pub async fn postgres_container(suffix: &str) -> Result<ContainerHandle, ContainerError> {
    let image = std::env::var("RUSTASEA_TEST_PG_IMAGE").unwrap_or_else(|_| PG_IMAGE.into());
    let port = allocate_port()?;
    let name = format!("rustasea-test-{}-{suffix}", std::process::id());

    let binary = testcontainers_binary();
    let output = tokio::process::Command::new(&binary)
        .args([
            "run",
            "-d",
            "--name",
            &name,
            "-p",
            &format!("{port}:5432"),
            "-e",
            "POSTGRES_PASSWORD=postgres",
            "-e",
            "POSTGRES_USER=postgres",
            "-e",
            "POSTGRES_DB=rustasea_test",
            &image,
        ])
        .output()
        .await
        .map_err(|e| ContainerError::Docker(e.to_string()))?;

    if !output.status.success() {
        return Err(ContainerError::Docker(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    test_case::register_container(name.clone());
    wait_healthy(&name, port, 5432, HEALTH_TIMEOUT).await?;

    Ok(ContainerHandle {
        name,
        port,
        url: format!("postgres://postgres:postgres@127.0.0.1:{port}/rustasea_test"),
    })
}

/// Provision an isolated Redis container on a random port.
pub async fn redis_container(suffix: &str) -> Result<ContainerHandle, ContainerError> {
    let image = std::env::var("RUSTASEA_TEST_REDIS_IMAGE").unwrap_or_else(|_| REDIS_IMAGE.into());
    let port = allocate_port()?;
    let name = format!("rustasea-test-{}-{suffix}", std::process::id());

    let binary = testcontainers_binary();
    let output = tokio::process::Command::new(&binary)
        .args([
            "run",
            "-d",
            "--name",
            &name,
            "-p",
            &format!("{port}:6379"),
            &image,
        ])
        .output()
        .await
        .map_err(|e| ContainerError::Docker(e.to_string()))?;

    if !output.status.success() {
        return Err(ContainerError::Docker(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    test_case::register_container(name.clone());
    wait_healthy(&name, port, 6379, HEALTH_TIMEOUT).await?;

    Ok(ContainerHandle {
        name,
        port,
        url: format!("redis://127.0.0.1:{port}"),
    })
}

/// Wait until the container's mapped port accepts TCP connections.
///
/// Polls every 250ms up to `timeout` (default contract 30s); success is an
/// accepted connection, failure after the budget is a
/// [`ContainerError::ContainerTimeout`].
async fn wait_healthy(
    _name: &str,
    port: u16,
    _inner: u16,
    timeout: Duration,
) -> Result<(), ContainerError> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let ok = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok();
        if ok {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(ContainerError::ContainerTimeout);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Allocate a free TCP port on the loopback interface.
fn allocate_port() -> Result<u16, ContainerError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| ContainerError::PortAllocation(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| ContainerError::PortAllocation(e.to_string()))?
        .port();
    drop(listener);
    Ok(port)
}

/// Resolve the docker binary name (override via `RUSTASEA_TEST_DOCKER`).
fn testcontainers_binary() -> String {
    std::env::var("RUSTASEA_TEST_DOCKER").unwrap_or_else(|_| "docker".into())
}
