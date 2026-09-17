//! ExampleApp HTTP entry point.

use std::net::SocketAddr;
use std::sync::Arc;

use example_app::{bootstrap, routes};
use rustasea::http::AppState;

/// Default bind address for the dev server (Laravel `php artisan serve` parity).
const DEFAULT_BIND: &str = "0.0.0.0:8000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the container and run the provider boot DAG. `configure`
    // returns `Result` so a misconfigured boot aborts before the server starts
    // instead of silently ignoring the failure.
    let app = bootstrap::app::configure()?;

    // Create the shared HTTP state once; every route table receives this same
    // instance so handlers resolve one application state, not a disconnected one.
    let state = Arc::new(AppState::new("local", true));

    // Build the axum router from the generated route tables.
    let router = routes::router(state);

    // Resolve the bind address from `APP_URL` (host:port) so the advertised URL
    // and the listening socket stay in sync; fall back to the default.
    let addr: SocketAddr = bind_address();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("ExampleApp listening on http://{addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(app.shutdown())
        .await?;
    Ok(())
}

/// Resolve the bind address from `APP_URL` (host:port) or the default.
fn bind_address() -> SocketAddr {
    std::env::var("APP_URL")
        .ok()
        .and_then(parse_host_port)
        .unwrap_or_else(default_bind)
}

/// The fallback bind address used when `APP_URL` is unset or carries no port.
fn default_bind() -> SocketAddr {
    match DEFAULT_BIND.parse() {
        Ok(addr) => addr,
        // `DEFAULT_BIND` is a compile-time constant and always parses; this arm
        // keeps the function panic-free without an `expect`.
        Err(_) => SocketAddr::from(([0, 0, 0, 0], 8000)),
    }
}

/// Parse an `APP_URL` value into a `SocketAddr` when it carries a port.
fn parse_host_port(url: String) -> Option<SocketAddr> {
    let authority = url.split("://").nth(1)?;
    let host_port = authority.split('/').next()?;
    host_port.parse().ok()
}
