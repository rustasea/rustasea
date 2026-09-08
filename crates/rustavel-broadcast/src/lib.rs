//! Rustavel Broadcast — realtime channels over WebSocket and SSE.
//!
//! Sprint 07 (M6) scope: the `ShouldBroadcast` marker trait, typed
//! `Channel` kinds (`Public`/`Private`/`Presence`), the `Authorize` gate for
//! channel authentication, an SSE `eventStream` stub and an Axum WebSocket
//! stub. Events are serializable, cloneable payloads — no `Any` (C-03).

pub mod channel;
pub mod error;
pub mod sse;
#[cfg(feature = "ws")]
pub mod ws;

pub use async_trait::async_trait;
pub use channel::{AuthDecision, Authorize, Channel, PresenceUser, Private, Public, Subscriber};
pub use error::{BroadcastError, Result};
pub use sse::{event_stream, EventSender, EventStream, SseEvent};
#[cfg(feature = "ws")]
pub use ws::{ws_broadcast_handler, WebSocketConfig, WsMessage};

/// Marker trait for models that should broadcast model events.
///
/// Implementing this trait advertises that instances participate in channel
/// broadcasting; the channel identity is derived by implementors via
/// [`ShouldBroadcast::broadcast_channel`] (Laravel `ShouldBroadcast` parity).
pub trait ShouldBroadcast: Send + Sync + 'static {
    /// Channel this event broadcasts on.
    fn broadcast_channel(&self) -> String;
}

/// A broadcastable event payload.
pub trait BroadcastEvent: Send + Sync + 'static {
    /// Stable event name reported to channel subscribers.
    fn event_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OrderShipped;

    impl ShouldBroadcast for OrderShipped {
        fn broadcast_channel(&self) -> String {
            "orders.1".to_string()
        }
    }

    #[test]
    fn should_broadcast_exposes_channel() {
        assert_eq!(OrderShipped.broadcast_channel(), "orders.1");
    }
}
