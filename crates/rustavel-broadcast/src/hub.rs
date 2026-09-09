//! Broadcast hub: shared fan-out state and per-identity subscription auth.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::channel::Authorize;
use crate::error::{BroadcastError, Result};

/// Capacity of the fan-out broadcast channel.
pub const WS_BUFFER: usize = 256;

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

/// Router state shared by a broadcast endpoint.
///
/// Holds no identity: authorization is performed per subscription with the
/// identity of the calling connection (audit S5 B1), so one hub instance can
/// serve many sockets under different identities.
#[derive(Clone)]
pub struct BroadcastHub {
    /// Fan-out per wire channel name.
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<WsMessage>>>>,
    /// Authorization gate for private/presence channels.
    gate: Arc<dyn Authorize>,
    /// Outbound buffer size.
    buffer: usize,
}

impl BroadcastHub {
    /// Create a hub with a channel authorization gate.
    pub fn new(gate: impl Authorize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            gate: Arc::new(gate),
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

    /// Run the subscription authorization gate for `wire_channel`.
    ///
    /// Shared by `subscribe` and the streaming `prompt` path so both deny an
    /// unauthenticated/unauthorized connection before any payload is produced
    /// (audit S5 S1): a denied join surfaces [`BroadcastError::Unauthorized`]
    /// and a missing identity on a private channel surfaces
    /// [`BroadcastError::Unauthenticated`].
    pub async fn authorize(&self, identity: &str, wire_channel: &str) -> Result<()> {
        let kind = normalize_kind(wire_channel);
        let decision = crate::channel::authorize_subscription(
            kind,
            Some(&crate::channel::Subscriber {
                id: identity.to_string(),
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
        Ok(())
    }

    /// Subscribe `identity` to a wire channel.
    ///
    /// The wire name is normalized back to its kind (`private-chat.1` →
    /// `Private("chat.1")`) so the [`Authorize`] gate receives the prefixed
    /// name it keys on. Denials surface as [`BroadcastError::Unauthorized`].
    /// The identity is a per-connection value — the hub stores no identity.
    pub async fn subscribe(
        &self,
        identity: &str,
        wire_channel: &str,
    ) -> Result<broadcast::Receiver<WsMessage>> {
        self.authorize(identity, wire_channel).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Gate allowing only `user-1` on `private-chat.1`.
    struct AllowOne;

    #[async_trait::async_trait]
    impl Authorize for AllowOne {
        async fn authorize(&self, channel: &str, identity: &str) -> crate::channel::AuthDecision {
            if channel == "private-chat.1" && identity == "user-1" {
                crate::channel::AuthDecision::Allow
            } else {
                crate::channel::AuthDecision::Deny("not allowed".to_string())
            }
        }
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
        let hub = BroadcastHub::new(DenyAll);
        let err = hub.subscribe("user-2", "private-chat.1").await.unwrap_err();
        assert!(matches!(
            err,
            BroadcastError::Unauthorized { ref channel } if channel == "private-chat.1"
        ));
    }

    #[tokio::test]
    async fn identities_are_resolved_per_connection_not_shared() {
        // B1 regression shape: two sockets with different tokens resolve to
        // different identities; the second is denied on the private channel.
        let hub = BroadcastHub::new(AllowOne);

        // Socket one authenticates as user-1 and may join the private channel.
        let allowed = hub
            .subscribe("user-1", "private-chat.1")
            .await
            .expect("first identity authorized");
        drop(allowed);

        // Socket two with a different token resolves to user-2 and is denied.
        let err = hub.subscribe("user-2", "private-chat.1").await.unwrap_err();
        assert!(matches!(err, BroadcastError::Unauthorized { .. }));
    }

    #[tokio::test]
    async fn public_channel_allows_any_identity() {
        let hub = BroadcastHub::new(AllowOne);
        let rx = hub
            .subscribe("anonymous", "orders.1")
            .await
            .expect("public channel needs no auth");
        drop(rx);
    }

    #[tokio::test]
    async fn subscribe_then_publish_fans_out() {
        let hub = BroadcastHub::new(AllowOne);
        let mut rx = hub
            .subscribe("user-1", "private-chat.1")
            .await
            .expect("user-1 allowed");
        hub.publish(
            "private-chat.1",
            WsMessage::new("UserCreated", "private-chat.1", "{\"name\":\"Ada\"}"),
        )
        .await
        .unwrap();
        let msg = rx.recv().await.expect("message delivered");
        assert_eq!(msg.event, "UserCreated");
        assert_eq!(msg.channel, "private-chat.1");
    }
}
