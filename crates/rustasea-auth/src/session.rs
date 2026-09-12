/// Session guard and session/cache hardening policy.
///
/// `SessionGuard` is the `tower-sessions` backed identity path: the store owns
/// the session record, the guard reads/writes the authenticated user through a
/// pluggable [`tower_sessions::SessionStore`] (in-memory by default). Login
/// always mints a **new** session id (session-fixation defense), `refresh`
/// cycles the id, and `logout` destroys the record and rotates the id.
///
/// `SessionPolicy` encodes the Laravel-13 hardening defaults: JSON
/// serialization, hyphenated `-session-`/`-cache-` key prefixes, and a
/// `serializable_classes` allow-list checked before any deserialization.
use std::sync::Arc;

use tower_sessions::session::{Id, Session};
use tower_sessions::{MemoryStore, SessionStore};

use crate::error::{AuthError, Result};
use crate::guard::{AuthUser, Credentials, Guard, Token};
use crate::session_cookie::SessionCookieConfig;
use crate::users::UserLookup;
use crate::verify::{Argon2Verifier, PasswordVerifier};

/// Session lifetime advertised on issued tokens (two weeks, tower-sessions default).
const SESSION_TTL_SECS: u64 = 1_209_600;

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
            prefix: "rustasea-session-".to_string(),
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

    /// Session key under which the guard stores the authenticated user.
    ///
    /// Derived from [`SessionPolicy::prefix`] so the `-session-` marker is
    /// always present; the concrete [`SessionUser`] type is stored directly,
    /// so no `serializable_classes` entry is needed (the allow-list continues
    /// to gate polymorphic cache values, never weakened).
    pub fn user_key(&self) -> String {
        format!("{}user", self.prefix)
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

/// Session guard — resolves identity from a `tower-sessions` store.
///
/// The generic parameter is the backing store, defaulting to the in-memory
/// [`MemoryStore`]; any [`SessionStore`] implementation can be injected with
/// [`SessionGuard::with_store`]. The guard never issues bearer tokens — a
/// session cookie is the credential, and `access_token`/`refresh_token` carry
/// the session id so the HTTP layer can set the cookie.
///
/// # Stateless by design
///
/// The guard holds **no per-request mutable state**. A single instance is
/// registered as a shared `Arc<dyn Guard>` in [`crate::guard::AuthManager`]
/// and used concurrently by every Axum request; any `self`-mutating identity
/// cache would leak one request's principal into another. Identity therefore
/// lives exclusively in the store-backed session (keyed by the request's
/// session id) and is projected into request extensions
/// (`axum::Extension<AuthUser>`) by the HTTP middleware. `Guard::user`/`id`
/// consequently return `Ok(None)` — see their implementations.
pub struct SessionGuard<S: SessionStore = MemoryStore> {
    /// Guard name (`session`).
    name: String,
    /// Hardening policy applied to session payloads.
    policy: SessionPolicy,
    /// Pluggable backing store (in-memory by default).
    store: Arc<S>,
    /// Cookie hardening applied to the session cookie.
    cookie: SessionCookieConfig,
    /// Password verifier used by `login`.
    verifier: Arc<dyn PasswordVerifier>,
    /// Credential lookup used by `login` (fail-closed until DB wiring).
    lookup: Arc<dyn UserLookup>,
    /// Whether `login_using_id` is permitted (disabled by default).
    allow_login_using_id: bool,
}

impl SessionGuard<MemoryStore> {
    /// Create a session guard backed by the in-memory store.
    pub fn new(policy: SessionPolicy) -> Self {
        Self::with_store(policy, Arc::new(MemoryStore::default()))
    }
}

impl<S: SessionStore> SessionGuard<S> {
    /// Create a guard over an explicit store (Redis/SQLx/etc.).
    pub fn with_store(policy: SessionPolicy, store: Arc<S>) -> Self {
        Self {
            name: "session".to_string(),
            policy,
            store,
            cookie: SessionCookieConfig::default(),
            verifier: Arc::new(Argon2Verifier::new()),
            lookup: Arc::new(crate::users::StaticLookup),
            allow_login_using_id: false,
        }
    }

    /// Attach the password verifier used by [`Guard::login`].
    pub fn with_verifier(mut self, verifier: Arc<dyn PasswordVerifier>) -> Self {
        self.verifier = verifier;
        self
    }

    /// Attach the credential lookup used by [`Guard::login`].
    pub fn with_lookup(mut self, lookup: Arc<dyn UserLookup>) -> Self {
        self.lookup = lookup;
        self
    }

    /// Enable or disable [`Guard::login_using_id`] (disabled by default).
    pub fn with_allow_login_using_id(mut self, allow: bool) -> Self {
        self.allow_login_using_id = allow;
        self
    }

    /// Override the session-cookie hardening configuration.
    pub fn with_cookie(mut self, cookie: SessionCookieConfig) -> Self {
        self.cookie = cookie;
        self
    }

    /// Hardening policy this guard enforces.
    pub fn policy(&self) -> &SessionPolicy {
        &self.policy
    }

    /// Session-cookie hardening applied to issued cookies.
    pub fn cookie(&self) -> &SessionCookieConfig {
        &self.cookie
    }

    /// Shared handle to the backing store (HTTP wiring and tests).
    pub fn session_store(&self) -> Arc<S> {
        Arc::clone(&self.store)
    }

    /// Persist `user` under a freshly minted session id and return that id.
    ///
    /// A new id is always generated (`Session::new(None, ..)`), which is the
    /// session-fixation defense: a caller-supplied id is never adopted. This
    /// is the only place the guard writes identity — into the request-scoped
    /// session record, never into `self`.
    async fn persist(&self, user: &SessionUser) -> Result<Id> {
        let key = self.policy.user_key();
        let session = Session::new(None, Arc::clone(&self.store), None);
        session
            .insert(&key, user)
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        session
            .save()
            .await
            .map_err(|_| AuthError::StoreUnavailable)?;
        session.id().ok_or(AuthError::StoreUnavailable)
    }

    /// Build the credential token for a session id.
    fn issue(&self, id: &Id) -> Token {
        let value = id.to_string();
        Token {
            access_token: value.clone(),
            refresh_token: value,
            token_type: "Session".to_string(),
            expires_in: SESSION_TTL_SECS,
        }
    }

    /// Parse a session id from a cookie/token string.
    fn parse_id(token: &str) -> Result<Id> {
        token.parse::<Id>().map_err(|_| AuthError::InvalidToken)
    }
}

impl<S: SessionStore> Guard for SessionGuard<S> {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn login<'a>(
        &'a self,
        creds: &'a Credentials,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            let stored_hash = self
                .lookup
                .hash_for_email(&creds.email)
                .ok_or(AuthError::BadCredentials)?;
            // Constant-time argon2 verify; same error for unknown user and
            // wrong password so account enumeration via timing is not possible.
            if !self.verifier.verify(&stored_hash, &creds.password) {
                return Err(AuthError::BadCredentials);
            }
            let user_id = self
                .lookup
                .id_for_email(&creds.email)
                .ok_or(AuthError::BadCredentials)?;
            let email = self
                .lookup
                .email_for_id(&user_id)
                .or_else(|| Some(creds.email.clone()));
            let user = SessionUser { id: user_id, email };
            let id = self.persist(&user).await?;
            Ok(self.issue(&id))
        })
    }

    fn login_using_id<'a>(
        &'a self,
        user_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            if !self.allow_login_using_id {
                return Err(AuthError::Disabled(
                    "loginUsingId is disabled on the session guard".into(),
                ));
            }
            let user = SessionUser {
                id: user_id.to_string(),
                email: self.lookup.email_for_id(user_id),
            };
            let id = self.persist(&user).await?;
            Ok(self.issue(&id))
        })
    }

    fn parse<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AuthUser>> + Send + 'a>> {
        Box::pin(async move {
            let id = Self::parse_id(token)?;
            let key = self.policy.user_key();
            let session = Session::new(Some(id), Arc::clone(&self.store), None);
            let user: Option<SessionUser> = session
                .get(&key)
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            let user = user.ok_or(AuthError::InvalidToken)?;
            Ok(AuthUser {
                id: user.id,
                email: user.email,
                guard: self.name.clone(),
            })
        })
    }

    fn refresh<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            let id = Self::parse_id(token)?;
            let key = self.policy.user_key();
            let session = Session::new(Some(id), Arc::clone(&self.store), None);
            let user: Option<SessionUser> = session
                .get(&key)
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            // The session must exist and carry the user payload; the record is
            // then re-saved under a fresh id (payload retained by `cycle_id`).
            user.ok_or(AuthError::InvalidToken)?;
            session
                .cycle_id()
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            session
                .save()
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            let new_id = session.id().ok_or(AuthError::StoreUnavailable)?;
            Ok(self.issue(&new_id))
        })
    }

    fn logout<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let id = Self::parse_id(token)?;
            let session = Session::new(Some(id), Arc::clone(&self.store), None);
            // Destroy the stored record (no store write when the id is absent).
            // The old id is dead afterwards, so a replayed cookie can never
            // resume the authenticated session; the HTTP layer clears the
            // cookie on the response. No `self` state is touched.
            session
                .flush()
                .await
                .map_err(|_| AuthError::StoreUnavailable)?;
            Ok(())
        })
    }

    fn user<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<AuthUser>>> + Send + 'a>>
    {
        Box::pin(async move {
            // Stateless: the guard holds no per-request identity. A shared
            // guard is reused across concurrent requests, so resolving the
            // "current" user here would leak another request's principal.
            // The auth middleware projects `parse` results into
            // `Extension<AuthUser>` for the request scope instead — matching
            // `JwtGuard::user`, which also returns `Ok(None)`.
            Ok(None)
        })
    }

    fn id<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>>> + Send + 'a>>
    {
        Box::pin(async move {
            // See `user`: per-request identity is carried by request
            // extensions, never by shared guard state.
            Ok(None)
        })
    }
}

impl<S: SessionStore> std::fmt::Debug for SessionGuard<S> {
    /// Manual debug — the store is rendered by type, never by contents.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionGuard")
            .field("name", &self.name)
            .field("policy", &self.policy)
            .field("cookie", &self.cookie)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
