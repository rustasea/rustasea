//! HTMX integration: partial-swap routing and request detection.
//!
//! The action endpoint always answers with a rendered fragment, never a full
//! page: HTMX swaps the fragment into the `hx-target` element. Full pages are
//! rendered by the application controller via [`crate::Livewire::render_page`].

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};

use crate::authorizer::Actor;
use crate::component::ActionRequest;
use crate::error::LivewireError;
use crate::realtime::sse_handler;
use crate::runtime::Livewire;

/// Request header HTMX sends on every partial request.
pub const HX_REQUEST_HEADER: &str = "HX-Request";

/// Whether `headers` mark an HTMX partial request (`HX-Request: true`).
///
/// The action endpoint returns a fragment regardless; application handlers use
/// this to choose between a full page and a fragment on shared GET routes.
pub fn is_htmx(headers: &HeaderMap) -> bool {
    headers
        .get(HX_REQUEST_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Mount the livewire HTMX action route and the SSE event route.
///
/// The caller supplies the runtime state:
/// ```rust,ignore
/// let app = rustasea_livewire::routes().with_state(livewire);
/// ```
pub fn routes() -> Router<Livewire> {
    Router::new()
        .route("/livewire/:component/actions/:action", post(action_handler))
        .route("/livewire/:component/events", get(sse_handler))
}

/// Handle `POST /livewire/:component/actions/:action` — authorize, run the
/// action, and return only the re-rendered HTML fragment for an HTMX swap.
pub(crate) async fn action_handler(
    State(livewire): State<Livewire>,
    Path((component, action)): Path<(String, String)>,
    actor: Option<Extension<Actor>>,
    Json(request): Json<ActionRequest>,
) -> Result<Response, LivewireError> {
    let actor = actor.map(|Extension(actor)| actor);
    let result = livewire
        .run_action(
            &component,
            &action,
            actor.as_ref(),
            request.state,
            &request.patch,
        )
        .await?;
    Ok(result.fragment.into_response())
}
