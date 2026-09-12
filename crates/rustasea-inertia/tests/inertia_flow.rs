//! End-to-end flow tests: the full `X-Inertia*` contract through an axum router.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rustasea_inertia::axum;
use rustasea_inertia::{handle_inertia_requests, Inertia, InertiaRequest, Page};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Controller rendering a fixed dashboard page through the Inertia service.
async fn dashboard(State(inertia): State<Arc<Inertia>>, request: InertiaRequest) -> Response {
    inertia
        .render(
            "dashboard",
            json!({ "user": "Ada", "stats": 3, "secret": true }),
            &request,
        )
        .into_response()
}

/// Build a router with the Inertia middleware and shared service state.
fn app(inertia: Arc<Inertia>) -> Router {
    Router::new()
        .route("/dashboard", get(dashboard))
        .layer(axum::middleware::from_fn_with_state(
            inertia.clone(),
            handle_inertia_requests,
        ))
        .with_state(inertia)
}

/// Dispatch a `GET /dashboard` request with the given headers.
async fn dispatch(inertia: Arc<Inertia>, headers: &[(&str, &str)]) -> axum::response::Response {
    let mut builder = axum::http::Request::builder().uri("/dashboard");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = builder.body(Body::empty()).expect("build request");
    ServiceExt::oneshot(app(inertia), request)
        .await
        .expect("dispatch request")
}

/// Read a response body as a UTF-8 string.
async fn body_string(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    String::from_utf8(bytes.to_vec()).expect("utf-8 body")
}

/// An Inertia request receives a JSON page envelope with every prop.
#[tokio::test]
async fn inertia_request_receives_json_page() {
    let response = dispatch(
        Arc::new(Inertia::new("v1")),
        &[("X-Inertia", "true"), ("X-Inertia-Version", "v1")],
    )
    .await;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response.headers()[axum::http::header::CONTENT_TYPE],
        "application/json"
    );
    assert_eq!(response.headers()["X-Inertia"], "true");
    assert_eq!(response.headers()["Vary"], "X-Inertia");

    let body = body_string(response).await;
    let page: Page<Value> = serde_json::from_str(&body).expect("parse page");
    assert_eq!(page.component, "dashboard");
    assert_eq!(page.url, "/dashboard");
    assert_eq!(page.version, "v1");
    assert_eq!(page.props["user"], "Ada");
    assert_eq!(page.props["stats"], 3);
}

/// A partial reload filters the prop map on the wire.
#[tokio::test]
async fn partial_reload_filters_props_on_the_wire() {
    let response = dispatch(
        Arc::new(Inertia::new("v1")),
        &[
            ("X-Inertia", "true"),
            ("X-Inertia-Version", "v1"),
            ("X-Inertia-Partial-Component", "dashboard"),
            ("X-Inertia-Partial-Data", "user"),
        ],
    )
    .await;

    let body = body_string(response).await;
    let page: Page<Value> = serde_json::from_str(&body).expect("parse page");
    assert_eq!(page.props["user"], "Ada");
    assert!(page.props.get("stats").is_none());
    assert!(page.props.get("secret").is_none());
}

/// A first load without `X-Inertia` receives the HTML root document.
#[tokio::test]
async fn non_inertia_request_receives_html_shell() {
    let response = dispatch(Arc::new(Inertia::new("v1")), &[]).await;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response.headers()[axum::http::header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );
    assert_eq!(response.headers()["Vary"], "X-Inertia");

    let body = body_string(response).await;
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("data-page="));
    assert!(body.contains("&quot;component&quot;"));
}

/// A stale asset version yields `409` plus `X-Inertia-Location` and no body.
#[tokio::test]
async fn version_mismatch_returns_conflict_with_location() {
    let response = dispatch(
        Arc::new(Inertia::new("v2")),
        &[("X-Inertia", "true"), ("X-Inertia-Version", "v1")],
    )
    .await;

    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    assert_eq!(response.headers()["X-Inertia-Location"], "/dashboard");

    let body = body_string(response).await;
    assert!(body.is_empty());
}
