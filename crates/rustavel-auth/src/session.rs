/// Session guard and session/cache hardening policy.
///
/// `SessionGuard` is the tower-sessions backed identity path: the session
/// store owns the cookie, and the guard reads the authenticated user id from
/// the session. `SessionPolicy` encodes the Laravel-13 hardening defaults:
/// JSON serialization, hyphenated `-session-`/`-cache-` key prefixes, and a
/// `serializable_classes` allow-list checked before any deserialization.
use crate::error::{AuthError, Result};
use crate::guard::{AuthUser, Credentials, Guard, Token};

/// Hardening policy for session/cache serialization (FR-303/FR-304).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPolicy {
    /// Serialization format; `json` is the only supported default.
    pub serialization: String,
    /// Key prefix — must contain the hyphenated `-session-` marker.
    pub prefix: String,
    /// Fully-qualified type names allowed to deserialize.
    pub serializable_classes: Vec<String>,
}

impl Default for SessionPolicy {
    /// Laravel-13 defaults: JSON, hyphenated prefix, empty allow-list.
    fn default() -> Self {
        Self {
            serialization: "json".to_string(),
            prefix: "rustavel-session-".to_string(),
            serializable_classes: Vec::new(),
        }
    }
}

impl SessionPolicy {
    /// Create a policy with an explicit allow-list.
    pub fn with_classes(classes: Vec<String>) -> Self {
        Self {
            serializable_classes: classes,
            ..Self::default()
        }
    }

    /// Verify the prefix uses hyphens, not underscores (`-session-`).
    pub fn validate_prefix(&self) -> Result<()> {
        if self.serialization != "json" {
            return Err(AuthError::Disabled(format!(
                "unsupported session serialization {:?} (only \"json\" is allowed)",
                self.serialization
            )));
        }
        if !self.prefix.contains("-session-") {
            return Err(AuthError::Disabled(format!(
                "session prefix {:?} must contain -session-",
                self.prefix
            )));
        }
        Ok(())
    }

    /// Verify a cache prefix uses the hyphenated `-cache-` marker.
    ///
    /// Separate from [`SessionPolicy::validate_prefix`] because cache keys
    /// and session keys use different markers (FS-M3-03, TC-M3-07). A prefix
    /// with the underscore variant (`_cache_`) is rejected, matching the
    /// Laravel-13 hyphenation rule (#12).
    pub fn validate_cache_prefix(prefix: &str) -> Result<()> {
        if !prefix.contains("-cache-") {
            return Err(AuthError::Disabled(format!(
                "cache prefix {prefix:?} must contain -cache-"
            )));
        }
        Ok(())
    }

    /// Gate a type name against the allow-list before deserialization.
    pub fn allow(
        &self,
        type_name: &str,
    ) -> std::result::Result<(), crate::error::SerializationError> {
        if self.serializable_classes.iter().any(|c| c == type_name) {
            Ok(())
        } else {
            Err(crate::error::SerializationError::NotAllowed {
                type_name: type_name.to_string(),
            })
        }
    }
}

/// Allow-list contract for safe deserialization (FR-304, TC-M3-06).
///
/// Any store that deserializes values from untrusted bytes (sessions,
/// caches) must gate the concrete type name against an explicit allow-list
/// *before* `from_str`, so a poisoned cache entry can never decode into an
/// attacker-chosen gadget type. Session and cache policies share this trait;
/// both surface `SerializationError::NotAllowed { type_name }` on a miss.
pub trait DeserializationAllowList {
    /// Reject deserializing `type_name` unless it is allow-listed.
    fn allow(&self, type_name: &str) -> std::result::Result<(), crate::error::SerializationError>;
}

impl DeserializationAllowList for SessionPolicy {
    /// Delegates to [`SessionPolicy::allow`].
    fn allow(&self, type_name: &str) -> std::result::Result<(), crate::error::SerializationError> {
        SessionPolicy::allow(self, type_name)
    }
}

/// Identity stored inside a session for the session guard.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionUser {
    /// Authenticated user UUID.
    pub id: String,
    /// Optional display email.
    pub email: Option<String>,
}

/// Session guard — resolves identity from a tower-sessions store.
///
/// `login` is a stub until the store is wired: it persists the issued user
/// id under the session key. The guard refuses to *issue* tokens (session
/// guard issues cookies, not bearer tokens) and only resolves identities.
pub struct SessionGuard {
    /// Guard name (`session`).
    name: String,
    /// Store-backed session handle placeholder (M5 wiring target).
    session: Option<SessionUser>,
    /// Hardening policy applied to session payloads.
    policy: SessionPolicy,
}

impl SessionGuard {
    /// Create a session guard with a hardening policy.
    pub fn new(policy: SessionPolicy) -> Self {
        Self {
            name: "session".to_string(),
            session: None,
            policy,
        }
    }

    /// Hardening policy this guard enforces.
    pub fn policy(&self) -> &SessionPolicy {
        &self.policy
    }

