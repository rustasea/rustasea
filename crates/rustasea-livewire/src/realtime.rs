//! Realtime fan-out: SSE event streams and the WebSocket route helper.
//!
//! Components publish their re-rendered fragment to a
//! [`rustasea_broadcast::Channel`] after every successful action. Browsers can
//! consume those updates either over Server-Sent Events (this module) or over
//! the WebSocket endpoint provided by `rustasea-broadcast`.

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::MethodRouter;
use axum::Extension;
use futures_util::StreamExt;
use rustasea_broadcast::{event_stream_response, BroadcastError, SseEvent, WsMessage};
use tokio::sync::broadcast::Receiver;
use tokio_stream::wrappers::BroadcastStream;

use crate::authorizer::Actor;
use crate::error::LivewireError;
use crate::runtime::Livewire;

/// Render a broadcast receiver as an SSE HTTP response.
///
/// Each [`WsMessage`] becomes one `event:`/`data:` frame. A lagged receiver
/// surfaces as a stream error and closes the response rather than silently
/// dropping updates, so the client can reconnect and resynchronize.
pub fn sse_from_receiver(rx: Receiver<WsMessage>) -> Response {
    let stream = BroadcastStream::new(rx).map(|item| match item {
        Ok(message) => Ok(SseEvent::new(message.event, message.data)),
        Err(error) => Err(BroadcastError::Transport(error.to_string())),
    });
    event_stream_response(stream)
}

/// Handle `GET /livewire/:component/events` by streaming the component's
/// broadcast channel as SSE.
///
/// The subscription is authorized by the broadcast hub using the identity from
/// the request extension, so private/presence channels reject anonymous
/// connections with `403` before any frame is produced.
pub(crate) async fn sse_handler(
    State(livewire): State<Livewire>,
    Path(component): Path<String>,
    actor: Option<Extension<Actor>>,
) -> Result<Response, LivewireError> {
    let identity = actor.map(|Extension(actor)| actor.id).unwrap_or_default();
    let channel = livewire
        .component(&component)
        .ok_or_else(|| LivewireError::ComponentNotFound {
            name: component.clone(),
        })?
        .channel()
        .auth_channel();
    let receiver = livewire
        .hub()
        .subscribe(&identity, &channel)
        .await
        .map_err(|source| match source {
            // A rejected subscription is an authorization failure, not a
            // server fault: surface it as `403`, never `500`.
            BroadcastError::Unauthorized { .. }
            | BroadcastError::Unauthenticated { .. }
            | BroadcastError::Forbidden { .. } => LivewireError::Unauthorized {
                component: component.clone(),
                action: "events".to_string(),
            },
            source => LivewireError::Broadcast {
                component: component.clone(),
                source,
            },
        })?;
    Ok(sse_from_receiver(receiver))
}

/// WebSocket route for `rustasea-broadcast`, using [`Livewire`] as router state.
///
/// Mount it alongside [`crate::routes`] to serve the WS fan-out:
/// ```rust,ignore
/// let app = Router::new()
///     .merge(rustasea_livewire::routes())
///     .route("/broadcasting/auth", rustasea_livewire::ws_route())
///     .with_state(livewire);
/// ```
pub fn ws_route() -> MethodRouter<Livewire> {
    rustasea_broadcast::ws_route::<Livewire>()
}
