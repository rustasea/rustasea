//! HTTP client policy checks — timeout + throw semantics (FR-107).
//!
//! Spins up a local axum listener and drives `HttpClient` against it,
//! asserting the `throw` predicate maps a matching status to
//! `HttpError::Status` and that satisfied predicates pass through.

use std::net::SocketAddr;
use std::time::Duration;

use axum::routing::get;
use axum::Router;
use rustasea_http::{HttpClient, HttpError};

/// Boot a one-route axum app on an ephemeral port.
async fn spawn_app(status_code: u16) -> SocketAddr {
    let app = Router::new().route(
        "/probe",
        get(move || async move { axum::http::StatusCode::from_u16(status_code).unwrap() }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("listener address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("axum serve");
    });
    addr
}

/// A `throw` predicate matching the upstream status yields HttpError::Status.
#[tokio::test]
async fn throw_maps_matching_status_to_error() {
    let addr = spawn_app(500).await;
    let client = HttpClient::new()
        .timeout(Duration::from_secs(5))
        .throw(|resp| resp.status().is_server_error());
    let err = client
        .get(&format!("http://{addr}/probe"))
        .await
        .expect_err("500 must throw");
    match err {
        HttpError::Status { code } => assert_eq!(code, 500),
        other => panic!("expected HttpError::Status, got {other:?}"),
    }
}

/// A `throw` predicate that does not match lets the response through.
#[tokio::test]
async fn non_matching_throw_returns_response() {
    let addr = spawn_app(422).await;
    let client = HttpClient::new()
        .timeout(Duration::from_secs(5))
        .throw(|resp| resp.status().is_server_error());
    let resp = client
        .get(&format!("http://{addr}/probe"))
        .await
        .expect("422 must not throw for is_server_error predicate");
    assert_eq!(resp.status().as_u16(), 422);
}