    /// Persist the authenticated user into the session store.
    #[cfg(test)]
    fn store(&mut self, user: SessionUser) {
        self.session = Some(user);
    }
}

impl Guard for SessionGuard {
    fn name(&self) -> &str {
        "session"
    }

    fn login<'a>(
        &'a self,
        _creds: &'a Credentials,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            Err(AuthError::Disabled(
                "session guard authenticates via the login page flow, not bearer tokens".into(),
            ))
        })
    }

    fn login_using_id<'a>(
        &'a self,
        _user_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            Err(AuthError::Disabled(
                "loginUsingId is not available on the session guard".into(),
            ))
        })
    }

    fn parse<'a>(
        &'a self,
        _token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AuthUser>> + Send + 'a>> {
        Box::pin(async move {
            Err(AuthError::Disabled(
                "session guard parses the session cookie, not bearer tokens".into(),
            ))
        })
    }

    fn refresh<'a>(
        &'a self,
        _token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            Err(AuthError::Disabled(
                "session guard does not issue refresh tokens".into(),
            ))
        })
    }

    fn logout<'a>(
        &'a self,
        _token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // Store delete is a no-op stub until tower-sessions wiring (S05).
            Ok(())
        })
    }

    fn user<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<AuthUser>>> + Send + 'a>>
    {
        Box::pin(async move {
            Ok(self.session.as_ref().map(|s| AuthUser {
                id: s.id.clone(),
                email: s.email.clone(),
                guard: self.name.clone(),
            }))
        })
    }

    fn id<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>>> + Send + 'a>>
    {
        Box::pin(async move { Ok(self.session.as_ref().map(|s| s.id.clone())) })
    }
}

impl std::fmt::Debug for SessionGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionGuard")
            .field("name", &self.name)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

/// `markEmailAsUnverified` hook contract (FR-311).
///
/// Clears `email_verified_at` on the authenticated user so downstream
/// `MustVerifyEmail` flows re-send verification.
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

    /// Default policy is JSON with a hyphenated `-session-` prefix.
    #[test]
    fn default_policy_is_json_and_hyphenated() {
        let policy = SessionPolicy::default();
        assert_eq!(policy.serialization, "json");
        assert!(policy.validate_prefix().is_ok());
        assert!(policy.prefix.contains("-session-"));
        assert!(!policy.prefix.contains("_session_"));
    }

    /// Underscore prefixes are rejected.
    #[test]
    fn underscore_prefix_rejected() {
        let policy = SessionPolicy {
            prefix: "rustavel_session_".into(),
            ..SessionPolicy::default()
        };
        assert!(policy.validate_prefix().is_err());
    }

    /// Cache prefixes must use the hyphenated `-cache-` marker (TC-M3-07).
    #[test]
    fn cache_prefix_validation_rejects_underscores() {
        assert!(SessionPolicy::validate_cache_prefix("rustavel-cache-").is_ok());
        let err = SessionPolicy::validate_cache_prefix("rustavel_cache_").expect_err("rejected");
        assert!(matches!(err, AuthError::Disabled(_)));
    }

    /// The allow-list trait rejects unlisted types like the inherent method.
    #[test]
    fn deserialization_allow_list_trait_gates_types() {
        use crate::session::DeserializationAllowList;
        let policy = SessionPolicy::with_classes(vec!["App::UserDto".into()]);
        assert!(policy.allow("App::UserDto").is_ok());
        assert_eq!(
            <SessionPolicy as DeserializationAllowList>::allow(&policy, "App::AdminDto"),
            Err(crate::error::SerializationError::NotAllowed {
                type_name: "App::AdminDto".into()
            })
        );
    }

    /// Unlisted types are rejected by the allow-list gate.
    #[test]
    fn allow_list_rejects_unlisted_type() {
        let policy = SessionPolicy::with_classes(vec!["App::UserDto".into()]);
        assert!(policy.allow("App::UserDto").is_ok());
        assert_eq!(
            policy.allow("App::AdminDto"),
            Err(crate::error::SerializationError::NotAllowed {
                type_name: "App::AdminDto".into()
            })
        );
    }

    /// Storing a user in the session makes `user`/`id` resolve it.
    #[test]
    fn stored_session_user_resolves_identity() {
        let mut guard = SessionGuard::new(SessionPolicy::default());
        guard.store(SessionUser {
            id: "user-1".into(),
            email: Some("ada@example.com".into()),
        });
        let user = futures_user(&guard);
        assert_eq!(user.id, "user-1");
        assert_eq!(guard.policy().prefix, "rustavel-session-");
    }

    /// Resolve the current user (sync test helper — Guard::user is async).
    fn futures_user(guard: &SessionGuard) -> AuthUser {
        use crate::guard::Guard;
        let fut = guard.user();
        let user = tokio_test_block_on(fut);
        user.expect("user resolves").expect("stored user present")
    }

    /// Minimal single-threaded block-on for tests without a tokio runtime.
    fn tokio_test_block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime builds")
            .block_on(fut)
    }

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
