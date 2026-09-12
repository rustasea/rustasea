/// Email-verification hook contract (FR-311).
///
/// Clears `email_verified_at` on the authenticated user so downstream
/// `MustVerifyEmail` flows re-send verification. Extracted from `session` to
/// keep each module under the 500-line limit.
use crate::error::Result;

/// `markEmailAsUnverified` hook contract (FR-311).
pub trait EmailVerification {
    /// Clear the user's `email_verified_at` timestamp.
    fn mark_email_as_unverified(&self, user_id: &str) -> Result<()>;
}

/// In-memory email-verification hook for tests and local dev.
#[derive(Debug, Default)]
pub struct MemoryEmailVerification {
    verified_at: std::sync::RwLock<std::collections::HashMap<String, Option<String>>>,
}

impl MemoryEmailVerification {
    /// Record a verified timestamp for a user.
    pub fn seed_verified(&self, user_id: &str, at: String) {
        // Fail closed on a poisoned lock rather than panic.
        if let Ok(mut verified) = self.verified_at.write() {
            verified.insert(user_id.to_string(), Some(at));
        }
    }

    /// Read the current verification timestamp (test probe).
    pub fn verified_at(&self, user_id: &str) -> Option<Option<String>> {
        self.verified_at.read().ok()?.get(user_id).cloned()
    }
}

impl EmailVerification for MemoryEmailVerification {
    fn mark_email_as_unverified(&self, user_id: &str) -> Result<()> {
        if let Ok(mut verified) = self.verified_at.write() {
            verified.insert(user_id.to_string(), None);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// markEmailAsUnverified clears a seeded verification timestamp.
    #[test]
    fn mark_email_as_unverified_clears_timestamp() {
        let hook = MemoryEmailVerification::default();
        hook.seed_verified("user-1", "2026-01-01T00:00:00Z".into());
        assert_eq!(
            hook.verified_at("user-1"),
            Some(Some("2026-01-01T00:00:00Z".to_string()))
        );
        let result = hook.mark_email_as_unverified("user-1");
        assert!(result.is_ok());
        assert_eq!(hook.verified_at("user-1"), Some(None));
    }
}
