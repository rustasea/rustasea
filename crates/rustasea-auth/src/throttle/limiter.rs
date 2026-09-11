/// In-memory fixed-window rate limiter with bounded state.
///
/// Buckets are keyed per (decay_secs, key) in a shared map. The map is
/// bounded two ways: expired windows are pruned on an amortized schedule
/// (every [`PRUNE_EVERY`] hits) and the oldest live window is evicted when
/// the map exceeds [`MAX_BUCKETS`], so a flood of distinct keys cannot grow
/// memory without limit. A poisoned mutex fails closed — every request is
/// denied with the full window as `Retry-After`, never a panic.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{BucketState, Limit, RateLimiter, ThrottleDecision};

/// Soft cap on tracked buckets (~10k keys). When exceeded the oldest live
/// window is evicted so memory stays bounded under key floods.
pub const MAX_BUCKETS: usize = 10_000;

/// Run the full expired-window sweep every N hits (amortized O(n/N)).
const PRUNE_EVERY: u64 = 64;

/// Shared bucket state for a single key.
#[derive(Debug)]
struct Bucket {
    hits: u32,
    window_start: u64,
    /// Monotonic insert counter used for oldest-first eviction.
    seq: u64,
}

/// In-memory fixed-window limiter.
#[derive(Debug, Default)]
pub struct MemoryRateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    now: Arc<AtomicU64>,
    next_seq: AtomicU64,
    hit_counter: AtomicU64,
}

impl MemoryRateLimiter {
    /// Create an empty limiter.
    pub fn new() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            now: Arc::new(AtomicU64::new(0)),
            next_seq: AtomicU64::new(0),
            hit_counter: AtomicU64::new(0),
        }
    }

    /// Freeze the clock (tests).
    pub fn set_now(&self, unix_secs: u64) {
        self.now.store(unix_secs, Ordering::Relaxed);
    }

    /// Reset all buckets.
    pub fn reset(&self) {
        if let Ok(mut buckets) = self.buckets.lock() {
            buckets.clear();
        }
    }

    /// Current UNIX seconds (honors a frozen clock for tests).
    pub fn current_time(&self) -> u64 {
        let frozen = self.now.load(Ordering::Relaxed);
        if frozen > 0 {
            return frozen;
        }
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Compose the map key from the window size and bucket key.
    pub fn bucket_key(key: &str, limit: &Limit) -> String {
        format!("{}:{key}", limit.decay_secs)
    }

    /// Decay seconds encoded in a `{decay}:{key}` map key.
    fn decay_of(full_key: &str) -> u64 {
        full_key
            .split_once(':')
            .and_then(|(d, _)| d.parse().ok())
            .unwrap_or(0)
    }

    /// Drop windows that have fully elapsed; returns buckets retained.
    fn prune(&self, buckets: &mut HashMap<String, Bucket>, now: u64) {
        buckets.retain(|full_key, b| now.saturating_sub(b.window_start) < Self::decay_of(full_key));
    }

    /// Enforce the bucket cap by evicting the oldest window (by seq).
    fn enforce_cap(&self, buckets: &mut HashMap<String, Bucket>) {
        if buckets.len() <= MAX_BUCKETS {
            return;
        }
        let evict = buckets
            .iter()
            .min_by_key(|(_, b)| b.seq)
            .map(|(k, _)| k.clone());
        if let Some(key) = evict {
            buckets.remove(&key);
        }
    }
}

impl RateLimiter for MemoryRateLimiter {
    fn driver(&self) -> &str {
        "memory"
    }

    fn state(&self, key: &str) -> Option<BucketState> {
        let now = self.current_time();
        let buckets = self.buckets.lock().ok()?;
        let mut best: Option<BucketState> = None;
        for (full_key, bucket) in buckets.iter() {
            // Bucket keys are "{decay}:{key}"; the probe key must match the
            // suffix after the decay prefix.
            let (decay_str, suffix) = full_key.split_once(':')?;
            if suffix != key {
                continue;
            }
            let decay_secs: u64 = decay_str.parse().unwrap_or(0);
            let elapsed = now.saturating_sub(bucket.window_start);
            if elapsed >= decay_secs {
                continue; // expired window — not a live state
            }
            let candidate = BucketState {
                hits: bucket.hits,
                window_start: bucket.window_start,
                retry_after_secs: decay_secs.saturating_sub(elapsed).max(1),
            };
            // Report the window that resets soonest (least Retry-After).
            if best
                .as_ref()
                .is_none_or(|b| candidate.retry_after_secs < b.retry_after_secs)
            {
                best = Some(candidate);
            }
        }
        best
    }

