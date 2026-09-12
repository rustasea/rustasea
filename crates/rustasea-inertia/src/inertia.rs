//! The Inertia service: shared props, partial filtering, and response building.

use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    HtmlShell, InertiaError, InertiaRequest, InertiaResponse, Page, PartialReload, RootView,
};

/// A per-request shared prop provider.
type SharedPropProvider = dyn Fn(&InertiaRequest) -> Value + Send + Sync;

/// Inertia service: builds page responses and owns shared props plus the root
/// view.
///
/// The service is cheap to clone (shared props and the root view are behind
/// [`Arc`]) so it can live in axum router state and be handed to the
/// [`crate::handle_inertia_requests`] middleware.
///
/// ```rust,ignore
/// let inertia = Inertia::new(env!("CARGO_PKG_VERSION"))
///     .share(|req| json!({ "url": req.url }));
///
/// let response = inertia.render("dashboard", DashboardProps { user }, &request);
/// ```
#[derive(Clone)]
pub struct Inertia {
    root_view: Arc<dyn RootView>,
    version: String,
    shared: Vec<Arc<SharedPropProvider>>,
}

impl Inertia {
    /// Create a service with the given asset version and the default root view.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            root_view: Arc::new(HtmlShell::default()),
            version: version.into(),
            shared: Vec::new(),
        }
    }

    /// Replace the root view used for non-Inertia (first-load) requests.
    pub fn with_root_view<R: RootView + 'static>(mut self, root_view: R) -> Self {
        self.root_view = Arc::new(root_view);
        self
    }

    /// Register a shared prop provider resolved on every render.
    ///
    /// Providers are merged in registration order; a page prop with the same
    /// name always wins over a shared prop.
    pub fn share<F>(mut self, provider: F) -> Self
    where
        F: Fn(&InertiaRequest) -> Value + Send + Sync + 'static,
    {
        self.shared.push(Arc::new(provider));
        self
    }

    /// The asset version this service advertises.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Render `component` with typed `props`, honoring the request contract.
    ///
    /// Returns a `409` response on a stale asset version, a JSON page for an
    /// Inertia request, or the HTML root document otherwise. Serialization and
    /// root-view failures surface as [`InertiaError`].
    pub fn try_render<T: Serialize>(
        &self,
        component: &str,
        props: T,
        request: &InertiaRequest,
    ) -> Result<InertiaResponse, InertiaError> {
        if request.version_mismatch(&self.version) {
            return Ok(InertiaResponse::version_conflict(&request.url));
        }

        let mut map = self.resolve_props(component, props, request)?;
        if let Some(partial) = request.partial_reload(component) {
            apply_partial(&mut map, &partial);
        }

        let page = Page {
            component: component.to_string(),
            props: Value::Object(map),
            url: request.url.clone(),
            version: self.version.clone(),
        };

        if request.is_inertia {
            Ok(InertiaResponse::json(&page))
        } else {
            Ok(InertiaResponse::html(self.root_view.render(&page)?))
        }
    }

    /// Render `component`, mapping any error to a generic `500` response.
    pub fn render<T: Serialize>(
        &self,
        component: &str,
        props: T,
        request: &InertiaRequest,
    ) -> InertiaResponse {
        self.try_render(component, props, request)
            .unwrap_or_else(InertiaResponse::from_error)
    }

    /// Serialize typed props to a JSON map and merge shared props underneath.
    fn resolve_props<T: Serialize>(
        &self,
        component: &str,
        props: T,
        request: &InertiaRequest,
    ) -> Result<Map<String, Value>, InertiaError> {
        let mut map =
            match serde_json::to_value(props).map_err(|source| InertiaError::SerializeProps {
                component: component.to_string(),
                source,
            })? {
                Value::Object(map) => map,
                _ => {
                    return Err(InertiaError::PropsNotObject {
                        component: component.to_string(),
                    })
                }
            };

        for provider in &self.shared {
            if let Value::Object(shared) = provider(request) {
                for (key, value) in shared {
                    // Page props win over shared props with the same name.
                    map.entry(key).or_insert(value);
                }
            }
        }

        Ok(map)
    }
}

