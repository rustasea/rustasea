//! Rustavel Broadcast — realtime channels over WebSocket and SSE.
//!
//! Sprint 07 (M6) scope: the `ShouldBroadcast` trait, typed `Channel` kinds
//! (`Public(name)`/`Private(name)`/`Presence(name)`), the `Authorize` gate for
//! channel authentication, an SSE `eventStream` response, and an Axum
//! WebSocket broadcast handler. Events are serializable, cloneable payloads —
//! no `Any` (C-03).

pub mod channel;
pub mod error;
#[cfg(feature = "ws")]
pub mod hub;
pub mod sse;
#[cfg(feature = "ws")]
pub mod ws;

pub use async_trait::async_trait;
pub use channel::{
    authorize_subscription, AuthDecision, Authorize, Channel, PresenceUser, Private, Public,
    Subscriber, WS_CLOSE_UNAUTHENTICATED, WS_CLOSE_UNAUTHORIZED,
};
pub use error::{BroadcastError, Result};
#[cfg(feature = "ws")]
pub use hub::{BroadcastHub, WsMessage};
pub use sse::{event_stream, event_stream_response, EventSender, EventStream, SseEvent};
#[cfg(feature = "ws")]
pub use ws::{
    ws_frame, ws_handler, ws_route, SubscribeFrame, WebSocketConfig, WsAck, WsConnection,
};

/// Trait for broadcastable events that know their destination channel.
///
/// Implementing this trait advertises that instances participate in channel
/// broadcasting; the channel identity is derived by implementors via
/// [`ShouldBroadcast::broadcast_on`] (Laravel `ShouldBroadcast` parity).
pub trait ShouldBroadcast: Send + Sync + 'static {
    /// Channel this event broadcasts on.
    fn broadcast_on(&self) -> Channel;
}

/// A broadcastable event payload.
pub trait BroadcastEvent: Send + Sync + 'static {
    /// Stable event name reported to channel subscribers.
    fn event_name(&self) -> &'static str;
}

/// Serialize a broadcastable payload into the wire push shape
/// `{ event, channel, data }` (FS-M6-01).
pub fn to_wire(
    event_name: &str,
    channel: &Channel,
    data: &impl serde::Serialize,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "event": event_name,
        "channel": channel.auth_channel(),
        "data": serde_json::to_value(data)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UserCreated {
        name: String,
    }

    impl ShouldBroadcast for UserCreated {
        fn broadcast_on(&self) -> Channel {
            Channel::Private("chat.1".to_string())
        }
    }

    #[test]
    fn should_broadcast_exposes_channel() {
        let event = UserCreated {
            name: "Ada".to_string(),
        };
        assert_eq!(event.broadcast_on().auth_channel(), "private-chat.1");
    }

    #[test]
    fn to_wire_produces_push_shape() {
        let channel = Channel::Private("chat.1".to_string());
        let wire = to_wire(
            "UserCreated",
            &channel,
            &serde_json::json!({ "name": "Ada" }),
        )
        .unwrap();
        assert_eq!(wire["event"], "UserCreated");
        assert_eq!(wire["channel"], "private-chat.1");
        assert_eq!(wire["data"]["name"], "Ada");
    }
}
