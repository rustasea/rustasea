//! ExampleApp HTTP entry point.

use std::sync::Arc;

use example_app::{bootstrap, routes};
use rustasea::http::AppState;

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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("ExampleApp listening on http://0.0.0.0:3000");

    axum::serve(listener, router)
        .with_graceful_shutdown(app.shutdown())
        .await?;
    Ok(())
}