/// Apply a partial-reload allow/deny filter to a prop map.
///
/// The allow-list runs first, then the deny-list, so `except` wins on overlap.
fn apply_partial(map: &mut Map<String, Value>, partial: &PartialReload) {
    if !partial.only.is_empty() {
        map.retain(|key, _| partial.only.iter().any(|allowed| allowed == key));
    }
    for key in &partial.except {
        map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InertiaResponse;
    use http::{HeaderMap, Method};
    use serde_json::json;

    /// Build an Inertia GET request from raw header pairs.
    fn request(pairs: &[(&str, &str)]) -> InertiaRequest {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                http::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
                http::HeaderValue::from_str(value).expect("header value"),
            );
        }
        InertiaRequest::from_parts(&Method::GET, &"/dashboard".parse().expect("uri"), &headers)
    }

    /// Parse a JSON page response body.
    fn page_body(response: &InertiaResponse) -> Page<Value> {
        serde_json::from_str(response.body()).expect("parse page body")
    }

    /// A full Inertia render carries every prop and the page envelope.
    #[test]
    fn renders_full_page_for_inertia_request() {
        let inertia = Inertia::new("v1");
        let response = inertia.render(
            "dashboard",
            json!({ "user": "Ada", "stats": 3 }),
            &request(&[("X-Inertia", "true"), ("X-Inertia-Version", "v1")]),
        );

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let page = page_body(&response);
        assert_eq!(page.component, "dashboard");
        assert_eq!(page.version, "v1");
        assert_eq!(page.props["user"], "Ada");
        assert_eq!(page.props["stats"], 3);
    }

    /// Shared props are merged underneath page props.
    #[test]
    fn shared_props_merge_and_page_props_win() {
        let inertia = Inertia::new("v1")
            .share(|req| json!({ "flash": "saved", "user": "shared", "url": req.url }));
        let response = inertia.render(
            "dashboard",
            json!({ "user": "page" }),
            &request(&[("X-Inertia", "true")]),
        );

        let page = page_body(&response);
        assert_eq!(page.props["flash"], "saved");
        assert_eq!(page.props["user"], "page");
        assert_eq!(page.props["url"], "/dashboard");
    }

    /// A partial reload keeps only the allow-listed props.
    #[test]
    fn partial_reload_filters_to_only() {
        let inertia = Inertia::new("v1");
        let response = inertia.render(
            "dashboard",
            json!({ "user": "Ada", "stats": 3, "secret": true }),
            &request(&[
                ("X-Inertia", "true"),
                ("X-Inertia-Partial-Component", "dashboard"),
                ("X-Inertia-Partial-Data", "user"),
            ]),
        );

        let page = page_body(&response);
        assert_eq!(page.props["user"], "Ada");
        assert!(page.props.get("stats").is_none());
        assert!(page.props.get("secret").is_none());
    }

    /// A partial reload removes the deny-listed props.
    #[test]
    fn partial_reload_applies_except() {
        let inertia = Inertia::new("v1");
        let response = inertia.render(
            "dashboard",
            json!({ "user": "Ada", "stats": 3 }),
            &request(&[
                ("X-Inertia", "true"),
                ("X-Inertia-Partial-Component", "dashboard"),
                ("X-Inertia-Partial-Except", "stats"),
            ]),
        );

        let page = page_body(&response);
        assert_eq!(page.props["user"], "Ada");
        assert!(page.props.get("stats").is_none());
    }

    /// A partial header for a different component does not filter.
    #[test]
    fn partial_component_mismatch_keeps_all_props() {
        let inertia = Inertia::new("v1");
        let response = inertia.render(
            "dashboard",
            json!({ "user": "Ada", "stats": 3 }),
            &request(&[
                ("X-Inertia", "true"),
                ("X-Inertia-Partial-Component", "users/index"),
                ("X-Inertia-Partial-Data", "user"),
            ]),
        );

        let page = page_body(&response);
        assert_eq!(page.props["stats"], 3);
    }

    /// A non-Inertia request renders the HTML root document.
    #[test]
    fn non_inertia_request_renders_html_shell() {
        let inertia = Inertia::new("v1");
        let response = inertia.render("dashboard", json!({ "user": "Ada" }), &request(&[]));

        assert_eq!(
            response.headers()[axum::http::header::CONTENT_TYPE],
            "text/html; charset=utf-8"
        );
        assert!(response.body().contains("data-page="));
        assert!(response.body().contains("&quot;component&quot;"));
    }

    /// A stale version yields `409` plus `X-Inertia-Location`.
    #[test]
    fn version_mismatch_returns_conflict_with_location() {
        let inertia = Inertia::new("v2");
        let response = inertia.render(
            "dashboard",
            json!({ "user": "Ada" }),
            &request(&[("X-Inertia", "true"), ("X-Inertia-Version", "v1")]),
        );

        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
        assert_eq!(response.headers()[crate::X_INERTIA_LOCATION], "/dashboard");
        assert!(response.body().is_empty());
    }

    /// Props that do not serialize to an object are a typed error.
    #[test]
    fn non_object_props_are_a_typed_error() {
        let inertia = Inertia::new("v1");
        let error = inertia
            .try_render(
                "dashboard",
                json!([1, 2, 3]),
                &request(&[("X-Inertia", "true")]),
            )
            .expect_err("array props must fail");

        match error {
            InertiaError::PropsNotObject { component } => assert_eq!(component, "dashboard"),
            other => panic!("expected PropsNotObject, got {other:?}"),
        }
    }
}
