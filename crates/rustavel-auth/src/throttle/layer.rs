/// Tower middleware enforcing a rate limit per request (FS-M3-04/FR-306).
///
/// [`ThrottleLayer`] wraps any service whose request type is an axum
/// `http::Request<Body>`; [`ThrottleService`] resolves the bucket key from
/// the peer address (honoring `X-Forwarded-For` only behind trusted
/// proxies), asks the shared limiter for one hit, and either forwards the
/// request or answers `429 Too Many Requests` with a `Retry-After` header
/// carrying the seconds until the window resets.
use std::convert::Infallible;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use super::limiter::MemoryRateLimiter;
use super::{KeyBy, Limit, RateLimiter, ThrottleConfig, ThrottleDecision};

/// JSON envelope code for throttle rejections (api-auth.md §4).
const THROTTLE_CODE: &str = "Throttle";

/// Tower layer applying one rate limit to every wrapped request.
///
/// Cloning the layer is cheap: the limiter is shared through an `Arc`, so a
/// stack of layers built from the same limiter observes one global bucket
/// set per key.
#[derive(Clone)]
pub struct ThrottleLayer {
    /// Shared limiter backend.
    limiter: Arc<dyn RateLimiter>,
    /// Limit + trusted-proxy configuration.
    config: ThrottleConfig,
}

impl std::fmt::Debug for ThrottleLayer {
    /// Manual debug — the trait object has no Debug.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThrottleLayer")
            .field("driver", &self.limiter.driver())
            .field("limit", &self.config.limit)
            .field("trusted_proxies", &self.config.trusted_proxies)
            .finish()
    }
}

impl ThrottleLayer {
    /// Build a layer over a fresh in-memory limiter.
    pub fn new(limit: Limit) -> Self {
        Self {
            limiter: Arc::new(MemoryRateLimiter::new()),
            config: ThrottleConfig::new(limit),
        }
    }

    /// Build a layer over a shared limiter backend (memory/redis).
    pub fn with_limiter(limiter: Arc<dyn RateLimiter>, config: ThrottleConfig) -> Self {
        Self { limiter, config }
    }

    /// Build a layer from an explicit config over a fresh in-memory limiter.
    pub fn from_config(config: ThrottleConfig) -> Self {
        Self {
            limiter: Arc::new(MemoryRateLimiter::new()),
            config,
        }
    }
}

impl<S> tower::Layer<S> for ThrottleLayer {
    type Service = ThrottleService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ThrottleService {
            inner,
            limiter: Arc::clone(&self.limiter),
            config: self.config.clone(),
        }
    }
}

/// Middleware service enforcing one rate limit per request.
#[derive(Clone)]
pub struct ThrottleService<S> {
    inner: S,
    limiter: Arc<dyn RateLimiter>,
    config: ThrottleConfig,
}

impl<S> std::fmt::Debug for ThrottleService<S> {
    /// Manual debug — inner service and trait object are opaque.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThrottleService")
            .field("driver", &self.limiter.driver())
            .field("limit", &self.config.limit)
            .field("inner", &std::any::type_name::<S>())
            .finish()
    }
}

impl<S> ThrottleService<S> {
    /// Resolve the bucket key for a request from its peer and headers.
    ///
    /// `KeyBy::Ip` consults `X-Forwarded-For` only when proxies are trusted;
    /// `KeyBy::Key` uses the pre-resolved key directly. `KeyBy::User` falls
    /// back to the peer address when no authentication middleware has
    /// stamped a user id extension (callers may set one via
    /// `request.extensions_mut().insert` before this layer runs).
    fn key_for(&self, req: &Request<Body>) -> String {
        let limit = &self.config.limit;
        match &limit.key_by {
            KeyBy::Ip | KeyBy::User => {
                let peer = req
                    .extensions()
                    .get::<UserKey>()
                    .map(|u| u.0.clone())
                    .or_else(|| peer_ip(req));
                match (peer, &limit.key_by) {
                    (Some(ip), KeyBy::User) => format!("user:{ip}"),
                    (Some(ip), _) => self.config.key_for_peer(&ip, xff(req)),
                    (None, _) => "unknown-peer".to_string(),
                }
            }
            KeyBy::Key(key) => format!("key:{key}"),
        }
    }
}

/// Extension stamping the authenticated user id (set by the auth middleware
/// before the throttle layer runs when `by_user()` is used).
#[derive(Clone, Debug)]
pub struct UserKey(pub String);

/// Read the peer IP from the connection info extension.
fn peer_ip(req: &Request<Body>) -> Option<String> {
    req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
}

/// Read the `X-Forwarded-For` header value.
///
/// Only the de-facto `X-Forwarded-For` header is consulted — the RFC 7239
/// `Forwarded` header is deliberately ignored so a client cannot smuggle a
/// trusted identity through a different hop header.
fn xff(req: &Request<Body>) -> Option<&str> {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
}

