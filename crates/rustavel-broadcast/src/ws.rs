//! WebSocket broadcasting over `axum::extract::ws` with channel auth.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::channel::Authorize;
use crate::error::{BroadcastError, Result};

/// Capacity of the fan-out broadcast channel.
pub const WS_BUFFER: usize = 256;

/// Configuration for the WebSocket broadcast.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

/// One broadcast payload destined for channel subscribers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// Channel this payload targets.
    pub channel: String,
    /// Event name.
    pub event: String,
    /// Serialized event payload.
    pub data: String,
}

impl WsMessage {
    /// Build a push payload from a broadcast event (FS-M6-01 wire shape).
    pub fn new(
        event: impl Into<String>,
        channel: impl Into<String>,
        data: impl Into<String>,
    ) -> Self {
        Self {
            event: event.into(),
            channel: channel.into(),
            data: data.into(),
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

/// Router state shared by a broadcast endpoint.
#[derive(Clone)]
pub struct BroadcastHub {
    /// Fan-out per wire channel name.
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<WsMessage>>>>,
    /// Authorization gate for private/presence channels.
    gate: Arc<dyn Authorize>,
    /// Subscriber id parsed from the handshake (Bearer identity).
    identity: String,
    /// Outbound buffer size.
    buffer: usize,
}

impl BroadcastHub {
    /// Create a hub with a channel auth gate and subscriber identity.
    pub fn new(gate: impl Authorize, identity: impl Into<String>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            gate: Arc::new(gate),
            identity: identity.into(),
            buffer: WS_BUFFER,
        }
    }

    /// Set the outbound buffer size (must be > 0).
    pub fn with_buffer(mut self, buffer: usize) -> Self {
        if buffer > 0 {
            self.buffer = buffer;
        }
        self
    }

    /// Identity authorizing this hub's subscriptions.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Publish a push payload to a channel's subscribers.
    pub async fn publish(&self, channel: &str, message: WsMessage) -> Result<()> {
        let sender = {
            let channels = self.channels.read().await;
            channels.get(channel).cloned()
        };
        match sender {
            Some(tx) => {
                let _ = tx.send(message);
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Subscribe this hub identity to a wire channel.
    ///
    /// The wire name is normalized back to its kind (`private-chat.1` →
    /// `Private("chat.1")`) so the [`Authorize`] gate receives the prefixed
    /// name it keys on. Denials surface as [`BroadcastError::Unauthorized`].
    pub async fn subscribe(&self, wire_channel: &str) -> Result<broadcast::Receiver<WsMessage>> {
        let kind = normalize_kind(wire_channel);
        let decision = crate::channel::authorize_subscription(
            kind,
            Some(&crate::channel::Subscriber {
                id: self.identity.clone(),
                name: None,
            }),
            Some(self.gate.as_ref()),
        )
        .await?;
        if decision != crate::channel::AuthDecision::Allow {
            return Err(BroadcastError::Unauthorized {
                channel: wire_channel.to_string(),
            });
        }
        let mut channels = self.channels.write().await;
        let sender = channels
            .entry(wire_channel.to_string())
            .or_insert_with(|| broadcast::channel(self.buffer).0)
            .clone();
        Ok(sender.subscribe())
    }
}

/// Derive a [`Channel`] kind from a wire name by prefix.
fn normalize_kind(wire_channel: &str) -> crate::channel::Channel {
    if let Some(name) = wire_channel.strip_prefix("private-") {
        crate::channel::Channel::Private(name.to_string())
    } else if let Some(name) = wire_channel.strip_prefix("presence-") {
        crate::channel::Channel::Presence(name.to_string())
    } else {
        crate::channel::Channel::Public(wire_channel.to_string())
    }
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

/// Upgrade a WebSocket and run the authorized broadcast session.
///
/// Gate semantics (FS-M6-01): a `subscribe` to a private channel that the
/// hub identity is not authorized for closes the socket with `4403` — never a
/// silent drop. Authorized subscribers receive ordered push frames; the
/// optional `prompt` event streams `data` back as ordered `token` frames
/// (AI streaming over WS contract).
pub async fn ws_handler<S>(ws: WebSocketUpgrade, State(hub): State<BroadcastHub>) -> Response
where
    S: Send + Sync,
    BroadcastHub: axum::extract::FromRef<S>,
{
    ws.on_upgrade(move |socket| session(socket, hub))
}

/// Run one authenticated broadcast session until the peer closes.
pub async fn session(socket: WebSocket, hub: BroadcastHub) {
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
        // AI streaming echo: one token frame per line, then a done event.
        if let Some(data) = frame.data {
            for line in data.lines() {
                let event = WsMessage::new("token", "ai", line.to_string());
                if sink.send(Message::Text(ws_frame(&event))).await.is_err() {
                    break;
                }
            }
        }
        let done = WsMessage::new("done", "ai", "[]");
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
    match hub.subscribe(&channel).await {
        Ok(mut rx) => {
            let subscribed = format!("{{\"event\":\"subscribed\",\"channel\":\"{channel}\"}}");
            if sink.send(Message::Text(subscribed)).await.is_err() {
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

    #[tokio::test]
    async fn subscribe_denied_is_unauthorized() {
        struct DenyAll;
        #[async_trait::async_trait]
        impl Authorize for DenyAll {
            async fn authorize(
                &self,
                _channel: &str,
                _identity: &str,
            ) -> crate::channel::AuthDecision {
                crate::channel::AuthDecision::Deny("no".to_string())
            }
        }
        let hub = BroadcastHub::new(DenyAll, "user-2");
        let err = hub.subscribe("private-chat.1").await.unwrap_err();
        assert!(matches!(
            err,
            BroadcastError::Unauthorized { ref channel } if channel == "private-chat.1"
        ));
    }
}
