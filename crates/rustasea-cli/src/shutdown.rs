//! Graceful shutdown for long-running workers (`queue:work`, `schedule:run`).
//!
//! [`Shutdownable`] mirrors the M4 foundation shutdown contract: a worker
//! registers a drain future and `wait_for_shutdown` resolves when SIGTERM/
//! SIGINT arrives. The CLI shim then joins the worker with the configured
//! `shutdown_timeout`, exiting 0 on a clean drain (FR-504, US-M5-01).

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// A worker that can drain its in-flight work before exiting.
///
/// Implementors return a future that completes once all in-flight tasks are
/// acknowledged or released. The trait is boxed at registration time, so
/// callers never name the underlying future type.
pub trait Shutdownable: Send + Sync {
    /// Signal the worker to stop accepting new work.
    fn request_shutdown(&self);

    /// Future resolving when in-flight work has drained.
    fn drained(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Wait for SIGTERM/SIGINT (unix) or ctrl-c (other platforms).
///
/// Returns immediately on platforms without signal support after a short
/// settle, mirroring the foundation `shutdown::graceful` behaviour.
pub async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).ok();
        let mut int = signal(SignalKind::interrupt()).ok();
        tokio::select! {
            _ = async { if let Some(s) = term.as_mut() { s.recv().await; } } => {}
            _ = async { if let Some(s) = int.as_mut() { s.recv().await; } } => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Join a shutdownable worker with a drain timeout.
///
/// Requests shutdown, waits for the drain future, then applies `timeout`: a
/// clean drain returns `Ok(())`; exceeding the timeout returns
/// [`crate::CliError::ShutdownTimeout`].
pub async fn drain_with_timeout(
    worker: &dyn Shutdownable,
    timeout: Duration,
) -> crate::error::CliResult<()> {
    worker.request_shutdown();
    let drained = worker.drained();
    match tokio::time::timeout(timeout, drained).await {
        Ok(()) => Ok(()),
        Err(_) => Err(crate::CliError::ShutdownTimeout(timeout)),
    }
}
