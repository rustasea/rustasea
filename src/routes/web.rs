//! Web route handlers — real dispatch targets for `/`, `/health`, `/welcome`.
//!
//! Lives at the README-canonical `routes/web.rs`. `rustavel::Router` (M0) is
//! currently a registration DSL whose `into_axum_router` wires stub handlers;
//! until controller binding ships, the app keeps a single explicit handler
//! map here that mirrors the DSL route table registered in `src/main.rs`.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{Html, Response};

use rustavel::http::AppState;

/// Welcome page markup, rendered from `resources/views/welcome.html`.
const WELCOME_HTML: &str = include_str!("../../resources/views/welcome.html");

/// Build the axum router serving the app routes.
pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/health", axum::routing::get(health))
        .route("/welcome", axum::routing::get(index))
        .with_state(state)
}

/// GET / and /welcome — serve the Laravel-style welcome page.
async fn index(State(_state): State<Arc<AppState>>) -> Html<&'static str> {
    Html(WELCOME_HTML)
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
    rustavel::http::JsonResponse::ok(Health {
        status: "ok",
        service: "rustavel-app",
        env: state.env.clone(),
    })
}
