/// Origin-aware CSRF protection — `PreventRequestForgery`.
///
/// Decision table (token-first, then `Sec-Fetch-Site`):
///
/// | Sec-Fetch-Site | Origin            | Result                          |
/// |----------------|-------------------|---------------------------------|
/// | same-origin    | any               | pass (after token)              |
/// | cross-site     | in allow-list     | pass                            |
/// | cross-site     | not in allow-list | `403 CsrfError::UntrustedOrigin` |
/// | none           | (direct nav)      | treated as same-origin          |
/// | missing        | (older browser)   | degrades to token-only          |
///
/// `GET`/`HEAD`/`OPTIONS` are exempt and never require a token.
use std::sync::Arc;

use crate::error::CsrfError;

/// HTTP methods exempt from forgery protection.
pub const SAFE_METHODS: [&str; 3] = ["GET", "HEAD", "OPTIONS"];

/// Default header carrying the CSRF token.
pub const CSRF_HEADER: &str = "x-csrf-token";

/// Default form field carrying the CSRF token.
pub const CSRF_FIELD: &str = "_token";

/// Parsed `Sec-Fetch-Site` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecFetchSite {
    /// Same origin as the target.
    SameOrigin,
    /// Cross-site navigation/submission.
    CrossSite,
    /// Direct navigation (address bar) — treated as same-origin.
    None,
}

impl SecFetchSite {
    /// Parse from the `Sec-Fetch-Site` header value.
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value?.trim().to_ascii_lowercase().as_str() {
            "same-origin" => Some(Self::SameOrigin),
            "cross-site" => Some(Self::CrossSite),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

/// A single request header bag (HTTP-agnostic for unit testing).
#[derive(Debug, Clone, Default)]
pub struct RequestHeaders {
    /// HTTP method (upper-case).
    pub method: String,
    /// CSRF token from `X-CSRF-TOKEN` or `_token` field.
    pub csrf_token: Option<String>,
    /// Raw `Sec-Fetch-Site` header value.
    pub sec_fetch_site: Option<String>,
    /// Raw `Origin` header value.
    pub origin: Option<String>,
}

impl RequestHeaders {
    /// Build from an axum request — cheap extraction of the headers we gate.
    pub fn from_request(req: &axum::http::Request<axum::body::Body>) -> Self {
        let method = req.method().as_str().to_string();
        let csrf_token = req
            .headers()
            .get(CSRF_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let sec_fetch_site = req
            .headers()
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let origin = req
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Self {
            method,
            csrf_token,
            sec_fetch_site,
            origin,
        }
    }

    /// Whether this is a safe method (exempt from forgery protection).
    pub fn is_safe_method(&self) -> bool {
        SAFE_METHODS.contains(&self.method.to_ascii_uppercase().as_str())
    }
}

/// Origin-aware forgery-protection policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreventRequestForgery {
    /// Origin allow-list from `config.app.csrf_origins`.
    pub allowed_origins: Vec<String>,
    /// Static token the request must carry (session-bound in production).
    ///
    /// A per-request `token_provider` takes precedence when set; see
    /// `CsrfLayer::with_token_provider`.
    pub expected_token: Option<String>,
}

impl PreventRequestForgery {
    /// Build the middleware policy from an origin allow-list.
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins,
            expected_token: None,
        }
    }

    /// Attach the expected CSRF token (session-bound).
    pub fn with_expected_token(mut self, token: impl Into<String>) -> Self {
        self.expected_token = Some(token.into());
        self
    }

    /// Run the full decision table against a request.
    ///
    /// Safe methods pass. The request must carry `expected_token`; when the
    /// policy holds no expected token the check fails closed with
    /// `TokenMismatch`. A cross-site request must additionally present an
    /// origin inside the allow-list.
    pub fn check(&self, req: &RequestHeaders) -> std::result::Result<(), CsrfError> {
        if req.is_safe_method() {
            return Ok(());
        }
        // Token first — the primary gate (R-03 spec-divergence guard).
        let provided = req.csrf_token.as_deref().unwrap_or("");
        match &self.expected_token {
            Some(expected) if !expected.is_empty() && expected == provided => {}
            // Missing expected token or mismatch both fail closed.
            _ => return Err(CsrfError::TokenMismatch),
        }
        // Sec-Fetch-Site secondary gate; missing degrades to token-only.
        match SecFetchSite::parse(req.sec_fetch_site.as_deref()) {
            None | Some(SecFetchSite::SameOrigin) | Some(SecFetchSite::None) => Ok(()),
            Some(SecFetchSite::CrossSite) => {
                let origin = req.origin.clone().unwrap_or_default();
                if self.allowed_origins.iter().any(|o| o == &origin) {
                    Ok(())
                } else {
                    Err(CsrfError::UntrustedOrigin { origin })
                }
            }
        }
    }
}

