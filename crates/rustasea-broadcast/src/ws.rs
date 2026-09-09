//! WebSocket transport: handshake identity resolution, subscribe session.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::BroadcastError;
use crate::hub::{BroadcastHub, WsMessage, WS_BUFFER};

/// Configuration for the WebSocket broadcast.
///
/// The limits are applied to the upgraded socket (audit S5 S2): every session
/// bounds inbound frames at [`WebSocketConfig::max_frame_size`] and inbound
/// messages at the same ceiling (`max_message_size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct WebSocketConfig {
    /// Max inbound frame size in bytes.
    pub max_frame_size: usize,
    /// Max outbound message buffer (Lagged backpressure cap).
    pub buffer: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_frame_size: 64 * 1024,
            buffer: WS_BUFFER,
        }
    }
}

/// Frame the wire message as a text WebSocket message.
pub fn ws_frame(message: &WsMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| "{}".to_string())
}

/// Client subscribe frame sent over the socket.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscribeFrame {
    /// `subscribe` or `prompt`.
    #[serde(default)]
    pub event: String,
    /// Channel name (wire form, e.g. `private-chat.1`).
    pub channel: Option<String>,
    /// Agent prompt payload (`event: prompt`).
    pub data: Option<String>,
}

/// Server ack sent once a subscription is live (`event: subscribed`).
#[derive(Debug, Clone, Serialize)]
pub struct WsAck {
    /// Event name.
    pub event: &'static str,
    /// Channel that was subscribed, JSON-escaped by serde.
    pub channel: String,
}

impl WsAck {
    /// Build the `subscribed` ack for a channel.
    pub fn subscribed(channel: impl Into<String>) -> Self {
        Self {
            event: "subscribed",
            channel: channel.into(),
        }
    }
}

/// Per-connection identity and the WebSocket upgrade to run it on.
///
/// Produced from the handshake before the socket is upgraded; the identity is
/// resolved from that connection's own bearer credential and is never shared
/// across sockets (audit S5 B1).
pub struct WsConnection {
    /// Subscriber identity resolved from the handshake.
    pub identity: String,
    /// Upgrade to finalize.
    ws: WebSocketUpgrade,
    /// Inbound frame/message size ceiling for the upgraded socket.
    max_frame_size: usize,
    /// Outbound fan-out buffer capacity for the subscribe session.
    buffer: usize,
}

impl WsConnection {
    /// Upgrade the socket and run the session with this connection's identity.
    ///
    /// The configured frame/message ceiling is applied to the upgraded socket
    /// via axum's `WebSocketUpgrade` builder — the transport rejects oversized
    /// inbound frames instead of buffering them unboundedly (audit S5 S2).
    pub fn on_upgrade(self, hub: BroadcastHub) -> Response {
        let max_frame_size = self.max_frame_size;
        let buffer = self.buffer;
        self.ws
            .max_frame_size(max_frame_size)
            .max_message_size(max_frame_size)
            .on_upgrade(move |socket| {
                let hub = hub.with_buffer(buffer);
                session(socket, hub, self.identity)
            })
    }
}

/// Extract a connection identity (bearer token) and upgrade together.
///
/// Runs the standard `WebSocketUpgrade` handshake validation on the same
/// request, then resolves the per-connection bearer credential from the
/// `Authorization` header or `?token=` query before the socket is upgraded.
#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for WsConnection
where
    S: Send + Sync,
{
    type Rejection = axum::extract::ws::rejection::WebSocketUpgradeRejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        // The upgrade extractor consumes the handshake from the request
        // extensions, so it must run here — once, before any other extractor.
        let ws = WebSocketUpgrade::from_request_parts(parts, state).await?;
        let headers = parts.headers.clone();
        let query = parts.uri.query().map(|q| q.to_string());
        let token = resolve_bearer(&headers, query.as_deref());
        let config = WebSocketConfig::default();
        Ok(Self {
            identity: token.unwrap_or_default(),
            ws,
            max_frame_size: config.max_frame_size,
            buffer: config.buffer,
        })
    }
}

