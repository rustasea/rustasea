/// Retry-policy contracts for queue jobs.
use std::time::Duration;

/// Advisory retry policy attached to a job via `#[tries]`/`#[backoff]`.
///
/// Stored on the `Job` as a zero-cost type-level constant (the doc-shape
/// `Job<T>` requires `T: Serialize + DeserializeOwned`, so the payload must
/// stay serializable — the policy rides alongside, not inside, the payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts (initial run counts as one).
    pub tries: u32,
    /// Delay before the first retry; later retries double (exponential).
    pub backoff: Duration,
}

impl RetryPolicy {
    /// Create a policy with `tries` attempts and a fixed `backoff` base.
    pub const fn new(tries: u32, backoff: Duration) -> Self {
        Self { tries, backoff }
    }

    /// Default policy: one attempt, no retries, no delay.
    pub const fn none() -> Self {
        Self {
            tries: 1,
            backoff: Duration::ZERO,
        }
    }

    /// Compute the delay before the given 1-based retry attempt.
    ///
    /// Doubles the base backoff for every retry beyond the first: attempt 2
    /// waits `backoff`, attempt 3 waits `2 * backoff`, and so on.
    pub fn delay_for_retry(&self, retry_attempt: u32) -> Duration {
        if retry_attempt <= 1 || self.backoff.is_zero() {
            return self.backoff;
        }
        let factor = 1u32 << (retry_attempt - 1).min(30);
        self.backoff.saturating_mul(factor)
    }
}

/// Opt-in retry contract for jobs whose retry decision depends on the error.
///
/// When implemented alongside `#[tries(n)]`, the worker consults
/// `should_retry` for every failed attempt before spending the budget.
pub trait ShouldRetry {
    /// Decide whether a failed attempt should be retried.
    fn should_retry(&self, attempt: u32, err: &crate::JobError) -> bool;
}

/// Opt-in retry contract bounding retries by an absolute wall-clock deadline.
pub trait ShouldRetryUntil {
    /// Absolute UTC instant after which the job must not be retried.
    fn retry_until(&self) -> Option<chrono::DateTime<chrono::Utc>>;
}