/// Tower layer applying `PreventRequestForgery` to every request.
///
/// The expected token comes from the layer's `token_provider` when one is
/// configured (resolved inside every service `call`, so a fresh session
/// token is honored per request); `policy.expected_token` applies only when
/// no provider is configured.
#[derive(Clone)]
pub struct CsrfLayer {
    /// Policy evaluated per request (origin allow-list + static token).
    pub policy: PreventRequestForgery,
    /// Per-request expected-token provider (session-bound). Takes precedence
    /// over `policy.expected_token` when set.
    pub token_provider: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
}

impl std::fmt::Debug for CsrfLayer {
    /// Manual debug — closures cannot be formatted.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CsrfLayer")
            .field("policy", &self.policy)
            .field("token_provider", &self.token_provider.is_some())
            .finish()
    }
}

impl CsrfLayer {
    /// Create a layer from an origin allow-list.
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            policy: PreventRequestForgery::new(allowed_origins),
            token_provider: None,
        }
    }

    /// Attach a static expected CSRF token to the underlying policy.
    ///
    /// Used when the expected token does not vary per request; a
    /// `token_provider` set via [`CsrfLayer::with_token_provider`] takes
    /// precedence over this static token.
    pub fn with_expected_token(mut self, token: impl Into<String>) -> Self {
        self.policy = self.policy.with_expected_token(token);
        self
    }

    /// Resolve the expected CSRF token per request from `provider`.
    ///
    /// Takes precedence over a static `policy.expected_token`; the static
    /// token still applies when no provider is set. Returning `None` makes
    /// the layer fail closed (`403 TokenMismatch`) for state-changing
    /// requests.
    pub fn with_token_provider(
        mut self,
        provider: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    ) -> Self {
        self.token_provider = Some(provider);
        self
    }
}

impl<S> tower::Layer<S> for CsrfLayer {
    type Service = CsrfService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CsrfService {
            inner,
            policy: self.policy.clone(),
            token_provider: self.token_provider.clone(),
        }
    }
}

/// Middleware service enforcing the CSRF decision table.
#[derive(Clone)]
pub struct CsrfService<S> {
    inner: S,
    policy: PreventRequestForgery,
    /// Per-request expected-token provider; `None` falls back to the static
    /// `policy.expected_token`.
    token_provider: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
}

impl<S> std::fmt::Debug for CsrfService<S> {
    /// Manual debug — closures and the inner service are opaque.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CsrfService")
            .field("policy", &self.policy)
            .field("token_provider", &self.token_provider.is_some())
            .field("inner", &std::any::type_name::<S>())
            .finish()
    }
}

impl<S> tower::Service<axum::http::Request<axum::body::Body>> for CsrfService<S>
where
    S: tower::Service<axum::http::Request<axum::body::Body>, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<Self::Response, Self::Error>>
                + Send,
        >,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<axum::body::Body>) -> Self::Future {
        // Resolve the expected token now, per request: a session token issued
        // after the layer was built must be honored on the next call. When a
        // provider is set its value takes precedence over the static policy
        // token; a `None` result fails closed below (no expected token).
        let mut policy = self.policy.clone();
        if let Some(provider) = &self.token_provider {
            policy.expected_token = provider();
        }
        let outcome = policy.check(&RequestHeaders::from_request(&req));
        if let Err(err) = outcome {
            return Box::pin(async move { Ok(csrf_error_response(&err)) });
        }
        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}