/// Upgrade a WebSocket and run the authorized broadcast session.
///
/// The connection identity is resolved per handshake — never shared — from
/// the bearer credential (`Authorization: Bearer <token>` or `?token=`),
/// using the token substring as the subscriber id (full JWT resolution is a
/// follow-up). Gate semantics (FS-M6-01): a `subscribe` to a private channel
/// that this connection's identity is not authorized for closes the socket
/// with `4403` — never a silent drop.
pub async fn ws_handler<S>(ws: WsConnection, State(hub): State<BroadcastHub>) -> Response
where
    S: Send + Sync,
    BroadcastHub: axum::extract::FromRef<S>,
{
    ws.on_upgrade(hub)
}

/// Run one authenticated broadcast session until the peer closes.
///
/// Subscribers receive ordered push frames; the optional `prompt` event
/// streams `data` back as ordered `token` frames (AI streaming over WS). The
/// prompt path requires a named channel and passes the same authorization
/// gate as `subscribe` — a denied prompt is closed `4403`, an identity-less
/// private prompt `4401` (audit S5 S1).
pub async fn session(socket: WebSocket, hub: BroadcastHub, identity: String) {
    let (mut sink, mut source) = socket.split();

    let _ = sink
        .send(Message::Text("{\"event\":\"connected\"}".to_string()))
        .await;

    let Some(Ok(first)) = source.next().await else {
        let _ = sink.close().await;
        return;
    };
    let frame: Option<SubscribeFrame> = match &first {
        Message::Text(text) => serde_json::from_str(text).ok(),
        _ => None,
    };
    let Some(frame) = frame else {
        send_close(
            &mut sink,
            axum::extract::ws::close_code::PROTOCOL,
            "expected subscribe frame",
        )
        .await;
        return;
    };
    if frame.event == "prompt" {
        // AI streaming echo over WS: the prompt is tied to a channel so the
        // connection's identity is authorized exactly like a `subscribe`
        // before anything is streamed.
        let Some(channel) = frame.channel else {
            send_close(
                &mut sink,
                axum::extract::ws::close_code::PROTOCOL,
                "prompt requires a channel",
            )
            .await;
            return;
        };
        match hub.authorize(&identity, &channel).await {
            Ok(()) => {}
            Err(err) => {
                let code = unauthorized_code(&err);
                send_close(&mut sink, code, err.to_string()).await;
                return;
            }
        }
        let Some(data) = frame.data else {
            send_close(
                &mut sink,
                axum::extract::ws::close_code::PROTOCOL,
                "prompt requires data",
            )
            .await;
            return;
        };
        for line in data.lines() {
            let event = WsMessage::new("token", &channel, line.to_string());
            if sink.send(Message::Text(ws_frame(&event))).await.is_err() {
                break;
            }
        }
        let done = WsMessage::new("done", &channel, "[]");
        let _ = sink.send(Message::Text(ws_frame(&done))).await;
        send_close(
            &mut sink,
            axum::extract::ws::close_code::NORMAL,
            "prompt complete",
        )
        .await;
        return;
    }
    let Some(channel) = frame.channel else {
        send_close(
            &mut sink,
            axum::extract::ws::close_code::PROTOCOL,
            "missing channel",
        )
        .await;
        return;
    };
    match hub.subscribe(&identity, &channel).await {
        Ok(mut rx) => {
            // B2 fix: ack is serde-serialized so the channel name is
            // JSON-escaped (no raw string interpolation / injection).
            let ack = serde_json::to_string(&WsAck::subscribed(&channel))
                .unwrap_or_else(|_| "{}".to_string());
            if sink.send(Message::Text(ack)).await.is_err() {
                return;
            }
            loop {
                tokio::select! {
                    broadcast = rx.recv() => {
                        match broadcast {
                            Ok(message) => {
                                if sink.send(Message::Text(ws_frame(&message))).await.is_err() {
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(broadcast::error::RecvError::Closed) => break,
                        }
                    }
                    next = source.next() => {
                        match next {
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Ok(_)) => {}
                            Some(Err(_)) => break,
                        }
                    }
                }
            }
            let _ = sink.close().await;
        }
        Err(err) => {
            let code = unauthorized_code(&err);
            send_close(&mut sink, code, err.to_string()).await;
        }
    }
}

/// Method-router helper for `Router::route("/broadcasting/auth", ws_route::<AppState>())`.
///
/// `S` is the router state; `BroadcastHub` must be reachable from it via
/// `FromRef` (or be the state itself). The state type is resolved explicitly
/// because `get` cannot infer it from the extractor alone.
pub fn ws_route<S>() -> axum::routing::MethodRouter<S>
where
    S: Send + Sync + Clone + 'static,
    BroadcastHub: axum::extract::FromRef<S>,
{
    axum::routing::get(ws_handler::<S>)
}

/// Map an auth failure to the 4xxx WebSocket close code.
pub fn unauthorized_code(err: &BroadcastError) -> u16 {
    match err {
        BroadcastError::Unauthorized { .. } => crate::channel::WS_CLOSE_UNAUTHORIZED,
        BroadcastError::Unauthenticated { .. } => crate::channel::WS_CLOSE_UNAUTHENTICATED,
        _ => axum::extract::ws::close_code::NORMAL,
    }
}

/// Send a close frame with `code`/`reason` over a sink.
async fn send_close<S>(sink: &mut S, code: u16, reason: impl Into<String>)
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let _ = sink
        .send(Message::Close(Some(axum::extract::ws::CloseFrame {
            code,
            reason: reason.into().into(),
        })))
        .await;
}

