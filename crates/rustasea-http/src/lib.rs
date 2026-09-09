//! RustaSea HTTP layer — AppState, JSON helpers, middleware stubs, and HTTP client.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json as AxumJson;
use serde::Serialize;
use serde_json::Value;

/// Security posture consumed by HTTP middleware.
///
/// Plain data only — the enforcing types (PreventRequestForgery, Throttle)
/// live in `rustasea-auth`, which depends on this crate; keeping the config
/// data here preserves the DAG (AUTH -> HTTP) without cycles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityConfig {
    /// Origin allow-list for CSRF (`config.app.csrf_origins`).
    pub csrf_origins: Vec<String>,
    /// Trusted proxy CIDRs; `X-Forwarded-For` is ignored until non-empty.
    pub trusted_proxies: Vec<String>,
}

impl SecurityConfig {
    /// Create a security config from a CSRF origin allow-list.
    pub fn with_csrf_origins(origins: Vec<String>) -> Self {
        Self {
            csrf_origins: origins,
            trusted_proxies: Vec::new(),
        }
    }

    /// Mark proxies as trusted so forwarded identities are honored.
    pub fn behind_proxies(mut self, proxies: Vec<String>) -> Self {
        self.trusted_proxies = proxies;
        self
    }
}

/// Shared application state passed to handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Application environment name.
    pub env: String,
    /// Debug flag.
    pub debug: bool,
    /// Security posture (CSRF origins, trusted proxies).
    pub security: SecurityConfig,
}

impl AppState {
    /// Create a new AppState.
    pub fn new(env: impl Into<String>, debug: bool) -> Self {
        Self {
            env: env.into(),
            debug,
            security: SecurityConfig::default(),
        }
    }

    /// Attach the security posture.
    pub fn with_security(mut self, security: SecurityConfig) -> Self {
        self.security = security;
        self
    }

    /// Convenience accessor for the CSRF origin allow-list.
    pub fn csrf_origins(&self) -> &[String] {
        &self.security.csrf_origins
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new("production", false)
    }
}

/// Helper to build JSON responses with status code.
pub struct JsonResponse;

impl JsonResponse {
    /// Build a 200 JSON response from a serializable value.
    pub fn ok<T: Serialize>(data: T) -> Response {
        (StatusCode::OK, AxumJson(data)).into_response()
    }

    /// Build a JSON response with custom status.
    pub fn with_status<T: Serialize>(status: StatusCode, data: T) -> Response {
        (status, AxumJson(data)).into_response()
    }

    /// Build a 422 validation error response.
    pub fn validation_error(errors: Value) -> Response {
        (StatusCode::UNPROCESSABLE_ENTITY, AxumJson(errors)).into_response()
    }
}

/// CORS middleware configuration stub.
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins.
    pub allowed_origins: Vec<String>,
    /// Allow credentials.
    pub allow_credentials: bool,
}

impl CorsConfig {
    /// Create CORS config with allowed origins.
    pub fn new(origins: Vec<String>) -> Self {
        Self {
            allowed_origins: origins,
            allow_credentials: false,
        }
    }

    /// Build the tower-http CORS layer.
    ///
    /// The layer composes onto any tower/axum stack, so routers apply it
    /// directly:
    ///
    /// ```rust,ignore
    /// use rustasea_http::CorsConfig;
    /// let layer = CorsConfig::default().layer();
    /// // router.layer(layer); — rustasea::router::Router::layer()
    /// ```
    ///
    /// Allowed origins map to an explicit AllowOrigin::list; when none are
    /// configured the layer stays restrictive (no origins allowed) instead of
    /// silently becoming permissive — opt into permissive explicitly.
    pub fn layer(&self) -> tower_http::cors::CorsLayer {
        use tower_http::cors::AllowOrigin;
        use tower_http::cors::CorsLayer;

        let mut layer = CorsLayer::new();
        if !self.allowed_origins.is_empty() {
            let origins: Vec<_> = self
                .allowed_origins
                .iter()
                .map(|o| o.parse::<axum::http::HeaderValue>())
                .filter_map(Result::ok)
                .collect();
            layer = layer.allow_origin(AllowOrigin::list(origins));
        }
        if self.allow_credentials {
            layer = layer.allow_credentials(true);
        }
        layer
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Throttle / rate-limit middleware configuration.
///
/// The enforcing `MemoryRateLimiter` + tower middleware live in
/// `rustasea-auth` (M3). This config type is the HTTP-layer representation
/// parsed from `#[middleware("throttle:60,1")]` specs before delegation.
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    /// Max requests per window.
    pub max_requests: u32,
    /// Window duration in seconds.
    pub window_secs: u64,
    /// Key bucket: by_ip or by_user.
    pub bucket: String,
}

impl ThrottleConfig {
    /// Create a new throttle config.
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            bucket: "by_ip".to_string(),
        }
    }

    /// Set bucket key to by_ip.
    pub fn by_ip(mut self) -> Self {
        self.bucket = "by_ip".to_string();
        self
    }

    /// Set bucket key to by_user.
    pub fn by_user(mut self) -> Self {
        self.bucket = "by_user".to_string();
        self
    }
}

/// Timeout classification for the HTTP client (#18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutKind {
    /// Connection establishment exceeded the timeout.
    Connect,
    /// Total request time exceeded the timeout.
    Total,
    /// No bytes received within the idle timeout.
    Idle,
}

