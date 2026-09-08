//! Factory sequence registry — `Str`-style resets per test.
//!
//! The M5 harness requires that model factories reset their sequence between
//! tests (US-M5-04): `test_a` producing `user10@example.com` must not leak
//! into `test_b`, whose first row is `user1@example.com`. The registry keeps
//! an `AtomicU64` counter per factory key; [`reset_factory_sequences`] zeroes
//! every counter and is invoked by [`crate::TestCase::setup`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// A counter handle shared with one factory key.
pub type Sequence = Arc<AtomicU64>;

/// Access the process-wide factory sequence registry.
fn registry() -> &'static Mutex<HashMap<String, Sequence>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Sequence>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a factory key, returning its (possibly shared) sequence counter.
///
/// A key registered twice returns the same counter, so two factories sharing
/// a sequence (e.g. email domains) advance together.
pub fn register_sequence(key: impl Into<String>) -> Sequence {
    let key = key.into();
    let mut map = registry().lock().unwrap_or_else(|p| p.into_inner());
    map.entry(key)
        .or_insert_with(|| Arc::new(AtomicU64::new(0)))
        .clone()
}

/// Reset every registered factory sequence to zero (per-TestCase hook).
pub fn reset_factory_sequences() {
    let map = registry().lock().unwrap_or_else(|p| p.into_inner());
    for counter in map.values() {
        counter.store(0, Ordering::SeqCst);
    }
}

/// Snapshot of every registered factory key → current sequence.
pub fn factory_registry() -> Vec<(String, u64)> {
    let map = registry().lock().unwrap_or_else(|p| p.into_inner());
    map.iter()
        .map(|(k, v)| (k.clone(), v.load(Ordering::SeqCst)))
        .collect()
}

/// A `Str`-style factory emitting `user1@example.com`, `user2@example.com`, …
///
/// Emulates the Laravel `Str` factory helper: unique per sequence, resets per
/// test via the harness hook. The returned `Sequence` is shared with any
/// other factory registered under the same key.
#[derive(Debug, Clone)]
pub struct StrFactory {
    /// Registry key (default `str`).
    key: String,
    /// Shared sequence counter.
    sequence: Sequence,
    /// Email domain.
    domain: String,
}

impl Default for StrFactory {
    /// Create the default `str` factory over `example.com`.
    fn default() -> Self {
        Self::new("str")
    }
}

impl StrFactory {
    /// Create a factory under `key` emitting `<key><seq>@example.com`.
    pub fn new(key: &str) -> Self {
        let sequence = register_sequence(key);
        Self {
            key: key.to_string(),
            sequence,
            domain: "example.com".to_string(),
        }
    }

    /// Override the email domain (default `example.com`).
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Produce the next unique value (`user1@example.com`, `user2@…`).
    pub fn next(&self) -> String {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        format!("{}{seq}@{}", self.key, self.domain)
    }

    /// Current sequence value (how many values have been produced).
    pub fn count(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Reset this factory's counter to zero.
    pub fn reset(&self) {
        self.sequence.store(0, Ordering::SeqCst);
    }
}

/// Convenience constructor for the default `Str` factory.
pub fn str_factory() -> StrFactory {
    StrFactory::default()
}

/// Test-only helpers used by the test_case module tests.
#[cfg(test)]
pub(crate) fn bump_sequence_for_test() {
    let factory = StrFactory::new("user");
    let _ = factory.next();
}

/// Test-only read of the `str` key used by the test_case module tests.
#[cfg(test)]
pub(crate) fn current_sequence_for_test() -> u64 {
    let factory = StrFactory::new("user");
    factory.count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies next/count advance and reset returns to zero.
    #[test]
    fn str_factory_sequences_and_resets() {
        let factory = StrFactory::new("user");
        assert_eq!(factory.next(), "user1@example.com");
        assert_eq!(factory.next(), "user2@example.com");
        assert_eq!(factory.count(), 2);

        factory.reset();
        assert_eq!(factory.count(), 0);
        assert_eq!(factory.next(), "user1@example.com");
    }

    /// Verifies shared keys advance together and the registry reflects them.
    #[test]
    fn shared_sequence_advances_together() {
        let a = StrFactory::new("shared");
        let b = StrFactory::new("shared");
        let _ = a.next();
        assert_eq!(b.count(), 1);
        assert!(factory_registry()
            .iter()
            .any(|(k, v)| k == "shared" && *v == 1));
    }

    /// Verifies reset_factory_sequences zeroes every counter.
    #[test]
    fn reset_clears_all_keys() {
        let a = StrFactory::new("alpha");
        let b = StrFactory::new("beta");
        let _ = a.next();
        let _ = b.next();
        reset_factory_sequences();
        assert_eq!(a.count(), 0);
        assert_eq!(b.count(), 0);
    }
}