    fn hit(&self, key: &str, limit: &Limit) -> ThrottleDecision {
        let now = self.current_time();
        let full_key = Self::bucket_key(key, limit);
        let mut buckets = match self.buckets.lock() {
            Ok(b) => b,
            Err(_) => {
                return ThrottleDecision::Denied {
                    retry_after_secs: limit.decay_secs,
                };
            }
        };

        if now == 0 {
            return ThrottleDecision::Denied {
                retry_after_secs: limit.decay_secs,
            };
        }

        // Amortized sweep: prune fully-expired windows once per N hits so the
        // hot path stays O(1) for short-lived keys.
        let count = self.hit_counter.fetch_add(1, Ordering::Relaxed);
        if count % PRUNE_EVERY == 0 {
            self.prune(&mut buckets, now);
        }

        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let mut fresh = false;
        let bucket = buckets.entry(full_key).or_insert_with(|| {
            fresh = true;
            Bucket {
                hits: 0,
                window_start: now,
                seq,
            }
        });
        if now.saturating_sub(bucket.window_start) >= limit.decay_secs {
            bucket.window_start = now;
            bucket.hits = 0;
            bucket.seq = seq;
        }
        if bucket.hits >= limit.max_attempts {
            let elapsed = now.saturating_sub(bucket.window_start);
            let retry_after_secs = limit.decay_secs.saturating_sub(elapsed).max(1);
            return ThrottleDecision::Denied { retry_after_secs };
        }
        bucket.hits += 1;
        let remaining = limit.max_attempts - bucket.hits;
        let decided = ThrottleDecision::Allowed { remaining };
        // Cap enforcement happens outside the entry borrow so the eviction
        // can mutate the map after the bucket was updated.
        if fresh {
            self.enforce_cap(&mut buckets);
        }
        decided
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::throttle::Limit;

    /// 4th request in the same window is denied with Retry-After.
    #[test]
    fn fourth_request_in_window_is_denied() {
        let limiter = MemoryRateLimiter::new();
        limiter.set_now(1_000_000);
        let limit = Limit::per_minute(3).by_ip();

        assert_eq!(
            limiter.hit("1.2.3.4", &limit),
            ThrottleDecision::Allowed { remaining: 2 }
        );
        assert_eq!(
            limiter.hit("1.2.3.4", &limit),
            ThrottleDecision::Allowed { remaining: 1 }
        );
        assert_eq!(
            limiter.hit("1.2.3.4", &limit),
            ThrottleDecision::Allowed { remaining: 0 }
        );
        let denied = limiter.hit("1.2.3.4", &limit);
        match denied {
            ThrottleDecision::Denied { retry_after_secs } => {
                assert!((1..=60).contains(&retry_after_secs));
            }
            ThrottleDecision::Allowed { .. } => panic!("4th request must be denied"),
        }
    }

    /// A fresh window (after decay) resets the bucket.
    #[test]
    fn window_decay_resets_bucket() {
        let limiter = MemoryRateLimiter::new();
        limiter.set_now(1_000_000);
        let limit = Limit::per_minute(1).by_ip();
        assert_eq!(
            limiter.hit("1.2.3.4", &limit),
            ThrottleDecision::Allowed { remaining: 0 }
        );
        limiter.set_now(1_000_061);
        assert_eq!(
            limiter.hit("1.2.3.4", &limit),
            ThrottleDecision::Allowed { remaining: 0 }
        );
    }

    /// `state()` reports a live bucket and `None` once it decays.
    #[test]
    fn state_reports_live_window_only() {
        let limiter = MemoryRateLimiter::new();
        limiter.set_now(1_000_000);
        let limit = Limit::per_minute(3).by_ip();
        limiter.hit("1.2.3.4", &limit);
        let state = limiter.state("1.2.3.4").expect("bucket exists");
        assert_eq!(state.hits, 1);
        assert_eq!(state.window_start, 1_000_000);
        assert_eq!(state.retry_after_secs, 60);

        // Unknown key has no bucket.
        assert!(limiter.state("9.9.9.9").is_none());

        // After the window decays the bucket is no longer live.
        limiter.set_now(1_000_060);
        assert!(limiter.state("1.2.3.4").is_none());
    }

    /// Distinct windows for the same key (1s vs 60s) coexist; state reports
    /// the window that resets soonest.
    #[test]
    fn state_handles_multi_window_keys() {
        let limiter = MemoryRateLimiter::new();
        limiter.set_now(1_000_000);
        limiter.hit("1.2.3.4", &Limit::per_second(1).by_ip());
        limiter.hit("1.2.3.4", &Limit::per_minute(5).by_ip());
        let state = limiter.state("1.2.3.4").expect("bucket exists");
        assert_eq!(state.retry_after_secs, 1);
    }

    /// A key flood cannot grow the bucket map beyond the soft cap.
    #[test]
    fn key_flood_is_bounded() {
        let limiter = MemoryRateLimiter::new();
        limiter.set_now(1_000_000);
        let limit = Limit::per_minute(1).by_ip();
        for i in 0..(MAX_BUCKETS + 100) {
            limiter.hit(&format!("k{i}"), &limit);
        }
        let count = limiter.buckets.lock().expect("lock").len();
        assert!(count <= MAX_BUCKETS, "bounded at cap, got {count}");
    }
}