/// Resolve the bearer credential from a handshake's `Authorization` header.
fn bearer_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
}

/// Pull a `token` value out of a URL query string (`?token=...`).
fn token_from_query(query: Option<&str>) -> Option<String> {
    let query = query?;
    let token = query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == "token" {
            Some(value)
        } else {
            None
        }
    })?;
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Extract the bearer token from a request's headers or `?token=` query.
pub fn resolve_bearer(headers: &axum::http::HeaderMap, query: Option<&str>) -> Option<String> {
    bearer_from_headers(headers).or_else(|| token_from_query(query))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_message_serializes() {
        let msg = WsMessage::new("OrderShipped", "orders.1", "{}");
        let wire = ws_frame(&msg);
        assert!(wire.contains("\"channel\":\"orders.1\""));
        assert!(wire.contains("\"event\":\"OrderShipped\""));
    }

    #[test]
    fn close_code_maps_unauthorized() {
        let err = BroadcastError::Unauthorized {
            channel: "private-chat.1".to_string(),
        };
        assert_eq!(unauthorized_code(&err), 4403);
    }

    #[test]
    fn subscribed_ack_escapes_channel_json() {
        // B2 regression: quote/backslash in the channel must be escaped, not
        // spliced raw into the wire JSON.
        let wire = serde_json::to_string(&WsAck::subscribed("pr\"iv\\ate.1")).unwrap();
        assert_eq!(
            wire,
            "{\"event\":\"subscribed\",\"channel\":\"pr\\\"iv\\\\ate.1\"}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(parsed["event"], "subscribed");
        assert_eq!(parsed["channel"], "pr\"iv\\ate.1");
    }

    #[test]
    fn bearer_extracted_from_header_and_query() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer abc123"),
        );
        assert_eq!(resolve_bearer(&headers, None).as_deref(), Some("abc123"));

        let empty = axum::http::HeaderMap::new();
        assert_eq!(
            resolve_bearer(&empty, Some("token=xyz")).as_deref(),
            Some("xyz")
        );
        assert_eq!(resolve_bearer(&empty, None), None);
        assert_eq!(
            resolve_bearer(&empty, Some("token=abc&other=1")).as_deref(),
            Some("abc")
        );
    }
}
