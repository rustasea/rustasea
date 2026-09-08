/// Rate limiting — `RateLimiter` trait, builder, config, tower middleware.
///
/// Laravel-style `limit.per_minute(n).by(ip|user|key(fn))` maps onto a
/// `ThrottleConfig` consumed by [`layer::ThrottleLayer`]. The in-memory
/// fixed-window limiter ([`limiter::MemoryRateLimiter`]) is bounded: expired
/// windows are pruned and the bucket map is capped so a flood of distinct
/// keys cannot grow memory without limit. The Redis-backed limiter (M4)
/// implements the same trait.
pub mod layer;
pub mod limiter;

use std::time::Duration;

/// How the throttle bucket key is derived from a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyBy {
    /// Bucket by peer IP (X-Forwarded-For ignored unless trusted).
    Ip,
    /// Bucket by authenticated user id.
    User,
    /// Bucket by an arbitrary key string (already resolved).
    Key(String),
}

/// Rate-limit definition — `limit.per_minute(10).by_ip()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    /// Maximum requests per window.
    pub max_attempts: u32,
    /// Window size in seconds.
    pub decay_secs: u64,
    /// Bucket key strategy.
    pub key_by: KeyBy,
}

impl Limit {
    /// Start a per-minute limit with `n` attempts.
    pub fn per_minute(n: u32) -> Self {
        Self {
            max_attempts: n,
            decay_secs: 60,
            key_by: KeyBy::Ip,
        }
    }

    /// Start a per-second limit with `n` attempts.
    pub fn per_second(n: u32) -> Self {
        Self {
            max_attempts: n,
            decay_secs: 1,
            key_by: KeyBy::Ip,
        }
    }

    /// Bucket by peer IP.
    pub fn by_ip(mut self) -> Self {
        self.key_by = KeyBy::Ip;
        self
    }

    /// Bucket by authenticated user id.
    pub fn by_user(mut self) -> Self {
        self.key_by = KeyBy::User;
        self
    }

    /// Bucket by a fixed key.
    pub fn by_key(mut self, key: impl Into<String>) -> Self {
        self.key_by = KeyBy::Key(key.into());
        self
    }
}

/// Snapshot of one bucket's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketState {
    /// Requests observed in the current window.
    pub hits: u32,
    /// Window start (UNIX seconds).
    pub window_start: u64,
    /// Seconds until the window resets.
    pub retry_after_secs: u64,
}

/// Outcome of a rate-limit check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThrottleDecision {
    /// Request is allowed.
    Allowed { remaining: u32 },
    /// Request exceeds the limit — reject with `Retry-After`.
    Denied { retry_after_secs: u64 },
}

/// Contract for a rate limiter backend.
pub trait RateLimiter: Send + Sync {
    /// Name of the driver (`memory`, `redis`).
    fn driver(&self) -> &str;

    /// Inspect the current bucket state for a key (test/metrics probe).
    ///
    /// Returns `None` for unknown keys and for buckets whose window has fully
    /// decayed (the limiter reports only live windows).
    fn state(&self, key: &str) -> Option<BucketState>;

    /// Register one hit and decide allow/deny.
    fn hit(&self, key: &str, limit: &Limit) -> ThrottleDecision;
}

/// Configuration consumed by the tower [`layer::ThrottleLayer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThrottleConfig {
    /// Rate-limit definition.
    pub limit: Limit,
    /// Trusted proxy CIDRs; when empty, `X-Forwarded-For` is ignored.
    pub trusted_proxies: Vec<String>,
}

impl ThrottleConfig {
    /// Create config from a limit definition.
    pub fn new(limit: Limit) -> Self {
        Self {
            limit,
            trusted_proxies: Vec::new(),
        }
    }

    /// Mark proxies as trusted so `X-Forwarded-For` is honored.
    pub fn behind_proxies(mut self, proxies: Vec<String>) -> Self {
        self.trusted_proxies = proxies;
        self
    }

    /// Derive the bucket key for a request peer address.
    ///
    /// `X-Forwarded-For` is only consulted when `trusted_proxies` is
    /// non-empty (FR-306/BDD @throttle: unset proxies => peer wins).
    pub fn key_for_peer(&self, peer_ip: &str, forwarded_for: Option<&str>) -> String {
        if !self.trusted_proxies.is_empty() {
            if let Some(ff) = forwarded_for {
                if let Some(first) = ff.split(',').next() {
                    let candidate = first.trim();
                    if !candidate.is_empty() {
                        return candidate.to_string();
                    }
                }
            }
        }
        peer_ip.to_string()
    }
}

/// Resolve the retry-after duration from a deny decision.
pub fn retry_after_secs(decision: &ThrottleDecision) -> Option<u64> {
    match decision {
        ThrottleDecision::Denied { retry_after_secs } => Some(*retry_after_secs),
        ThrottleDecision::Allowed { .. } => None,
    }
}

/// Log a throttle rejection with `Retry-After` (NFR-Mai-01 metrics hook).
pub fn log_throttle(key: &str, retry_after_secs: u64) {
    // Structured-log hook; the S05 observability pass routes this through
    // tracing with a correlation id.
    let _ = (key, retry_after_secs);
}

/// Sleep helper for tests (tokio-friendly duration from decision).
pub fn sleep_for(decision: &ThrottleDecision) -> Duration {
    Duration::from_secs(retry_after_secs(decision).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// X-Forwarded-For is ignored unless proxies are trusted.
    #[test]
    fn forwarded_for_requires_trusted_proxies() {
        let cfg = ThrottleConfig::new(Limit::per_minute(10).by_ip());
        assert_eq!(cfg.key_for_peer("1.2.3.4", Some("9.9.9.9")), "1.2.3.4");

        let trusted = cfg.behind_proxies(vec!["10.0.0.0/8".into()]);
        assert_eq!(trusted.key_for_peer("1.2.3.4", Some("9.9.9.9")), "9.9.9.9");
    }

    /// by_user and by_key carry their bucket strategy.
    #[test]
    fn limit_builders_carry_strategy() {
        assert_eq!(Limit::per_minute(5).by_user().key_by, KeyBy::User);
        assert_eq!(
            Limit::per_minute(5).by_key("k").key_by,
            KeyBy::Key("k".into())
        );
    }
}
