//! The WASM-side Inertia client.

use http::{HeaderMap, HeaderName, HeaderValue};
use rustasea_inertia::{
    Page, X_INERTIA, X_INERTIA_LOCATION, X_INERTIA_PARTIAL_COMPONENT, X_INERTIA_PARTIAL_DATA,
    X_INERTIA_PARTIAL_EXCEPT, X_INERTIA_VERSION,
};

use crate::{ClientError, ComponentRegistry, Value};

/// Action the client should take after an Inertia response.
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationOutcome {
    /// Mount the returned page in place (SPA prop swap).
    Mount(Page<Value>),
    /// Hard-navigate to the location (asset-version mismatch).
    HardNavigate(String),
    /// The server returned HTML; perform a full document load.
    FullReload,
}

/// WASM-side Inertia client: parses pages and drives navigation decisions.
///
/// The client owns the protocol logic only; DOM mutation is delegated to the
/// registry's mount functions (provided by the generated app).
pub struct InertiaClient {
    registry: Box<dyn ComponentRegistry>,
    version: Option<String>,
    root_id: String,
}

impl InertiaClient {
    /// Create a client over `registry`, mounting into the `root_id` element.
    pub fn new(registry: Box<dyn ComponentRegistry>, root_id: impl Into<String>) -> Self {
        Self {
            registry,
            version: None,
            root_id: root_id.into(),
        }
    }

    /// Set the asset version sent with navigations.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// The mount element id.
    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    /// The configured asset version, if any.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Parse `page_json`, mount its component, and return the parsed page.
    ///
    /// `page_json` is either the JSON embedded in the root document's
    /// `data-page` attribute or the body of an Inertia JSON response.
    pub fn hydrate(&self, page_json: &str) -> Result<Page<Value>, ClientError> {
        let page: Page<Value> =
            serde_json::from_str(page_json).map_err(|source| ClientError::ParsePage { source })?;
        self.registry.mount(&page.component, &page.props)?;
        Ok(page)
    }

