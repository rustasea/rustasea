//! Authorization gate for component actions.

use serde::{Deserialize, Serialize};

/// Identity resolved by auth middleware for the current request.
///
/// The actor is attached to the request as an axum extension by the session
/// guard; it is never read from the request body, so a client cannot spoof it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// Stable user/connection identifier.
    pub id: String,
}

impl Actor {
    /// Create an actor from an identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Authorization gate consulted before any component action runs.
///
/// The runtime denies by default when no gate is configured (see
/// [`DenyAll`]) and always evaluates the gate **before** mutating state, so an
/// unauthorized caller cannot observe whether an action exists.
pub trait ActionAuthorizer: Send + Sync + 'static {
    /// Return `true` when `actor` may run `action` on `component`.
    fn is_authorized(&self, actor: Option<&Actor>, component: &str, action: &str) -> bool;
}

/// Authorizer that permits every action.
///
/// Intended for fully public components and tests; production components
/// should use a gate that checks the actor's identity or policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAll;

impl ActionAuthorizer for AllowAll {
    /// Permit every actor and action.
    fn is_authorized(&self, _actor: Option<&Actor>, _component: &str, _action: &str) -> bool {
        true
    }
}

/// Authorizer that denies every action — the secure default.
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyAll;

impl ActionAuthorizer for DenyAll {
    /// Deny every actor and action.
    fn is_authorized(&self, _actor: Option<&Actor>, _component: &str, _action: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AllowAll` permits anonymous and authenticated actors.
    #[test]
    fn allow_all_permits_every_action() {
        assert!(AllowAll.is_authorized(None, "counter", "increment"));
        assert!(AllowAll.is_authorized(Some(&Actor::new("user-1")), "counter", "increment"));
    }

    /// `DenyAll` denies even an authenticated actor.
    #[test]
    fn deny_all_denies_every_action() {
        assert!(!DenyAll.is_authorized(Some(&Actor::new("user-1")), "counter", "increment"));
    }
}
