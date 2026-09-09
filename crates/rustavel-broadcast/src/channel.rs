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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    /// Anyone may subscribe (e.g. `Public("orders.1")`).
    Public(String),
    /// Only authenticated, authorized users may subscribe (`private-*`).
    Private(String),
    /// Authorized users subscribe and are visible to each other (`presence-*`).
    Presence(String),
}

impl Channel {
    /// The wire channel name (`private-chat.1`, `orders.1`, …).
    pub fn name(&self) -> &str {
        match self {
            Channel::Public(name) | Channel::Private(name) | Channel::Presence(name) => name,
        }
    }

    /// Channel kind reported on the wire (`public`/`private`/`presence`).
    pub fn kind(&self) -> &'static str {
        match self {
            Channel::Public(_) => "public",
            Channel::Private(_) => "private",
            Channel::Presence(_) => "presence",
        }
    }

    /// Whether subscribing requires an identity.
    pub fn requires_auth(&self) -> bool {
        !matches!(self, Channel::Public(_))
    }

    /// The channel name with its kind prefix (`private-`/`presence-`), which
    /// is what the `Authorize` gate keys on (Laravel `private-chat.1` parity).
    pub fn auth_channel(&self) -> String {
        match self {
            Channel::Public(name) => name.clone(),
            Channel::Private(name) => format!("private-{name}"),
            Channel::Presence(name) => format!("presence-{name}"),
        }
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
/// Implementors decide whether `identity` may subscribe to `channel`; the
/// channel name is the prefixed wire name (`private-chat.1`).
#[async_trait::async_trait]
pub trait Authorize: Send + Sync + 'static {
    /// Authorize an identity for a channel; `Deny` maps to 403/4403.
    async fn authorize(&self, channel: &str, identity: &str) -> AuthDecision;
}

/// Trait alias kept for DSL parity (`gate: impl Authorize` reads naturally).
pub trait Gate: Authorize {}

impl<T: Authorize> Gate for T {}

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

/// WebSocket close code for authorization denials (`4403 Forbidden`).
pub const WS_CLOSE_UNAUTHORIZED: u16 = 4403;

/// WebSocket close code for unauthenticated subscribe attempts (`4401`).
pub const WS_CLOSE_UNAUTHENTICATED: u16 = 4401;

/// Run the authentication gate for a channel subscription.
///
/// Public channels always resolve to [`AuthDecision::Allow`]; private and
/// presence channels require a `Subscriber` and a passing [`Authorize`].
/// The gate keys on the prefixed name ([`Channel::auth_channel`]) so a
/// `Channel::Private("chat.1")` is authorized as `private-chat.1`.
pub async fn authorize_subscription(
    channel: Channel,
    subscriber: Option<&Subscriber>,
    gate: Option<&dyn Authorize>,
) -> Result<AuthDecision> {
    if !channel.requires_auth() {
        return Ok(AuthDecision::Allow);
    }
    let (Some(sub), Some(gate)) = (subscriber, gate) else {
        return Err(BroadcastError::Unauthenticated {
            channel: channel.auth_channel(),
        });
    };
    let wire = channel.auth_channel();
    match gate.authorize(&wire, &sub.id).await {
        AuthDecision::Allow => Ok(AuthDecision::Allow),
        AuthDecision::Deny(_) => Err(BroadcastError::Unauthorized { channel: wire }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AllowList;

    #[async_trait::async_trait]
    impl Authorize for AllowList {
        async fn authorize(&self, channel: &str, identity: &str) -> AuthDecision {
            if channel == "private-chat.1" && identity == "user-1" {
                AuthDecision::Allow
            } else {
                AuthDecision::Deny("not allowed".to_string())
            }
        }
    }

    #[test]
    fn channel_kind_and_wire_name() {
        let channel = Channel::Private("chat.1".to_string());
        assert_eq!(channel.kind(), "private");
        assert_eq!(channel.name(), "chat.1");
        assert_eq!(channel.auth_channel(), "private-chat.1");
        assert!(channel.requires_auth());
    }

    #[tokio::test]
    async fn public_channel_needs_no_auth() {
        let decision = authorize_subscription(Channel::Public("orders.1".to_string()), None, None)
            .await
            .unwrap();
        assert_eq!(decision, AuthDecision::Allow);
    }

    #[tokio::test]
    async fn private_channel_without_subscriber_is_rejected() {
        let err = authorize_subscription(Channel::Private("chat.1".to_string()), None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, BroadcastError::Unauthenticated { .. }));
    }

    #[tokio::test]
    async fn private_channel_denial_surfaces_unauthorized() {
        let sub = Subscriber {
            id: "user-2".to_string(),
            name: None,
        };
        let err = authorize_subscription(
            Channel::Private("chat.1".to_string()),
            Some(&sub),
            Some(&AllowList),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            BroadcastError::Unauthorized { ref channel } if channel == "private-chat.1"
        ));
    }

    #[tokio::test]
    async fn authorized_subscription_allows() {
        let sub = Subscriber {
            id: "user-1".to_string(),
            name: None,
        };
        let decision = authorize_subscription(
            Channel::Private("chat.1".to_string()),
            Some(&sub),
            Some(&AllowList),
        )
        .await
        .unwrap();
        assert_eq!(decision, AuthDecision::Allow);
    }
}