    /// Decide how to react to an Inertia HTTP response.
    ///
    /// A `409` yields [`NavigationOutcome::HardNavigate`]; a response marked
    /// `X-Inertia: true` is parsed and mounted; anything else is a
    /// [`NavigationOutcome::FullReload`].
    pub fn handle_response(
        &self,
        status: u16,
        headers: &HeaderMap,
        body: &str,
    ) -> Result<NavigationOutcome, ClientError> {
        if status == 409 {
            let location = headers
                .get(X_INERTIA_LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(ClientError::MissingLocation)?;
            return Ok(NavigationOutcome::HardNavigate(location.to_string()));
        }

        if is_inertia_response(headers) {
            let page: Page<Value> =
                serde_json::from_str(body).map_err(|source| ClientError::ParsePage { source })?;
            self.registry.mount(&page.component, &page.props)?;
            return Ok(NavigationOutcome::Mount(page));
        }

        Ok(NavigationOutcome::FullReload)
    }

    /// Build the headers for an Inertia visit to `component`.
    ///
    /// Partial headers are only attached when `only` or `except` is non-empty;
    /// an empty pair is a full visit.
    pub fn navigation_headers(&self, component: &str, only: &[&str], except: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(X_INERTIA, HeaderValue::from_static("true"));
        headers.insert(
            HeaderName::from_static("x-requested-with"),
            HeaderValue::from_static("XMLHttpRequest"),
        );

        if let Some(version) = &self.version {
            if let Ok(value) = HeaderValue::from_str(version) {
                headers.insert(X_INERTIA_VERSION, value);
            }
        }

        if !only.is_empty() || !except.is_empty() {
            if let Ok(value) = HeaderValue::from_str(component) {
                headers.insert(X_INERTIA_PARTIAL_COMPONENT, value);
            }
            if !only.is_empty() {
                if let Ok(value) = HeaderValue::from_str(&only.join(",")) {
                    headers.insert(X_INERTIA_PARTIAL_DATA, value);
                }
            }
            if !except.is_empty() {
                if let Ok(value) = HeaderValue::from_str(&except.join(",")) {
                    headers.insert(X_INERTIA_PARTIAL_EXCEPT, value);
                }
            }
        }

        headers
    }
}

/// Whether a response marks itself as an Inertia response.
fn is_inertia_response(headers: &HeaderMap) -> bool {
    headers
        .get(X_INERTIA)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inertia_registry;

    /// A mount function that succeeds only when `email` is present.
    fn mount_login(props: &Value) -> Result<(), ClientError> {
        if props.get("email").is_some() {
            Ok(())
        } else {
            Err(ClientError::Mount {
                component: "auth/login".to_string(),
                message: "missing email".to_string(),
            })
        }
    }

    inertia_registry!(TestRegistry {
        "auth/login" => mount_login,
    });

    /// Build a header map from raw pairs.
    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).expect("header name"),
                HeaderValue::from_str(value).expect("header value"),
            );
        }
        map
    }

    /// Build a client over the test registry.
    fn client() -> InertiaClient {
        InertiaClient::new(Box::new(TestRegistry::new()), "app").with_version("v1")
    }

    /// Hydrating a known page mounts its component.
    #[test]
    fn hydrate_mounts_known_component() {
        let page = client()
            .hydrate(r#"{"component":"auth/login","props":{"email":"a@b.c"},"url":"/login","version":"v1"}"#)
            .expect("hydrate");

        assert_eq!(page.component, "auth/login");
        assert_eq!(page.url, "/login");
    }

    /// An unmapped component is an `UnknownComponent` error.
    #[test]
    fn hydrate_rejects_unknown_component() {
        let error = client()
            .hydrate(r#"{"component":"missing","props":{},"url":"/","version":"v1"}"#)
            .expect_err("unknown component");

        match error {
            ClientError::UnknownComponent { component } => assert_eq!(component, "missing"),
            other => panic!("expected UnknownComponent, got {other:?}"),
        }
    }

    /// A malformed page body is a `ParsePage` error.
    #[test]
    fn hydrate_rejects_malformed_json() {
        let error = client().hydrate("not json").expect_err("malformed");
        assert!(matches!(error, ClientError::ParsePage { .. }));
    }

    /// A `409` response yields a hard navigation to the location header.
    #[test]
    fn conflict_response_hard_navigates() {
        let outcome = client()
            .handle_response(
                409,
                &headers(&[("X-Inertia-Location", "/dashboard?page=2")]),
                "",
            )
            .expect("handle conflict");

        assert_eq!(
            outcome,
            NavigationOutcome::HardNavigate("/dashboard?page=2".to_string())
        );
    }

    /// A `409` without the location header is an error.
    #[test]
    fn conflict_response_requires_location() {
        let error = client()
            .handle_response(409, &HeaderMap::new(), "")
            .expect_err("missing location");

        assert!(matches!(error, ClientError::MissingLocation));
    }

    /// A `200` Inertia response mounts the returned page.
    #[test]
    fn inertia_response_mounts_page() {
        let outcome = client()
            .handle_response(
                200,
                &headers(&[("X-Inertia", "true")]),
                r#"{"component":"auth/login","props":{"email":"a@b.c"},"url":"/login","version":"v1"}"#,
            )
            .expect("handle page");

        match outcome {
            NavigationOutcome::Mount(page) => assert_eq!(page.component, "auth/login"),
            other => panic!("expected Mount, got {other:?}"),
        }
    }

    /// A `200` response without the Inertia marker is a full reload.
    #[test]
    fn plain_response_triggers_full_reload() {
        let outcome = client()
            .handle_response(200, &HeaderMap::new(), "<html></html>")
            .expect("handle html");

        assert_eq!(outcome, NavigationOutcome::FullReload);
    }

    /// Navigation headers carry the marker, version, and partial allow-list.
    #[test]
    fn navigation_headers_include_partial_contract() {
        let headers = client().navigation_headers("auth/login", &["email"], &[]);

        assert_eq!(headers[X_INERTIA], "true");
        assert_eq!(headers[X_INERTIA_VERSION], "v1");
        assert_eq!(headers[X_INERTIA_PARTIAL_COMPONENT], "auth/login");
        assert_eq!(headers[X_INERTIA_PARTIAL_DATA], "email");
        assert!(headers.get(X_INERTIA_PARTIAL_EXCEPT).is_none());
    }

    /// A full visit omits the partial headers.
    #[test]
    fn navigation_headers_omit_partial_when_empty() {
        let headers = client().navigation_headers("auth/login", &[], &[]);

        assert!(headers.get(X_INERTIA_PARTIAL_COMPONENT).is_none());
        assert!(headers.get(X_INERTIA_PARTIAL_DATA).is_none());
        assert!(headers.get(X_INERTIA_PARTIAL_EXCEPT).is_none());
    }
}
