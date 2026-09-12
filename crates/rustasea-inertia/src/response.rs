//! Inertia HTTP responses: JSON pages, HTML root documents, and version
//! conflicts.

use axum::http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use crate::{InertiaError, Page, VARY, X_INERTIA, X_INERTIA_LOCATION};

/// Content type for Inertia JSON page responses.
pub const JSON_CONTENT_TYPE: &str = "application/json";
/// Content type for the HTML root document.
pub const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";

/// A built Inertia response, convertible into an axum response.
///
/// Carries the status, headers, and body explicitly so callers and tests can
/// inspect the wire decision (JSON page vs HTML shell vs `409`) without
/// consuming an axum response.
#[derive(Debug, Clone)]
pub struct InertiaResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl InertiaResponse {
    /// Build a response with `Vary: X-Inertia` and an explicit content type.
    fn base(status: StatusCode, body: impl Into<String>, content_type: &'static str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        headers.insert(VARY, HeaderValue::from_static(X_INERTIA));
        Self {
            status,
            headers,
            body: body.into(),
        }
    }

    /// Build a `200` Inertia JSON page response.
    ///
    /// Serialization of a [`Page<Value>`] cannot fail; the fallback body keeps
    /// the constructor infallible.
    pub fn json(page: &Page<Value>) -> Self {
        let body = serde_json::to_string(page).unwrap_or_else(|_| "{}".to_string());
        let mut response = Self::base(StatusCode::OK, body, JSON_CONTENT_TYPE);
        response
            .headers
            .insert(X_INERTIA, HeaderValue::from_static("true"));
        response
    }

    /// Build a `200` HTML root-document response.
    pub fn html(body: impl Into<String>) -> Self {
        Self::base(StatusCode::OK, body, HTML_CONTENT_TYPE)
    }

    /// Build a `409 Conflict` with the hard-navigation target.
    ///
    /// The `X-Inertia-Location` value falls back to `/` when `location` is not
    /// a valid header value, so a malformed URL can never panic a handler.
    pub fn version_conflict(location: &str) -> Self {
        let mut response = Self::base(StatusCode::CONFLICT, String::new(), JSON_CONTENT_TYPE);
        let value =
            HeaderValue::from_str(location).unwrap_or_else(|_| HeaderValue::from_static("/"));
        response.headers.insert(X_INERTIA_LOCATION, value);
        response
    }

    /// Build a generic `500` response from a response-building error.
    pub fn from_error(_error: InertiaError) -> Self {
        Self::base(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error",
            HTML_CONTENT_TYPE,
        )
    }

    /// HTTP status code.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Response headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Response body.
    pub fn body(&self) -> &str {
        &self.body
    }
}

impl IntoResponse for InertiaResponse {
    fn into_response(self) -> Response {
        let mut response = (self.status, self.body).into_response();
        *response.headers_mut() = self.headers;
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A JSON page response carries the Inertia marker and content type.
    #[test]
    fn json_response_sets_marker_and_content_type() {
        let page = Page::new("dashboard", json!({ "user": "Ada" }), "/dashboard", "v1");
        let response = InertiaResponse::json(&page);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], JSON_CONTENT_TYPE);
        assert_eq!(response.headers()[X_INERTIA], "true");
        assert_eq!(response.headers()[VARY], X_INERTIA);
    }

    /// A version conflict is a `409` with the location header and no body.
    #[test]
    fn version_conflict_sets_409_and_location() {
        let response = InertiaResponse::version_conflict("/dashboard?page=2");

        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response.headers()[X_INERTIA_LOCATION], "/dashboard?page=2");
        assert!(response.body().is_empty());
    }

    /// The `IntoResponse` conversion preserves status and headers.
    #[tokio::test]
    async fn into_response_preserves_headers() {
        let response = InertiaResponse::html("<html></html>").into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], HTML_CONTENT_TYPE);
        assert_eq!(response.headers()[VARY], X_INERTIA);
    }
}