/// Typed error returned by [`HttpClient::send`].
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// The `throw` predicate matched the response status.
    #[error("upstream {code} matched throw predicate")]
    Status { code: u16 },
    /// A request timeout fired; kind distinguishes connect/total/idle.
    #[error("http client timed out ({kind:?})")]
    Timeout { kind: TimeoutKind },
    /// The `throw` predicate itself failed.
    #[error("throw callback failed: {source}")]
    ThrowCallback {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Transport-level failure surfaced by reqwest.
    #[error("http transport error: {source}")]
    Transport { source: reqwest::Error },
}

/// Result of evaluating a `throw` predicate; an `Err` aborts the send as
/// [`HttpError::ThrowCallback`].
type ThrowOutcome = Result<bool, Box<dyn std::error::Error + Send + Sync>>;

/// Shared `throw` callback boxed signature.
type ThrowCallback = dyn Fn(&reqwest::Response) -> ThrowOutcome + Send + Sync;

/// Thin wrapper around reqwest for outgoing HTTP calls.
///
/// Laravel-style builder: `.timeout(Duration)` configures the total request
/// budget, `.throw(predicate)` maps a matching status to
/// [`HttpError::Status`], and `.send().await` returns
/// `Result<reqwest::Response, HttpError>`.
pub struct HttpClient {
    /// Per-request timeout budget.
    pub timeout: std::time::Duration,
    /// Optional idle timeout (inter-byte silence).
    pub idle_timeout: Option<std::time::Duration>,
    /// Optional `throw` predicate evaluated against the response.
    throw: Option<Box<ThrowCallback>>,
}

impl HttpClient {
    /// Create a new HTTP client with a 30s default timeout.
    pub fn new() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
            idle_timeout: None,
            throw: None,
        }
    }

    /// Set the total request timeout.
    ///
    /// ```rust
    /// # use std::time::Duration;
    /// use rustasea_http::HttpClient;
    /// let client = HttpClient::new().timeout(Duration::from_secs(30));
    /// # let _ = client;
    /// ```
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the idle (inter-byte) timeout.
    pub fn idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// Install a `throw` predicate: when it returns `true` for the response
    /// status, [`send`](Self::send) yields [`HttpError::Status`].
    ///
    /// ```rust
    /// # use std::time::Duration;
    /// use rustasea_http::HttpClient;
    /// let client = HttpClient::new()
    ///     .timeout(Duration::from_secs(30))
    ///     .throw(|resp| resp.status().is_success());
    /// # let _ = client;
    /// ```
    pub fn throw<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&reqwest::Response) -> bool + Send + Sync + 'static,
    {
        self.throw = Some(Box::new(move |resp| Ok(predicate(resp))));
        self
    }

    /// Install a fallible `throw` callback returning `Result<bool, E>`.
    ///
    /// An `Err` from the callback surfaces as [`HttpError::ThrowCallback`];
    /// an `Ok(true)` match surfaces as [`HttpError::Status`].
    pub fn try_throw<F, E>(mut self, callback: F) -> Self
    where
        F: Fn(&reqwest::Response) -> Result<bool, E> + Send + Sync + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.throw = Some(Box::new(move |resp| {
            callback(resp).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }));
        self
    }

    /// Send a prepared reqwest request through this client's policy.
    ///
    /// Applies the configured total timeout, then evaluates the `throw`
    /// predicate — a `true` match becomes [`HttpError::Status`].
    pub async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, HttpError> {
        self.send_with(request, reqwest::Client::new()).await
    }

    /// Send a prepared request through an explicit reqwest client.
    pub async fn send_with(
        &self,
        request: reqwest::RequestBuilder,
        client: reqwest::Client,
    ) -> Result<reqwest::Response, HttpError> {
        // reqwest needs a tokio runtime; building per-send keeps the wrapper
        // clone-free. Map request-build errors straight to Transport.
        let request = request
            .timeout(self.timeout)
            .build()
            .map_err(|e| HttpError::Transport { source: e })?;
        let response = client.execute(request).await.map_err(map_timeout)?;
        if let Some(throw) = &self.throw {
            let matched = throw(&response).map_err(|source| HttpError::ThrowCallback { source })?;
            if matched {
                return Err(HttpError::Status {
                    code: response.status().as_u16(),
                });
            }
        }
        Ok(response)
    }

    /// Convenience: perform a GET request with this client's policy.
    pub async fn get(&self, url: &str) -> Result<reqwest::Response, HttpError> {
        self.send(reqwest::Client::new().get(url)).await
    }

    /// Convenience: perform a POST request with this client's policy.
    pub async fn post(&self, url: &str) -> Result<reqwest::Response, HttpError> {
        self.send(reqwest::Client::new().post(url)).await
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Classify a reqwest transport error into a typed error.
///
/// Connection-phase failures map to [`TimeoutKind::Connect`] and total-budget
/// expiry to [`TimeoutKind::Total`]. Idle (inter-byte) timeout enforcement
/// requires a body-stream watcher and lands with the M3 pass; the builder
/// already carries the idle budget so call sites are stable.
fn map_timeout(error: reqwest::Error) -> HttpError {
    if error.is_connect() {
        HttpError::Timeout {
            kind: TimeoutKind::Connect,
        }
    } else if error.is_timeout() {
        HttpError::Timeout {
            kind: TimeoutKind::Total,
        }
    } else {
        HttpError::Transport { source: error }
    }
}
