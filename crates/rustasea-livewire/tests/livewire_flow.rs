//! End-to-end tests for the livewire runtime over the axum router.
//!
//! Covers the required positive path (an authorized action returns the
//! re-rendered fragment, not a full page) and negative path (an unauthorized
//! action is rejected before any state is touched), plus the realtime fan-out
//! and SSE surface.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use rustasea_broadcast::{async_trait, AuthDecision, Authorize, BroadcastHub, Channel};
use rustasea_livewire::{routes, ActionAuthorizer, Actor, Component, Livewire};
use rustasea_view::AskamaEngine;
use serde::Deserialize;
use tower::ServiceExt;

/// Askama fragment template for the counter component.
#[derive(askama::Template, Deserialize)]
#[template(path = "counter.html")]
struct CounterFragment {
    /// Counter value interpolated into the fragment.
    count: u32,
}

/// Askama full-page template for the counter component.
#[derive(askama::Template, Deserialize)]
#[template(path = "counter_page.html")]
struct CounterPage {
    /// Counter value interpolated into the page.
    count: u32,
}

/// Action gate allowing only `user-1` to run `counter.increment`.
struct OwnerOnly;

impl ActionAuthorizer for OwnerOnly {
    fn is_authorized(&self, actor: Option<&Actor>, component: &str, action: &str) -> bool {
        actor.map(|actor| actor.id.as_str()) == Some("user-1")
            && component == "counter"
            && action == "increment"
    }
}

/// Broadcast channel gate that allows every subscription (public channels).
struct AllowGate;

#[async_trait]
impl Authorize for AllowGate {
    async fn authorize(&self, _channel: &str, _identity: &str) -> AuthDecision {
        AuthDecision::Allow
    }
}

/// Broadcast channel gate that denies every subscription (private channels).
struct DenyGate;

#[async_trait]
impl Authorize for DenyGate {
    async fn authorize(&self, _channel: &str, _identity: &str) -> AuthDecision {
        AuthDecision::Deny("no".to_string())
    }
}

/// Build a runtime with the counter component registered.
fn build_livewire() -> Livewire {
    let mut engine = AskamaEngine::new();
    engine.register::<CounterFragment>("counter");
    engine.register::<CounterPage>("counter_page");

    let mut livewire = Livewire::new(
        Arc::new(engine),
        Arc::new(OwnerOnly),
        BroadcastHub::new(AllowGate),
    );

    let counter = Component::new("counter", "counter")
        .with_page_template("counter_page")
        .action("increment", |state| {
            let count = state
                .get("count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            state.set("count", serde_json::json!(count + 1));
            Ok(())
        });
    livewire.register(counter);
    livewire
}

/// Build a POST action request, optionally attaching an authenticated actor.
fn action_request(actor: Option<&str>, body: &'static str) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri("/livewire/counter/actions/increment")
        .header("HX-Request", "true")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("build request");
    if let Some(id) = actor {
        request.extensions_mut().insert(Actor::new(id));
    }
    request
}

/// Read a response body as UTF-8 text.
async fn body_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    String::from_utf8(bytes.to_vec()).expect("utf-8 body")
}

/// Positive: an authorized action returns the fragment (not a full page).
#[tokio::test]
async fn partial_swap_returns_the_fragment() {
    let app = routes().with_state(build_livewire());
    let response = app
        .oneshot(action_request(Some("user-1"), r#"{"state":{"count":0}}"#))
        .await
        .expect("dispatch request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type"),
        "text/html; charset=utf-8"
    );

    let body = body_text(response).await;
    assert!(body.contains("count: 1"), "fragment body was: {body}");
    assert!(
        !body.contains("<html"),
        "partial swap must not return a full page"
    );
    assert!(
        !body.contains("<!DOCTYPE"),
        "partial swap must not return a full page"
    );
}

/// Positive: a request patch is applied before the action runs.
#[tokio::test]
async fn request_patch_is_applied_before_the_action() {
    let app = routes().with_state(build_livewire());
    let response = app
        .oneshot(action_request(
            Some("user-1"),
            r#"{"state":{"count":5},"patch":{"count":10}}"#,
        ))
        .await
        .expect("dispatch request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("count: 11"), "patched body was: {body}");
}

/// Negative: an actor the gate rejects is refused with `403`.
#[tokio::test]
async fn unauthorized_action_is_rejected() {
    let app = routes().with_state(build_livewire());
    let response = app
        .oneshot(action_request(Some("user-2"), r#"{"state":{"count":0}}"#))
        .await
        .expect("dispatch request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = body_text(response).await;
    assert!(
        !body.contains("count"),
        "denied action must not render a fragment: {body}"
    );
}

/// Negative: an anonymous action is refused with `403`.
#[tokio::test]
async fn anonymous_action_is_rejected() {
    let app = routes().with_state(build_livewire());
    let response = app
        .oneshot(action_request(None, r#"{"state":{"count":0}}"#))
        .await
        .expect("dispatch request");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Realtime: a successful action fans the fragment out to subscribers.
#[tokio::test]
async fn action_publishes_fragment_to_subscribers() {
    let livewire = build_livewire();
    let mut receiver = livewire
        .hub()
        .subscribe("", "counter")
        .await
        .expect("subscribe to public channel");

    let app = routes().with_state(livewire);
    let response = app
        .oneshot(action_request(Some("user-1"), r#"{"state":{"count":0}}"#))
        .await
        .expect("dispatch request");
    assert_eq!(response.status(), StatusCode::OK);

    let message = receiver.recv().await.expect("broadcast delivered");
    assert_eq!(message.event, "increment");
    assert_eq!(message.channel, "counter");
    assert!(message.data.contains("count: 1"));
}

/// Realtime: the events endpoint streams `text/event-stream`.
#[tokio::test]
async fn events_endpoint_streams_sse() {
    let app = routes().with_state(build_livewire());
    let request = Request::builder()
        .method("GET")
        .uri("/livewire/counter/events")
        .body(Body::empty())
        .expect("build request");

    let response = app.oneshot(request).await.expect("dispatch request");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type"),
        "text/event-stream"
    );
}

/// Negative: an anonymous SSE subscription to a private channel is forbidden.
#[tokio::test]
async fn anonymous_sse_subscription_to_private_channel_is_forbidden() {
    let mut engine = AskamaEngine::new();
    engine.register::<CounterFragment>("counter");

    let mut livewire = Livewire::new(
        Arc::new(engine),
        Arc::new(OwnerOnly),
        BroadcastHub::new(DenyGate),
    );
    livewire.register(
        Component::new("counter", "counter").on_channel(Channel::Private("counter".to_string())),
    );

    let app = routes().with_state(livewire);
    let request = Request::builder()
        .method("GET")
        .uri("/livewire/counter/events")
        .body(Body::empty())
        .expect("build request");

    let response = app.oneshot(request).await.expect("dispatch request");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Full-page rendering uses the component's page template.
#[tokio::test]
async fn render_page_uses_the_page_template() {
    let livewire = build_livewire();
    let page = livewire
        .render_page("counter", &serde_json::json!({ "count": 3 }))
        .expect("render page");

    assert!(page.body().contains("<!DOCTYPE html>"));
    assert!(page.body().contains("count: 3"));
}
