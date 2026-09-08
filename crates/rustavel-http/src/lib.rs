//! Rustavel HTTP layer — AppState, JSON helpers, middleware stubs, and HTTP client.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json as AxumJson;
use serde::Serialize;
use serde_json::Value;

/// Security posture consumed by HTTP middleware.
///
/// Plain data only — the enforcing types (PreventRequestForgery, Throttle)
/// live in `rustavel-auth`, which depends on this crate; keeping the config
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
/// `rustavel-auth` (M3). This config type is the HTTP-layer representation
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

/// Thin wrapper around reqwest for outgoing HTTP calls.
pub struct HttpClient {
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Idle timeout in milliseconds.
    pub idle_timeout_ms: Option<u64>,
    /// Retry attempts.
    pub retries: u32,
}

impl HttpClient {
    /// Create a new HTTP client with defaults.
    pub fn new() -> Self {
        Self {
            timeout_ms: 30_000,
            idle_timeout_ms: None,
            retries: 0,
        }
    }

    /// Set request timeout.
    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Set idle timeout.
    pub fn idle_timeout(mut self, ms: u64) -> Self {
        self.idle_timeout_ms = Some(ms);
        self
    }

    /// Set retry count.
    pub fn retry(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Throw callback stub — validates response status.
    pub fn throw<F>(&self, _predicate: F) -> &Self
    where
        F: Fn(u16) -> bool,
    {
        self
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
