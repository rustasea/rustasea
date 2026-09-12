//! End-to-end feature tests: HTTP route → isolated Postgres → response.
//!
//! Each test provisions its own container-backed database via
//! [`PostgresTestDb`](rustasea::testing::PostgresTestDb) and drives the real
//! `rustasea_router` dispatch path, so the full stack (router → ORM → Postgres
//! → HTTP response) is exercised against a throwaway database.

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rustasea::orm::{DbPool, Value};
use rustasea::router::Router;
use rustasea::testing::PostgresTestDb;
use tower::ServiceExt;

/// Seed the isolated database with a `users` table and one row.
async fn seed(pool: &DbPool) {
    pool.execute_script("CREATE TABLE users (id BIGINT PRIMARY KEY, name TEXT NOT NULL)")
        .await
        .expect("create users table");
    pool.execute_bind(
        "INSERT INTO users (id, name) VALUES ($1, $2)",
        &[Value::Int(1), Value::Text("Ada".into())],
    )
    .await
    .expect("insert user");
}

/// `GET /users/{id}` — read the row from Postgres and return it as JSON.
///
/// Returns `404` when the row does not exist.
async fn show_user(pool: &DbPool, id: i64) -> Response {
    let rows = pool
        .fetch_json("SELECT name FROM users WHERE id = $1", &[Value::Int(id)])
        .await
        .expect("select user");
    match rows.into_iter().next() {
        Some(row) => (StatusCode::OK, Json(row)).into_response(),
        None => (StatusCode::NOT_FOUND, "user not found").into_response(),
    }
}

/// Build the application router with the isolated pool captured per request.
fn build_app(pool: Arc<DbPool>) -> axum::Router {
    let mut router = Router::new();
    router.get_action(
        "/users/{id}",
        move |Path(params): Path<HashMap<String, String>>| {
            let pool = Arc::clone(&pool);
            async move {
                let id = params
                    .get("id")
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or(-1);
                show_user(&pool, id).await
            }
        },
    );
    router.into_axum_router()
}

/// Send one GET request through the router and decode the JSON body.
async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.expect("router dispatch");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

/// Positive: a stored row round-trips from Postgres through the route.
#[tokio::test]
#[ignore = "requires docker"]
async fn route_reads_row_from_isolated_database() {
    let db = PostgresTestDb::start("feature-route-read")
        .await
        .expect("start postgres");
    seed(db.pool()).await;

    let app = build_app(Arc::new(db.pool().clone()));
    let (status, body) = get_json(app, "/users/1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Ada");

    db.shutdown().await.expect("teardown");
}

/// Negative: a missing row still reaches Postgres and yields `404`.
#[tokio::test]
#[ignore = "requires docker"]
async fn route_returns_404_for_missing_row() {
    let db = PostgresTestDb::start("feature-route-missing")
        .await
        .expect("start postgres");
    seed(db.pool()).await;

    let app = build_app(Arc::new(db.pool().clone()));
    let (status, _) = get_json(app, "/users/999").await;

    assert_eq!(status, StatusCode::NOT_FOUND);

    db.shutdown().await.expect("teardown");
}
