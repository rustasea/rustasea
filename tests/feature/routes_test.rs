//! Feature tests — route-table smoke checks.
//!
//! Asserts the generated routers register the expected paths by dispatching
//! real requests through the compiled router: a registered path with an
//! unsupported method yields `405`, while an unregistered path yields `404`.
//! This exercises `routes::router` end to end without invoking the unfinished
//! scaffold handlers (method/path matching happens before a handler runs).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use example_app::routes;
use rustasea::http::AppState;
use tower::ServiceExt;

/// Build the full application router over a throwaway `AppState`.
fn app() -> axum::Router {
    routes::router(Arc::new(AppState::new("testing", true)))
}

/// Dispatch one request and return the response.
async fn response_for(method: &str, uri: &str) -> axum::response::Response {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("valid request");
    app().oneshot(request).await.expect("router dispatch")
}

/// Dispatch one request and return the response status.
async fn status_for(method: &str, uri: &str) -> StatusCode {
    response_for(method, uri).await.status()
}

/// Every generated path is registered (a wrong method ⇒ `405`, never `404`).
#[tokio::test]
async fn generated_routes_register_expected_paths() {
    for (method, uri) in [
        ("POST", "/"),
        ("POST", "/dashboard"),
        ("DELETE", "/login"),
        ("GET", "/logout"),
        ("DELETE", "/register"),
        ("DELETE", "/confirm-password"),
        ("DELETE", "/settings/profile"),
        ("DELETE", "/settings/password"),
        ("POST", "/settings/security"),
    ] {
        assert_eq!(
            status_for(method, uri).await,
            StatusCode::METHOD_NOT_ALLOWED,
            "{method} {uri} must be registered"
        );
    }
}

/// `/settings` redirects (302) to the profile screen.
#[tokio::test]
async fn settings_redirects_to_profile() {
    let response = response_for("GET", "/settings").await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/settings/profile")
    );
}

/// Unregistered paths fall through to `404`.
#[tokio::test]
async fn unknown_paths_are_not_found() {
    for uri in ["/does-not-exist", "/settings/unknown"] {
        assert_eq!(
            status_for("GET", uri).await,
            StatusCode::NOT_FOUND,
            "GET {uri} must not be registered"
        );
    }
}
