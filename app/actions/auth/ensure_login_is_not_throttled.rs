//! Applies login throttling before authentication is attempted.

use rustasea::auth::{Limit, RateLimiter, ThrottleDecision};

/// Rejection returned when a login key exceeds its configured rate limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginThrottled {
    /// Seconds the caller must wait before retrying (`Retry-After`).
    pub retry_after_secs: u64,
}

impl std::fmt::Display for LoginThrottled {
    /// Render the throttle rejection together with its retry delay.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "too many login attempts; retry after {}s",
            self.retry_after_secs
        )
    }
}

impl std::error::Error for LoginThrottled {}

/// Register one attempt for `key` and reject it once `limit` is exceeded.
pub fn ensure(key: &str, limiter: &dyn RateLimiter, limit: &Limit) -> Result<(), LoginThrottled> {
    match limiter.hit(key, limit) {
        ThrottleDecision::Allowed { .. } => Ok(()),
        ThrottleDecision::Denied { retry_after_secs } => {
            Err(LoginThrottled { retry_after_secs })
        }
    }
}
