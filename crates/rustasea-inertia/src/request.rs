//! Inertia request metadata parsed from the `X-Inertia*` headers.

use http::{HeaderMap, Method, Uri};

use crate::{
    X_INERTIA, X_INERTIA_PARTIAL_COMPONENT, X_INERTIA_PARTIAL_DATA, X_INERTIA_PARTIAL_EXCEPT,
    X_INERTIA_RESET, X_INERTIA_VERSION,
};

/// Prop allow/deny filter derived from a partial-reload request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartialReload {
    /// Prop names the client asked to receive (`X-Inertia-Partial-Data`).
    pub only: Vec<String>,
    /// Prop names the client asked to omit (`X-Inertia-Partial-Except`).
    pub except: Vec<String>,
}

/// Inertia-specific request metadata parsed from headers and the request line.
///
/// The type is transport-agnostic (built from [`http`] primitives) so it can be
/// constructed in tests without an axum router. With the `server` feature it
/// also implements [`axum::extract::FromRequestParts`], letting a handler take
/// it as an extractor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InertiaRequest {
    /// Whether `X-Inertia: true` marked this as an Inertia request.
    pub is_inertia: bool,
    /// Client asset version (`X-Inertia-Version`), if sent.
    pub version: Option<String>,
    /// Component targeted by a partial reload (`X-Inertia-Partial-Component`).
    pub partial_component: Option<String>,
    /// Prop allow-list (`X-Inertia-Partial-Data`).
    pub partial_data: Vec<String>,
    /// Prop deny-list (`X-Inertia-Partial-Except`).
    pub partial_except: Vec<String>,
    /// Client-side reset list (`X-Inertia-Reset`); not applied on the server.
    pub reset: Vec<String>,
    /// HTTP method of the request.
    pub method: Method,
    /// Request URL (path + query) echoed into the page envelope.
    pub url: String,
}

impl InertiaRequest {
    /// Parse Inertia metadata from the method, URI, and headers.
    pub fn from_parts(method: &Method, uri: &Uri, headers: &HeaderMap) -> Self {
        Self {
            is_inertia: header(headers, X_INERTIA)
                .is_some_and(|value| value.eq_ignore_ascii_case("true")),
            version: header(headers, X_INERTIA_VERSION).map(str::to_string),
            partial_component: header(headers, X_INERTIA_PARTIAL_COMPONENT).map(str::to_string),
            partial_data: csv_header(headers, X_INERTIA_PARTIAL_DATA),
            partial_except: csv_header(headers, X_INERTIA_PARTIAL_EXCEPT),
            reset: csv_header(headers, X_INERTIA_RESET),
            method: method.clone(),
            url: uri
                .path_and_query()
                .map(|value| value.as_str().to_string())
                .unwrap_or_else(|| "/".to_string()),
        }
    }

    /// Whether this request carries a stale asset version.
    ///
    /// Only `GET` Inertia requests are checked: a non-`GET` mutation must run
    /// even when the client's asset version is behind (ADR-0002 §7). A missing
    /// `X-Inertia-Version` is treated as "no opinion" and never conflicts.
    pub fn version_mismatch(&self, current: &str) -> bool {
        self.is_inertia
            && self.method == Method::GET
            && self
                .version
                .as_deref()
                .is_some_and(|version| version != current)
    }

    /// Resolve the partial-reload filter for `component`, if any applies.
    ///
    /// Filtering is only honored for an Inertia request whose
    /// `X-Inertia-Partial-Component` matches the component being rendered; a
    /// mismatch (or an empty allow/deny pair) yields `None`, i.e. full props.
    pub fn partial_reload(&self, component: &str) -> Option<PartialReload> {
        if !self.is_inertia || self.partial_component.as_deref() != Some(component) {
            return None;
        }
        if self.partial_data.is_empty() && self.partial_except.is_empty() {
            return None;
        }
        Some(PartialReload {
            only: self.partial_data.clone(),
            except: self.partial_except.clone(),
        })
    }
}

/// Read a header as a trimmed UTF-8 string.
fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// Parse a comma-separated header into trimmed, non-empty values.
fn csv_header(headers: &HeaderMap, name: &str) -> Vec<String> {
    header(headers, name)
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(feature = "server")]
#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for InertiaRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    /// Build the metadata from the request parts; never fails.
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_parts(&parts.method, &parts.uri, &parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    /// Build a header map from a list of pairs.
    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                http::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
                HeaderValue::from_str(value).expect("header value"),
            );
        }
        map
    }

    /// A plain request is not an Inertia request.
    #[test]
    fn plain_request_is_not_inertia() {
        let request = InertiaRequest::from_parts(
            &Method::GET,
            &"/dashboard".parse().expect("uri"),
            &HeaderMap::new(),
        );

        assert!(!request.is_inertia);
        assert_eq!(request.url, "/dashboard");
        assert!(request.partial_data.is_empty());
    }

    /// The marker header and comma-separated partial headers are parsed.
    #[test]
    fn parses_marker_and_partial_headers() {
        let request = InertiaRequest::from_parts(
            &Method::GET,
            &"/users?page=2".parse().expect("uri"),
            &headers(&[
                ("X-Inertia", "true"),
                ("X-Inertia-Version", "v2"),
                ("X-Inertia-Partial-Component", "users/index"),
                ("X-Inertia-Partial-Data", "users, roles ,"),
                ("X-Inertia-Partial-Except", "stats"),
            ]),
        );

        assert!(request.is_inertia);
        assert_eq!(request.version.as_deref(), Some("v2"));
        assert_eq!(request.partial_component.as_deref(), Some("users/index"));
        assert_eq!(request.partial_data, vec!["users", "roles"]);
        assert_eq!(request.partial_except, vec!["stats"]);
        assert_eq!(request.url, "/users?page=2");
    }

    /// A partial filter only applies when the component matches.
    #[test]
    fn partial_reload_requires_component_match() {
        let request = InertiaRequest::from_parts(
            &Method::GET,
            &"/users".parse().expect("uri"),
            &headers(&[
                ("X-Inertia", "true"),
                ("X-Inertia-Partial-Component", "users/index"),
                ("X-Inertia-Partial-Data", "users"),
            ]),
        );

        assert!(request.partial_reload("users/index").is_some());
        assert!(request.partial_reload("dashboard").is_none());
    }

    /// A stale version conflicts only for `GET` Inertia requests.
    #[test]
    fn version_mismatch_only_for_get_inertia() {
        let stale = InertiaRequest::from_parts(
            &Method::GET,
            &"/dashboard".parse().expect("uri"),
            &headers(&[("X-Inertia", "true"), ("X-Inertia-Version", "old")]),
        );
        assert!(stale.version_mismatch("new"));
        assert!(!stale.version_mismatch("old"));

        let post = InertiaRequest::from_parts(
            &Method::POST,
            &"/dashboard".parse().expect("uri"),
            &headers(&[("X-Inertia", "true"), ("X-Inertia-Version", "old")]),
        );
        assert!(!post.version_mismatch("new"));

        let missing = InertiaRequest::from_parts(
            &Method::GET,
            &"/dashboard".parse().expect("uri"),
            &headers(&[("X-Inertia", "true")]),
        );
        assert!(!missing.version_mismatch("new"));
    }
}
