//! WebSocket broadcasting stub over `axum::extract::ws`.

use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::{BroadcastError, Result};

/// Capacity of the fan-out broadcast channel.
pub const WS_BUFFER: usize = 256;

/// Configuration for the WebSocket broadcast stub.
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

/// Frame the wire message as a text WebSocket message.
pub fn ws_frame(message: &WsMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| "{}".to_string())
}

/// Stub WS handler used by `Router::ws("/channel")` in M6 apps.
///
/// Accepts the socket, echoes a `connected` frame, then pumps text frames
/// until the peer closes. Contract (FS-M6-01): channel auth is enforced
/// *before* this handler via the [`crate::channel::Authorize`] gate (403 on
/// denial, close 4403 mid-stream) — full gated dispatch lands with the M6
/// router integration.
pub async fn ws_broadcast_handler(mut socket: WebSocket) -> Result<()> {
    socket
        .send(Message::Text("{\"event\":\"connected\"}".to_string()))
        .await
        .map_err(|e| BroadcastError::Transport(e.to_string()))?;
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Close(_) = msg {
            break;
        }
    }
    Ok(())
}

/// Dummy subscriber used to prove the broadcast type-checks (never connected).
pub fn _keep_broadcast_type(_tx: broadcast::Sender<WsMessage>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_message_serializes() {
        let msg = WsMessage {
            channel: "orders.1".to_string(),
            event: "OrderShipped".to_string(),
            data: "{}".to_string(),
        };
        let wire = ws_frame(&msg);
        assert!(wire.contains("\"channel\":\"orders.1\""));
    }
}
