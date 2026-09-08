//! Channel kinds and authentication gate for realtime broadcasting.

use serde::{Deserialize, Serialize};

use crate::error::{BroadcastError, Result};

/// Authentication outcome for a channel join attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    /// The identity may subscribe.
    Allow,
    /// The identity may not subscribe (HTTP 403 / WS close 4403).
    Deny(String),
}

/// Kinds of broadcast channel with their authorization semantics.
///
/// `Public` channels need no identity; `Private` and `Presence` channels are
/// gated by [`Authorize::authorize`]. Presence channels additionally carry a
/// member list through [`PresenceUser`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    /// Anyone may subscribe (e.g. `orders.public`).
    Public,
    /// Only authenticated, authorized users may subscribe (`private-*`).
    Private,
    /// Authorized users subscribe and are visible to each other (`presence-*`).
    Presence,
}

impl Channel {
    /// Whether subscribing requires an identity.
    pub fn requires_auth(&self) -> bool {
        !matches!(self, Channel::Public)
    }
}

/// Marker type for public channel declarations (Laravel `PublicChannel` parity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Public;

/// Marker type for private channel declarations (Laravel `PrivateChannel` parity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Private;

/// Channel authorization gate — Laravel `Broadcast::channel` callback parity.
///
/// Implementors decide whether `identity` may subscribe to `channel`.
#[async_trait::async_trait]
pub trait Authorize: Send + Sync + 'static {
    /// Authorize an identity for a channel; `Deny` maps to 403/4403.
    async fn authorize(&self, channel: &str, identity: &str) -> AuthDecision;
}

/// Identity supplied with a WebSocket/SSE subscribe attempt.
///
/// None for anonymous sockets on public channels; required once a
/// private/presence channel is requested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscriber {
    /// Stable user/connection identifier.
    pub id: String,
    /// Optional display name (presence channels).
    pub name: Option<String>,
}

/// Presence channel member entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceUser {
    /// User identifier shown to other members.
    pub id: String,
    /// Display name shown to other members.
    pub name: String,
}

/// Run the authentication gate for a channel subscription.
///
/// Public channels always resolve to [`AuthDecision::Allow`]; private and
/// presence channels require a `Subscriber` and a passing [`Authorize`].
pub async fn authorize_subscription(
    channel: Channel,
    channel_name: &str,
    subscriber: Option<&Subscriber>,
    gate: Option<&dyn Authorize>,
) -> Result<AuthDecision> {
    if !channel.requires_auth() {
        return Ok(AuthDecision::Allow);
    }
    let (Some(sub), Some(gate)) = (subscriber, gate) else {
        return Err(BroadcastError::Unauthenticated {
            channel: channel_name.to_string(),
        });
    };
    Ok(gate.authorize(channel_name, &sub.id).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DenyAll;

    #[async_trait::async_trait]
    impl Authorize for DenyAll {
        async fn authorize(&self, _channel: &str, _identity: &str) -> AuthDecision {
            AuthDecision::Deny("not allowed".to_string())
        }
    }

    #[tokio::test]
    async fn public_channel_needs_no_auth() {
        let decision = authorize_subscription(Channel::Public, "orders.1", None, None)
            .await
            .unwrap();
        assert_eq!(decision, AuthDecision::Allow);
    }

    #[tokio::test]
    async fn private_channel_without_subscriber_is_rejected() {
        let err = authorize_subscription(Channel::Private, "private-orders.1", None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, BroadcastError::Unauthenticated { .. }));
    }

    #[tokio::test]
    async fn private_channel_denial_surfaces() {
        let decision = authorize_subscription(
            Channel::Private,
            "private-orders.1",
            Some(&Subscriber {
                id: "u1".to_string(),
                name: None,
            }),
            Some(&DenyAll),
        )
        .await
        .unwrap();
        assert_eq!(decision, AuthDecision::Deny("not allowed".to_string()));
    }
}
