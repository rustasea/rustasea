//! Errors raised by the livewire runtime and their HTTP representation.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Failure while constructing or patching a component's JSON state.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// Component state must be a JSON object.
    #[error("component state must be a JSON object")]
    NotAnObject,

    /// A state patch must be a JSON object.
    #[error("state patch must be a JSON object")]
    PatchNotAnObject,
}

/// Error produced while authorizing, patching, rendering, or broadcasting a
/// livewire component action.
///
/// The [`std::fmt::Display`] form is for server-side logs and names the
/// component/action; the [`IntoResponse`] implementation deliberately discards
/// that detail and returns a generic status body so internals never leak.
#[derive(Debug, thiserror::Error)]
pub enum LivewireError {
    /// No component is registered under the requested name.
    #[error("livewire component not found: `{name}`")]
    ComponentNotFound {
        /// Component name that was requested.
        name: String,
    },

    /// The component has no handler registered for the requested action.
    #[error("livewire action not found: `{component}.{action}`")]
    ActionNotFound {
        /// Component that was targeted.
        component: String,
        /// Action that was requested.
        action: String,
    },

    /// The actor is not authorized to run the action.
    #[error("unauthorized livewire action: `{component}.{action}`")]
    Unauthorized {
        /// Component that was targeted.
        component: String,
        /// Action that was denied.
        action: String,
    },

    /// The component state or its patch was invalid.
    #[error("invalid livewire state: {0}")]
    InvalidState(#[from] StateError),

    /// An action handler rejected the request.
    #[error("livewire action failed for `{component}.{action}`: {reason}")]
    Action {
        /// Component that was targeted.
        component: String,
        /// Action that failed.
        action: String,
        /// Caller-safe rejection reason.
        reason: String,
    },

    /// A template failed to render.
    #[error("failed to render livewire component `{component}`")]
    Render {
        /// Component whose template failed.
        component: String,
        /// Underlying view failure (logged, never sent to the client).
        #[source]
        source: rustasea_view::ViewError,
    },

    /// The realtime fan-out failed.
    #[error("failed to broadcast livewire component `{component}`")]
    Broadcast {
        /// Component whose update failed to broadcast.
        component: String,
        /// Underlying broadcast failure.
        #[source]
        source: rustasea_broadcast::BroadcastError,
    },
}

impl IntoResponse for LivewireError {
    /// Map a livewire failure to a generic status response.
    fn into_response(self) -> Response {
        let status = match self {
            LivewireError::ComponentNotFound { .. } | LivewireError::ActionNotFound { .. } => {
                StatusCode::NOT_FOUND
            }
            LivewireError::Unauthorized { .. } => StatusCode::FORBIDDEN,
            LivewireError::InvalidState(_) | LivewireError::Action { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            LivewireError::Render { .. } | LivewireError::Broadcast { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let body = status.canonical_reason().unwrap_or("Error");
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Authorization failures map to `403 Forbidden`.
    #[test]
    fn unauthorized_maps_to_forbidden() {
        let error = LivewireError::Unauthorized {
            component: "counter".to_string(),
            action: "increment".to_string(),
        };
        assert_eq!(error.into_response().status(), StatusCode::FORBIDDEN);
    }

    /// Missing components map to `404 Not Found`.
    #[test]
    fn missing_component_maps_to_not_found() {
        let error = LivewireError::ComponentNotFound {
            name: "counter".to_string(),
        };
        assert_eq!(error.into_response().status(), StatusCode::NOT_FOUND);
    }

    /// Invalid state maps to `422 Unprocessable Entity`.
    #[test]
    fn invalid_state_maps_to_unprocessable_entity() {
        let error = LivewireError::InvalidState(StateError::NotAnObject);
        assert_eq!(
            error.into_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