/// Build the `403` JSON error envelope for a CSRF failure.
pub fn csrf_error_response(err: &CsrfError) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::Json;

    let (code, title, detail, meta) = match err {
        CsrfError::TokenMismatch => (
            err.code(),
            "CSRF token mismatch",
            "The provided CSRF token does not match the session token.".to_string(),
            serde_json::Value::Null,
        ),
        CsrfError::UntrustedOrigin { origin } => (
            err.code(),
            "Untrusted origin",
            format!("Origin {origin} is not in the csrf allow-list."),
            serde_json::json!({ "sec_fetch_site": "cross-site", "origin": origin }),
        ),
    };
    let mut error = serde_json::json!({
        "status": "403",
        "code": code,
        "title": title,
        "detail": detail,
    });
    if !meta.is_null() {
        error["meta"] = meta;
    }
    let body = serde_json::json!({ "errors": [error] });
    (StatusCode::FORBIDDEN, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn headers(
        method: &str,
        token: Option<&str>,
        site: Option<&str>,
        origin: Option<&str>,
    ) -> RequestHeaders {
        RequestHeaders {
            method: method.to_string(),
            csrf_token: token.map(|s| s.to_string()),
            sec_fetch_site: site.map(|s| s.to_string()),
            origin: origin.map(|s| s.to_string()),
        }
    }

    /// Full decision-table sweep for the CSRF matrix (6 rows).
    #[test]
    fn csrf_decision_matrix() {
        let policy = PreventRequestForgery::new(vec!["https://app.example.com".into()])
            .with_expected_token("tok-123");

        // Same-origin + valid token passes.
        assert_eq!(
            policy.check(&headers(
                "POST",
                Some("tok-123"),
                Some("same-origin"),
                Some("https://app.example.com")
            )),
            Ok(())
        );
        // Cross-site + trusted origin passes.
        assert_eq!(
            policy.check(&headers(
                "POST",
                Some("tok-123"),
                Some("cross-site"),
                Some("https://app.example.com")
            )),
            Ok(())
        );
        // Cross-site + untrusted origin -> UntrustedOrigin even with valid token.
        assert_eq!(
            policy.check(&headers(
                "POST",
                Some("tok-123"),
                Some("cross-site"),
                Some("https://evil.com")
            )),
            Err(CsrfError::UntrustedOrigin {
                origin: "https://evil.com".into()
            })
        );
        // Missing Sec-Fetch-Site degrades to token-only.
        assert_eq!(
            policy.check(&headers("POST", Some("tok-123"), None, None)),
            Ok(())
        );
        // none (direct navigation) treated as same-origin.
        assert_eq!(
            policy.check(&headers("POST", Some("tok-123"), Some("none"), None)),
            Ok(())
        );
        // Bad token rejected regardless of site.
        assert_eq!(
            policy.check(&headers("POST", Some("wrong"), Some("same-origin"), None)),
            Err(CsrfError::TokenMismatch)
        );
    }

    /// Safe methods are exempt and never need a token.
    #[test]
    fn safe_methods_are_exempt() {
        let policy = PreventRequestForgery::new(Vec::new()).with_expected_token("tok-123");
        for method in SAFE_METHODS {
            assert_eq!(
                policy.check(&headers(
                    method,
                    None,
                    Some("cross-site"),
                    Some("https://evil.com")
                )),
                Ok(())
            );
        }
    }

    /// Without any expected token every state-changing request fails closed
    /// with `TokenMismatch` (never silently passes).
    #[test]
    fn missing_expected_token_fails_closed() {
        let policy = PreventRequestForgery::new(Vec::new());
        // No static token -> fail closed even when a token is provided.
        assert_eq!(
            policy.check(&headers("POST", Some("tok-123"), Some("same-origin"), None)),
            Err(CsrfError::TokenMismatch)
        );
        // Empty static token also fails closed.
        let empty = PreventRequestForgery::new(Vec::new()).with_expected_token("");
        assert_eq!(
            empty.check(&headers("POST", Some("tok-123"), Some("same-origin"), None)),
            Err(CsrfError::TokenMismatch)
        );
    }

    /// The tower service resolves the provider per request, so a token that
    /// appears after the first call is honored on the second.
    #[tokio::test]
    async fn tower_service_resolves_token_per_request() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::{Layer, Service, ServiceExt};

        // A handler echo service stands in for the application router.
        let inner = tower::service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(axum::response::Response::new(Body::empty()))
        });
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        // Provider: no token on the first call (session not yet established),
        // session token `abc` from the second call onward.
        let layer = CsrfLayer::new(Vec::new())
            .with_expected_token("static-token")
            .with_token_provider(Arc::new(move || {
                let _ = Arc::clone(&counter);
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    None
                } else {
                    Some("abc".to_string())
                }
            }));
        let mut svc = layer.layer(inner);

        // First POST — provider has no token yet: it overrides the static
        // token, so the request fails closed (403).
        let req = Request::builder()
            .method("POST")
            .header("x-csrf-token", "static-token")
            .body(Body::empty())
            .expect("request builds");
        let resp = svc
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("call succeeds");
        assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);

        // Second POST — provider now yields `abc`: it passes through to the
        // app even though the static token differs.
        let req = Request::builder()
            .method("POST")
            .header("x-csrf-token", "abc")
            .body(Body::empty())
            .expect("request builds");
        let resp = svc
            .ready()
            .await
            .expect("service ready")
            .call(req)
            .await
            .expect("call succeeds");
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }
}
