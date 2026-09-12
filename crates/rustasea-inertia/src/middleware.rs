//! Axum middleware enforcing the Inertia version contract.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::{Inertia, InertiaRequest, InertiaResponse, VARY, X_INERTIA};

/// Axum middleware that enforces the Inertia request contract.
///
/// It parses [`InertiaRequest`] into the request extensions (so handlers can
/// extract it), short-circuits a stale `GET` with `409 Conflict` +
/// `X-Inertia-Location` before the controller runs, and guarantees
/// `Vary: X-Inertia` on the response for cache correctness.
///
/// Wire it with the shared service as router state:
///
/// ```rust,ignore
/// let inertia = Arc::new(Inertia::new("v1"));
/// let app = Router::new()
///     .route("/dashboard", get(dashboard))
///     .layer(axum::middleware::from_fn_with_state(
///         inertia.clone(),
///         rustasea_inertia::handle_inertia_requests,
///     ))
///     .with_state(inertia);
/// ```
pub async fn handle_inertia_requests(
    State(inertia): State<Arc<Inertia>>,
    request: Request,
    next: Next,
) -> Response {
    let parsed = InertiaRequest::from_parts(request.method(), request.uri(), request.headers());

    if parsed.version_mismatch(inertia.version()) {
        return InertiaResponse::version_conflict(&parsed.url).into_response();
    }

    let mut request = request;
    request.extensions_mut().insert(parsed);

    let mut response = next.run(request).await;
    append_vary_x_inertia(response.headers_mut());
    response
}

/// Append `X-Inertia` to a response's `Vary` header without duplicating it.
fn append_vary_x_inertia(headers: &mut HeaderMap) {
    match headers.get(VARY).and_then(|value| value.to_str().ok()) {
        Some(existing)
            if existing
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case(X_INERTIA)) => {}
        Some(existing) => {
            if let Ok(value) = HeaderValue::from_str(&format!("{existing}, {X_INERTIA}")) {
                headers.insert(VARY, value);
            }
        }
        None => {
            headers.insert(VARY, HeaderValue::from_static(X_INERTIA));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::routing::get;
    use axum::Router;
    use http::Request as HttpRequest;
    use tower::ServiceExt;

    /// Build a router whose handler records that it ran.
    fn app(inertia: Arc<Inertia>) -> Router {
        Router::new()
            .route(
                "/dashboard",
                get(|| async { (axum::http::StatusCode::OK, "controller ran") }),
            )
            .layer(axum::middleware::from_fn_with_state(
                inertia.clone(),
                handle_inertia_requests,
            ))
            .with_state(inertia)
    }

    /// A stale version short-circuits with `409` before the handler runs.
    #[tokio::test]
    async fn version_mismatch_short_circuits() {
        let inertia = Arc::new(Inertia::new("v2"));
        let request = HttpRequest::builder()
            .uri("/dashboard")
            .header("X-Inertia", "true")
            .header("X-Inertia-Version", "v1")
            .body(Body::empty())
            .expect("build request");

        let response = ServiceExt::oneshot(app(inertia), request)
            .await
            .expect("dispatch");

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
        assert_eq!(response.headers()[crate::X_INERTIA_LOCATION], "/dashboard");
    }

    /// A matching version passes through and the response carries `Vary`.
    #[tokio::test]
    async fn matching_version_passes_through_with_vary() {
        let inertia = Arc::new(Inertia::new("v2"));
        let request = HttpRequest::builder()
            .uri("/dashboard")
            .header("X-Inertia", "true")
            .header("X-Inertia-Version", "v2")
            .body(Body::empty())
            .expect("build request");

        let response = ServiceExt::oneshot(app(inertia), request)
            .await
            .expect("dispatch");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers()[VARY], "X-Inertia");
    }
}