/// Build the 429 JSON error envelope with `Retry-After` header.
fn throttle_response(retry_after_secs: u64) -> Response<Body> {
    let error = serde_json::json!({
        "errors": [{
            "status": "429",
            "code": THROTTLE_CODE,
            "title": "Too Many Requests",
            "detail": format!("Rate limit exceeded; retry in {retry_after_secs} seconds."),
            "meta": { "retry_after_secs": retry_after_secs },
        }]
    });
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after_secs.to_string())],
        Json(error),
    )
        .into_response()
}

impl<S> tower::Service<Request<Body>> for ThrottleService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Infallible>,
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

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let key = self.key_for(&req);
        match self.limiter.hit(&key, &self.config.limit) {
            ThrottleDecision::Allowed { .. } => {
                let mut inner = self.inner.clone();
                Box::pin(async move { inner.call(req).await })
            }
            ThrottleDecision::Denied { retry_after_secs } => {
                super::log_throttle(&key, retry_after_secs);
                let response = throttle_response(retry_after_secs);
                Box::pin(async move { Ok(response) })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::throttle::Limit;
    use axum::body::Body;
    use tower::{Layer, Service, ServiceExt};

    /// Echo service concrete type (fn item keeps the service `Clone`).
    type EchoService = tower::util::ServiceFn<
        fn(Request<Body>) -> std::future::Ready<std::result::Result<Response<Body>, Infallible>>,
    >;

    fn echo(
        _req: Request<Body>,
    ) -> std::future::Ready<std::result::Result<Response<Body>, Infallible>> {
        std::future::ready(Ok(Response::new(Body::from("ok"))))
    }

    fn inner_service() -> EchoService {
        tower::service_fn(echo)
    }

    fn request_from(ip: &str) -> Request<Body> {
        let mut req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request builds");
        let addr: std::net::SocketAddr = format!("{ip}:54321").parse().expect("addr parses");
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(addr));
        req
    }

    /// 3 allowed then a 429 with Retry-After on the 4th request.
    #[tokio::test]
    async fn fourth_request_rejected_with_retry_after() {
        let layer = ThrottleLayer::new(Limit::per_minute(3).by_ip());
        let mut svc = layer.layer(inner_service());

        for _ in 0..3 {
            let resp = svc
                .ready()
                .await
                .expect("ready")
                .call(request_from("1.2.3.4"))
                .await
                .expect("call succeeds");
            assert_eq!(resp.status(), StatusCode::OK);
        }
        let resp = svc
            .ready()
            .await
            .expect("ready")
            .call(request_from("1.2.3.4"))
            .await
            .expect("call succeeds");
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after = resp
            .headers()
            .get(header::RETRY_AFTER)
            .expect("Retry-After header");
        let secs: u64 = retry_after.to_str().expect("utf8").parse().expect("number");
        assert!((1..=60).contains(&secs));
    }

    /// Different peers have independent buckets.
    #[tokio::test]
    async fn peers_are_isolated() {
        let layer = ThrottleLayer::new(Limit::per_minute(1).by_ip());
        let mut svc = layer.layer(inner_service());

        let first = svc
            .ready()
            .await
            .expect("ready")
            .call(request_from("1.1.1.1"))
            .await
            .expect("ok");
        assert_eq!(first.status(), StatusCode::OK);

        let second = svc
            .ready()
            .await
            .expect("ready")
            .call(request_from("2.2.2.2"))
            .await
            .expect("ok");
        assert_eq!(second.status(), StatusCode::OK);

        let third = svc
            .ready()
            .await
            .expect("ready")
            .call(request_from("1.1.1.1"))
            .await
            .expect("denied");
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    /// X-Forwarded-For is honored only behind trusted proxies.
    #[tokio::test]
    async fn xff_honored_only_when_proxies_trusted() {
        let untrusted = ThrottleLayer::new(Limit::per_minute(1).by_ip());
        let mut svc = untrusted.layer(inner_service());
        let mut req = request_from("1.2.3.4");
        req.headers_mut()
            .insert("x-forwarded-for", "9.9.9.9".parse().expect("hdr"));
        let first = svc
            .ready()
            .await
            .expect("ready")
            .call(req)
            .await
            .expect("ok");
        assert_eq!(first.status(), StatusCode::OK);

        // Same spoofed XFF from a *different* peer — the untrusted layer
        // bucket is keyed by peer, so 1.2.3.4 is exhausted now.
        let mut req = request_from("5.5.5.5");
        req.headers_mut()
            .insert("x-forwarded-for", "9.9.9.9".parse().expect("hdr"));
        let second = svc
            .ready()
            .await
            .expect("ready")
            .call(req)
            .await
            .expect("ok");
        assert_eq!(second.status(), StatusCode::OK);
    }
}
