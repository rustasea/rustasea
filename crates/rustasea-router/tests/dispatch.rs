//! Router dispatch acceptance tests (GAP-002).
//!
//! Proves that `into_axum_router` dispatches to real controller actions rather
//! than the stub handler: path/query/body extraction, `#[route]` metadata
//! consumption, resource resolution, and axum's 404/405 semantics.

use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rustasea_router::Router;
use tower::ServiceExt;

/// Controller whose methods are the real dispatch targets under test.
struct UserController;

impl UserController {
    /// GET /users — list handler.
    async fn index() -> Response {
        (StatusCode::OK, "users-index").into_response()
    }

    /// GET /users/{id} — path-parameter handler.
    async fn show(Path(params): Path<HashMap<String, String>>) -> Response {
        let id = params.get("id").cloned().unwrap_or_default();
        (StatusCode::OK, format!("user:{id}")).into_response()
    }

    /// GET /users?q=... — query-parameter handler.
    async fn search(Query(query): Query<HashMap<String, String>>) -> Response {
        let q = query.get("q").cloned().unwrap_or_default();
        (StatusCode::OK, format!("search:{q}")).into_response()
    }

    /// POST /users — JSON body handler.
    async fn store(Json(body): Json<serde_json::Value>) -> Response {
        let name = body
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("?");
        (StatusCode::OK, format!("stored:{name}")).into_response()
    }
}

/// Drive one request through the compiled router and read status + body.
async fn call(app: axum::Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.oneshot(request).await.expect("router dispatch");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// GET /home — non-domain catch-all dispatch target.
async fn catch_all_home() -> Response {
    (StatusCode::OK, "catch-all-home").into_response()
}

/// GET /home — domain-constrained dispatch target.
async fn domain_home() -> Response {
    (StatusCode::OK, "domain-home").into_response()
}

/// Build a plain GET request for `uri`.
fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

/// Positive: a bound GET action runs and path parameters are extracted.
#[tokio::test]
async fn get_action_dispatches_and_extracts_path_params() {
    let mut router = Router::new();
    router.get_action("/users", UserController::index);
    router.get_action("/users/{id}", UserController::show);
    let app = router.into_axum_router();

    let request = Request::builder()
        .uri("/users/42")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "user:42");
}

/// Multi-segment handler extracting every path parameter.
async fn team_member(Path(params): Path<HashMap<String, String>>) -> Response {
    let team = params.get("team").cloned().unwrap_or_default();
    let member = params.get("member").cloned().unwrap_or_default();
    (StatusCode::OK, format!("team:{team}/member:{member}")).into_response()
}

/// Positive: multi-segment and typed (`{param:field}`) placeholders both
/// translate to Axum `:param` and extract every path parameter.
#[tokio::test]
async fn multi_and_typed_path_params_dispatch() {
    let mut router = Router::new();
    router.get_action("/teams/{team}/members/{member:slug}", team_member);
    let app = router.into_axum_router();

    let request = Request::builder()
        .uri("/teams/7/members/ada")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "team:7/member:ada");
}

/// Positive: query parameters reach the controller through `Query`.
#[tokio::test]
async fn get_action_extracts_query_params() {
    let mut router = Router::new();
    router.get_action("/search", UserController::search);
    let app = router.into_axum_router();

    let request = Request::builder()
        .uri("/search?q=rust")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "search:rust");
}

/// Positive: a JSON body reaches the controller through `Json`.
#[tokio::test]
async fn post_action_extracts_json_body() {
    let mut router = Router::new();
    router.post_action("/users", UserController::store);
    let app = router.into_axum_router();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"name":"ada"}"#))
        .unwrap();
    let (status, body) = call(app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "stored:ada");
}

