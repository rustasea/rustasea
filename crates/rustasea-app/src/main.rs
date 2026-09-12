//! RustaSea runnable app scaffold — served by `cargo run -p rustasea-app`.
//!
//! Mirrors the canonical Laravel-style layout described in `README.md`:
//! `crates/rustasea-app/src/bootstrap/app.rs` configures the [`Application`],
//! the workspace-root `config/*.toml` + `.env` provide typed settings, and
//! `routes/web.rs` owns the route table. This binary boots the framework,
//! serves real dispatch handlers over HTTP, and shuts down gracefully on
//! SIGINT/SIGTERM.

use std::net::SocketAddr;
use std::sync::Arc;

use rustasea::http::AppState;
use rustasea::router::Router as RouteTable;

mod bootstrap;
mod routes;

/// Default bind address for the dev server.
const DEFAULT_BIND: &str = "0.0.0.0:8000";

/// Program entry point: configure, boot, register routes, and serve.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Build + boot the foundation application (providers, bindings) from
    // `bootstrap/app.rs` — the README-mandated `Application::configure` home.
    let app = bootstrap::app::configure()?;

    // Declare the web route table with the framework DSL (`Router::new()`,
    // `.get(...)`). Until controller binding lands on rustasea::Router its
    // `into_axum_router` compiles stub handlers only, so the routes below
    // are mirrored by real dispatch handlers in `routes::web`.
    let mut route_table = RouteTable::new();
    route_table.get("/").get("/health").get("/welcome");
    println!("registered {} routes:", route_table.get_routes().len());
    for entry in route_table.get_routes() {
        println!("  {} {}", entry.method, entry.path);
    }

    // Compile real handlers into the axum router.
    let state = Arc::new(AppState::new("local", true));
    let router = routes::web::router(state);

    // Bind and serve with graceful shutdown.
    let addr: SocketAddr = bind_address();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("RustaSea dev server listening on http://{addr}");

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
        .unwrap_or_else(|| DEFAULT_BIND.parse().expect("static default bind address"))
}

/// Parse an `APP_URL` value into a `SocketAddr` when it carries a port.
fn parse_host_port(url: String) -> Option<SocketAddr> {
    let authority = url.split("://").nth(1)?;
    let host_port = authority.split('/').next()?;
    host_port.parse().ok()
}
