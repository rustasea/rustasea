//! View responses: rendered HTML ([`ViewResponse`]) and the askama-backed
//! [`View<T>`] handler return type.

use askama::Template;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::error::ViewError;

/// Content type used for every rendered view.
pub const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";

/// A rendered view body ready to be converted into an axum response.
#[derive(Debug, Clone)]
pub struct ViewResponse {
    status: StatusCode,
    body: String,
    content_type: HeaderValue,
}

impl ViewResponse {
    /// Create a `200 OK` HTML view response from a rendered body.
    pub fn html(body: impl Into<String>) -> Self {
        Self {
            status: StatusCode::OK,
            body: body.into(),
            content_type: HeaderValue::from_static(HTML_CONTENT_TYPE),
        }
    }

    /// Create a view response with an explicit status and content type.
    pub fn new(status: StatusCode, body: impl Into<String>, content_type: HeaderValue) -> Self {
        Self {
            status,
            body: body.into(),
            content_type,
        }
    }

    /// Replace the HTTP status code.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// HTTP status code.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Rendered body.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Content type header value.
    pub fn content_type(&self) -> &HeaderValue {
        &self.content_type
    }

    /// Consume the response and return its body.
    pub fn into_body(self) -> String {
        self.body
    }
}

impl IntoResponse for ViewResponse {
    fn into_response(self) -> Response {
        let mut response = (self.status, self.body).into_response();
        response
            .headers_mut()
            .insert(CONTENT_TYPE, self.content_type);
        response
    }
}

/// A compile-time askama view that a handler can return directly.
///
/// `View<T>` implements [`IntoResponse`] whenever `T` implements
/// [`askama::Template`], which is the minimal integration point between this
/// crate and `rustasea-http`/axum: no router or HTTP-layer changes are needed.
///
/// ```rust,ignore
/// #[derive(askama::Template)]
/// #[template(path = "dashboard.html")]
/// struct Dashboard {
///     user: String,
/// }
///
/// async fn dashboard() -> rustasea_view::View<Dashboard> {
///     rustasea_view::View::new(Dashboard { user: "Ada".into() })
/// }
/// ```
#[derive(Debug, Clone)]
pub struct View<T> {
    template: T,
}

impl<T> View<T> {
    /// Wrap a template value in a view response.
    pub fn new(template: T) -> Self {
        Self { template }
    }

    /// Borrow the wrapped template.
    pub fn template(&self) -> &T {
        &self.template
    }

    /// Consume the wrapper and return the wrapped template.
    pub fn into_template(self) -> T {
        self.template
    }
}

impl<T: Template> View<T> {
    /// Render the wrapped template into a [`ViewResponse`].
    pub fn render(&self) -> Result<ViewResponse, ViewError> {
        self.template
            .render()
            .map(ViewResponse::html)
            .map_err(|source| ViewError::render("view", source))
    }
}

impl<T: Template> From<T> for View<T> {
    fn from(template: T) -> Self {
        Self::new(template)
    }
}

impl<T: Template> IntoResponse for View<T> {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(response) => response.into_response(),
            Err(error) => error.into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Hello;

    /// A rendered view carries the HTML content type and `200 OK`.
    #[test]
    fn view_response_sets_html_content_type() {
        let response = ViewResponse::html("<h1>hi</h1>").into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).expect("content-type"),
            HTML_CONTENT_TYPE
        );
    }

    /// A typed `View` renders through askama and HTML-escapes interpolated data.
    #[test]
    fn view_renders_and_auto_escapes() {
        let rendered = View::new(Hello {
            name: "<b>Ada</b>".to_string(),
        })
        .render()
        .expect("render view");

        assert!(rendered.body().contains("Ada"));
        assert!(
            !rendered.body().contains('<'),
            "auto-escaping must be ON by default"
        );
    }

    /// A `View<T>` is directly usable as an axum handler return type — the
    /// minimal integration point with `rustasea-http`/axum routers.
    #[tokio::test]
    async fn view_is_usable_as_axum_handler_response() {
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(|| async {
                View::new(Hello {
                    name: "Ada".to_string(),
                })
            }),
        );

        let request = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .expect("build request");
        let response = tower::ServiceExt::oneshot(app, request)
            .await
            .expect("dispatch request");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).expect("content-type"),
            HTML_CONTENT_TYPE
        );
    }
}