/// Negative: a path registered for GET returns 405 for a wrong method.
#[tokio::test]
async fn wrong_method_returns_405() {
    let mut router = Router::new();
    router.get_action("/users", UserController::index);
    let app = router.into_axum_router();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(app, request).await;

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

/// Positive: a lowercase method verb still binds the concrete GET router, so
/// the same path rejects other verbs with 405 instead of matching `any`.
#[tokio::test]
async fn lowercase_action_method_normalizes_and_preserves_405() {
    let mut router = Router::new();
    router.action("get", "/lower", UserController::index);
    let app = router.into_axum_router();

    let (status, body) = call(app.clone(), get("/lower")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "users-index");

    let post = Request::builder()
        .method("POST")
        .uri("/lower")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(app, post).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

/// Negative: an unregistered path returns 404.
#[tokio::test]
async fn unknown_route_returns_404() {
    let mut router = Router::new();
    router.get_action("/users", UserController::index);
    let app = router.into_axum_router();

    let request = Request::builder()
        .uri("/missing")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(app, request).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Positive: resource routes resolve registered controller actions by name.
#[tokio::test]
async fn resource_resolves_registered_controller_actions() {
    let mut router = Router::new();
    router
        .controller("UserController")
        .controller_action("index", UserController::index)
        .controller_action("show", UserController::show);
    router.resource("users", "UserController");
    let app = router.into_axum_router();

    let list = Request::builder()
        .uri("/users")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(app.clone(), list).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "users-index");

    let show = Request::builder()
        .uri("/users/9")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(app, show).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "user:9");
}

/// Positive: `#[route]` metadata consts are consumed at registration.
#[tokio::test]
async fn route_meta_consumes_attribute_metadata() {
    let mut router = Router::new();
    router.route_meta(
        meta_controller::__RUSTASEA_ROUTE_show,
        meta_controller::show,
    );
    let app = router.into_axum_router();

    let request = Request::builder()
        .uri("/meta/7")
        .body(Body::empty())
        .unwrap();
    let (status, body) = call(app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "meta:7");
}

/// Positive: a domain route registered after a catch-all on the same
/// method+path still dispatches its own action, not the catch-all's.
///
/// `get_routes` sorts domain entries first while the bound-action registry
/// stays in registration order; matching must therefore be by method+path+
/// domain so the sorted domain entry claims its own action.
#[tokio::test]
async fn domain_route_dispatches_own_action_when_catch_all_registered_first() {
    // Baseline: the catch-all handler dispatches when no domain route shadows it.
    let mut baseline = Router::new();
    baseline.get_action("/home", catch_all_home);
    let (status, body) = call(baseline.into_axum_router(), get("/home")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "catch-all-home");

    // Catch-all registered before the domain route on the same method+path.
    let mut router = Router::new();
    router.get_action("/home", catch_all_home);
    router
        .domain("api.example.com")
        .get_action("/home", domain_home);
    let app = router.into_axum_router();

    let (status, body) = call(app, get("/home")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "domain-home");
}

/// Negative: an unbound route never consumes an action bound to another route.
///
/// The unbound route wins the method+path slot (it is registered first), so it
/// must resolve to the stub handler rather than steal the bound action.
#[tokio::test]
async fn unbound_route_does_not_steal_bound_action() {
    let mut router = Router::new();
    router.get("/home");
    router.get_action("/home", catch_all_home);
    let app = router.into_axum_router();

    let (status, body) = call(app, get("/home")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

/// `route:list` data exposes the resolved handler label + binding fields.
#[test]
fn route_entry_exposes_handler_and_binding_fields() {
    let mut router = Router::new();
    router.get_action("/users/{id}", UserController::show);

    let routes = router.get_routes();
    let route = &routes[0];
    assert_eq!(route.binding_fields, vec!["id".to_string()]);
    assert!(
        route
            .handler
            .as_deref()
            .unwrap_or_default()
            .contains("show"),
        "expected handler label to name the bound action, got {:?}",
        route.handler
    );
}

/// Controller with a `#[route]`-annotated handler used for metadata tests.
mod meta_controller {
    use std::collections::HashMap;

    use axum::extract::Path;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use rustasea_macros::route;

    /// GET /meta/{id} — metadata-driven dispatch target.
    #[route(method = "GET", path = "/meta/{id}")]
    pub async fn show(Path(params): Path<HashMap<String, String>>) -> Response {
        let id = params.get("id").cloned().unwrap_or_default();
        (StatusCode::OK, format!("meta:{id}")).into_response()
    }
}
