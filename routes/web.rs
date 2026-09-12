//! Web route handlers — real dispatch targets for `/`, `/health`, `/welcome`.
//!
//! Lives at the README-canonical `routes/web.rs`. `rustasea::Router` (M0) is
//! currently a registration DSL whose `into_axum_router` wires stub handlers;
//! until controller binding ships, the app keeps a single explicit handler
//! map here that mirrors the DSL route table registered in `src/main.rs`.
//!
//! The welcome page is rendered through `rustasea::view::MinijinjaEngine`
//! (the `view-runtime-templates` feature) rather than a raw `include_str!`, so
//! `resources/views/` can be edited without recompiling the binary.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use rustasea::http::AppState;
use rustasea::view::{MinijinjaEngine, ViewEngine};
use serde_json::json;

/// Runtime template engine rooted at `resources/views/`.
fn engine() -> MinijinjaEngine {
    MinijinjaEngine::from_default_root()
}

/// Build the axum router serving the app routes.
pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/health", axum::routing::get(health))
        .route("/welcome", axum::routing::get(index))
        .with_state(state)
}

/// GET / and /welcome — render the Laravel-style welcome page with data.
async fn index(State(_state): State<Arc<AppState>>) -> Response {
    match engine().render_value("welcome.html", &json!({ "app": "rustasea" })) {
        Ok(view) => view.into_response(),
        Err(error) => error.into_response(),
    }
}

/// Health check response body.
#[derive(serde::Serialize)]
struct Health {
    /// Liveness indicator.
    status: &'static str,
    /// Service name.
    service: &'static str,
    /// Current application environment.
    env: String,
}

/// GET /health — JSON liveness probe with HTTP status.
async fn health(State(state): State<Arc<AppState>>) -> Response {
    rustasea::http::JsonResponse::ok(Health {
        status: "ok",
        service: "rustasea",
        env: state.env.clone(),
    })
}
