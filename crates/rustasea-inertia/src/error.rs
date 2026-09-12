//! Inertia response-building errors and their HTTP representation.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Error produced while building an Inertia response.
///
/// The [`std::fmt::Display`] form is intended for server-side logs and carries
/// the component name plus the underlying cause. The [`IntoResponse`]
/// implementation deliberately discards all of that and returns a generic
/// `500 Internal Server Error`, so serialization internals never reach the
/// client.
#[derive(Debug, thiserror::Error)]
pub enum InertiaError {
    /// Page props could not be serialized to JSON.
    #[error("failed to serialize props for component `{component}`")]
    SerializeProps {
        /// Component being rendered.
        component: String,
        /// Serialization failure.
        #[source]
        source: serde_json::Error,
    },

    /// Page props serialized to a non-object JSON value.
    ///
    /// Inertia props are a named map at the wire boundary; an array or scalar
    /// cannot be merged with shared props or filtered by name.
    #[error("props for component `{component}` must serialize to a JSON object")]
    PropsNotObject {
        /// Component being rendered.
        component: String,
    },

    /// The page envelope could not be serialized for the root document.
    #[error("failed to serialize the Inertia page")]
    SerializePage {
        /// Serialization failure.
        #[source]
        source: serde_json::Error,
    },

    /// The configured root view failed to render.
    #[error("root view rendering failed")]
    RootView {
        /// Failure surfaced by the [`crate::RootView`] implementation.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Convert a response-building failure into a generic `500 Internal Server
/// Error` without leaking component names or serialization internals.
impl IntoResponse for InertiaError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    /// A serialization failure becomes a generic 500 without leaking the
    /// component name or any internal detail to the client.
    #[tokio::test]
    async fn error_response_is_a_generic_500() {
        let error = InertiaError::PropsNotObject {
            component: "secret/component".to_string(),
        };
        assert!(error.to_string().contains("secret/component"));

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let body = String::from_utf8(body.to_vec()).expect("utf-8 body");
        assert_eq!(body, "Internal Server Error");
        assert!(!body.contains("secret/component"));
    }
}
