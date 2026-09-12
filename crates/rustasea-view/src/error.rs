//! View rendering errors and their HTTP representation.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Error produced while resolving, serializing, or rendering a view.
///
/// The [`std::fmt::Display`] form is intended for server-side logs: it carries
/// the template name (application-relative, never an absolute path) and the
/// underlying cause. The [`IntoResponse`] implementation deliberately discards
/// all of that and returns a generic `500`.
#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    /// No template is registered under the requested name.
    #[error("view template not found: `{name}`")]
    TemplateNotFound {
        /// Template name (application-relative).
        name: String,
    },

    /// The view context could not be serialized for the engine.
    #[error("failed to serialize view data for `{name}`")]
    Data {
        /// Template name.
        name: String,
        /// Serialization failure.
        #[source]
        source: serde_json::Error,
    },

    /// The compile-time (askama) template failed to render.
    #[error("failed to render view `{name}`")]
    Render {
        /// Template name.
        name: String,
        /// askama rendering failure.
        #[source]
        source: askama::Error,
    },

    /// A runtime (minijinja) template failed to load or render.
    #[cfg(feature = "runtime-templates")]
    #[error("failed to render runtime view `{name}`")]
    Runtime {
        /// Template name (application-relative).
        name: String,
        /// minijinja failure.
        #[source]
        source: minijinja::Error,
    },
}

impl ViewError {
    /// Build a [`ViewError::Data`] for `name`.
    pub(crate) fn data(name: &str, source: serde_json::Error) -> Self {
        Self::Data {
            name: name.to_string(),
            source,
        }
    }

    /// Build a [`ViewError::Render`] for `name`.
    pub(crate) fn render(name: &str, source: askama::Error) -> Self {
        Self::Render {
            name: name.to_string(),
            source,
        }
    }

    /// Build a [`ViewError::TemplateNotFound`] for `name`.
    pub(crate) fn template_not_found(name: &str) -> Self {
        Self::TemplateNotFound {
            name: name.to_string(),
        }
    }

    /// Build a [`ViewError::Runtime`] for `name`.
    #[cfg(feature = "runtime-templates")]
    pub(crate) fn runtime(name: &str, source: minijinja::Error) -> Self {
        Self::Runtime {
            name: name.to_string(),
            source,
        }
    }
}

/// Convert a render failure into a generic `500 Internal Server Error`.
///
/// No part of the [`ViewError`] — template name, loader path, or underlying
/// source — is written to the response body; those details stay server-side
/// and are available to callers for logging before the response is produced.
impl IntoResponse for ViewError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Display form names the template for server-side logs.
    #[test]
    fn display_names_the_template() {
        let error = ViewError::template_not_found("auth/login");
        assert!(error.to_string().contains("auth/login"));
    }

    /// A render failure becomes a generic 500 without leaking the template
    /// name or any filesystem path to the client.
    #[tokio::test]
    async fn render_error_response_is_a_generic_500() {
        let response = ViewError::template_not_found("/etc/passwd").into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read error body");
        let body = String::from_utf8(body.to_vec()).expect("utf-8 body");
        assert_eq!(body, "Internal Server Error");
        assert!(
            !body.contains("passwd"),
            "filesystem path must not leak into the response body"
        );
    }
}
