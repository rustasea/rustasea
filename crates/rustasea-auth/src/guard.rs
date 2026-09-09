/// Guard trait and auth manager.
///
/// A guard verifies a credential/cookie and yields the authenticated user.
/// `AuthManager` registers named guards (`Auth::extend`) and resolves them
/// with typed `GuardMismatch` diagnostics when the name is unknown.
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{AuthError, Result};

/// Authenticated principal returned by `Guard::parse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUser {
    /// Primary key (UUID string) of the authenticated record.
    pub id: String,
    /// Login identifier (email for JWT/session guards).
    pub email: Option<String>,
    /// Guard name that produced this principal.
    pub guard: String,
}

impl AuthUser {
    /// Build a principal for a custom guard's `parse`/`user` results.
    ///
    /// # Custom guard example
    ///
    /// ```rust
    /// use rustasea_auth::{AuthUser, Guard, AuthError};
    ///
    /// /// Minimal API-key guard: `"secret"` is the only valid key.
    /// pub struct ApiKeyGuard;
    ///
    /// impl Guard for ApiKeyGuard {
    ///     fn name(&self) -> &str { "api" }
    ///     fn login<'a>(&'a self, _: &'a rustasea_auth::Credentials)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<rustasea_auth::Token>> + Send + 'a>>
    ///     { Box::pin(async move { Err(AuthError::BadCredentials) }) }
    ///     fn login_using_id<'a>(&'a self, _: &'a str)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<rustasea_auth::Token>> + Send + 'a>>
    ///     { Box::pin(async move { Err(AuthError::BadCredentials) }) }
    ///     fn parse<'a>(&'a self, token: &'a str)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<AuthUser>> + Send + 'a>>
    ///     {
    ///         Box::pin(async move {
    ///             if token == "secret" {
    ///                 Ok(AuthUser::new("user-1", Some("api@example.com"), "api"))
    ///             } else {
    ///                 Err(AuthError::InvalidToken)
    ///             }
    ///         })
    ///     }
    ///     fn refresh<'a>(&'a self, _: &'a str)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<rustasea_auth::Token>> + Send + 'a>>
    ///     { Box::pin(async move { Err(AuthError::BadCredentials) }) }
    ///     fn logout<'a>(&'a self, _: &'a str)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<()>> + Send + 'a>>
    ///     { Box::pin(async move { Ok(()) }) }
    ///     fn user<'a>(&'a self)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<Option<AuthUser>>> + Send + 'a>>
    ///     { Box::pin(async move { Ok(None) }) }
    ///     fn id<'a>(&'a self)
    ///         -> std::pin::Pin<Box<dyn std::future::Future<Output = rustasea_auth::error::Result<Option<String>>> + Send + 'a>>
    ///     { Box::pin(async move { Ok(None) }) }
    /// }
    /// ```
    pub fn new(
        id: impl Into<String>,
        email: Option<impl Into<String>>,
        guard: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            email: email.map(Into::into),
            guard: guard.into(),
        }
    }
}

/// Credentials accepted by `Guard::login`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    /// Login identifier (email).
    pub email: String,
    /// Plaintext password to verify against the stored hash.
    pub password: String,
}

/// Token pair issued by `Guard::login`/`refresh`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// JWT access token.
    pub access_token: String,
    /// JWT refresh token (longer-lived rotation credential).
    pub refresh_token: String,
    /// Token type, always `Bearer` for the JWT guard.
    pub token_type: String,
    /// Access-token lifetime in seconds.
    pub expires_in: u64,
}

impl Token {
    /// Build a token pair with the standard `Bearer` token type.
    pub fn bearer(access_token: String, refresh_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }
}

/// Identity-provider contract implemented by the JWT and session guards.
///
/// All methods are async: the session store and future database-backed
/// user providers are inherently async even when the JWT math is not.
/// Futures are boxed so the trait stays object-safe (`dyn Guard`).
/// Each method borrows `self` and its argument for one shared lifetime `'a`
/// that also bounds the returned future.
pub trait Guard: Send + Sync {
    /// Name this guard is registered under (`jwt`, `session`, custom).
    fn name(&self) -> &str;

    /// Verify credentials and issue a token pair.
    fn login<'a>(
        &'a self,
        creds: &'a Credentials,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>>;

    /// Issue a token for a known user id without a password check.
    ///
    /// Internal/testing helper — gate behind `allow_loginUsingId`.
    fn login_using_id<'a>(
        &'a self,
        user_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>>;

    /// Verify a token/cookie and resolve the authenticated user.
    fn parse<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AuthUser>> + Send + 'a>>;

    /// Rotate a valid (possibly near-expired) token into a fresh pair.
    fn refresh<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>>;

    /// Invalidate a token/cookie (deny-list or store delete).
    fn logout<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>>;

    /// Resolve the current user without an explicit token (session-cookie path).
    fn user<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<AuthUser>>> + Send + 'a>>;

    /// Return the current user id when authenticated.
    fn id<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>>> + Send + 'a>>;
}

/// Builder returned by `Auth::extend` for custom guard registration.
///
/// Holds the real `AuthManager` registry: every `register` call inserts into
/// that shared map, so a guard registered through the registrar resolves
/// through `AuthManager::guard` on the original manager.
pub struct GuardRegistrar {
    manager: Arc<AuthManager>,
    /// Manager default guard name at registration time.
    pub default_guard: String,
}

impl GuardRegistrar {
    /// Register a factory that builds the guard from application state.
    ///
    /// The factory is invoked lazily on first `guard(name)` resolve; a
    /// duplicate registration of the same guard name is rejected with
    /// `GuardMismatch` (`expected` = existing default, `actual` = duplicate)
    /// instead of silently replacing the first guard (ADR-005 first-wins).
    pub fn register(
        &self,
        factory: impl Fn() -> Arc<dyn Guard> + Send + Sync + 'static,
    ) -> Result<()> {
        let name = self.default_guard.clone();
        let mut guards = self
            .manager
            .guards
            .write()
            .map_err(|_| AuthError::Token("guard registry lock poisoned".to_string()))?;
        if guards.contains_key(&name) {
            let expected = self.manager.default.clone();
            return Err(AuthError::GuardMismatch {
                expected,
                actual: name,
            });
        }
        guards.insert(name, factory());
        Ok(())
    }
}

impl std::fmt::Debug for GuardRegistrar {
    /// Manual debug — the manager is rendered via its guard names.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuardRegistrar")
            .field("manager", &self.manager)
            .field("default_guard", &self.default_guard)
            .finish()
    }
}

/// Thread-safe registry of named guards.
///
/// Per ADR-005 there are no global facades: an `AuthManager` lives inside
/// `AppState` and is injected explicitly via `axum::extract::State`.
#[derive(Default)]
pub struct AuthManager {
    /// Shared guard registry: clones (and registrars) observe the same map
    /// through this `Arc`, so a guard registered on any handle resolves on
    /// every other handle.
    guards: Arc<std::sync::RwLock<HashMap<String, Arc<dyn Guard>>>>,
    /// Default guard name used when `Auth::guard` is called without a name.
    pub default: String,
}

impl AuthManager {
    /// Create an empty manager with `jwt` as the default guard.
    pub fn new() -> Self {
        Self {
            guards: Arc::new(std::sync::RwLock::new(HashMap::new())),
            default: "jwt".to_string(),
        }
    }

    /// Create an empty manager with a custom default guard name.
    pub fn with_default(default: impl Into<String>) -> Self {
        Self {
            guards: Arc::new(std::sync::RwLock::new(HashMap::new())),
            default: default.into(),
        }
    }

    /// Register a guard instance under its own `Guard::name`.
    ///
    /// First registration wins: a duplicate `Guard::name` silently replaces
    /// nothing because this is the low-level insert used by boot wiring;
    /// prefer [`AuthManager::extend`] when duplicate names must be rejected.
    pub fn register(&self, guard: Arc<dyn Guard>) -> Result<()> {
        let name = guard.name().to_string();
        let mut guards = self
            .guards
            .write()
            .map_err(|_| AuthError::Token("guard registry lock poisoned".to_string()))?;
        guards.insert(name, guard);
        Ok(())
    }

    /// `Auth::extend`-style registration: name plus lazy factory.
    ///
    /// Registers the factory immediately under `name` and returns a
    /// `GuardRegistrar` bound to this manager so later `register` calls land
    /// in the same real registry. A duplicate name is rejected here and by
    /// `GuardRegistrar::register`.
    ///
    /// # Custom guard registration (end-to-end)
    ///
    /// Implement [`Guard`], then register an `Arc`-producing factory under a
    /// name and resolve it through the manager:
    ///
    /// ```rust,ignore
    /// // bootstrap/providers.rs
    /// let auth = AuthManager::new();
    /// auth.extend("api", || std::sync::Arc::new(ApiKeyGuard::new()))?;
    /// // app/http/middleware/authenticate.rs (handler side)
    /// let guard = auth.guard("api")?;              // resolves the custom guard
    /// let user = guard.parse("secret").await?;     // AuthUser { id: "user-1", .. }
    /// ```
    ///
    /// `Auth::extend` fails with `AuthError::GuardMismatch` when the name is
    /// already registered (first-wins registry, ADR-005).
    pub fn extend(
        &self,
        name: impl Into<String>,
        factory: impl Fn() -> Arc<dyn Guard> + Send + Sync + 'static,
    ) -> Result<GuardRegistrar> {
        let name = name.into();
        let registrar_default = name.clone();
        // Bound the registrar to the real (shared) registry *before* taking
        // the write lock — `Arc::clone` never contends with the lock.
        let manager = Arc::new(self.clone());
        let mut guards = self
            .guards
            .write()
            .map_err(|_| AuthError::Token("guard registry lock poisoned".to_string()))?;
        if guards.contains_key(&name) {
            let expected = self.default.clone();
            return Err(AuthError::GuardMismatch {
                expected,
                actual: name,
            });
        }
        guards.insert(name.clone(), factory());
        Ok(GuardRegistrar {
            manager,
            default_guard: registrar_default,
        })
    }

    /// Resolve a guard by name; unknown names yield `GuardMismatch`.
    pub fn guard(&self, name: &str) -> Result<Arc<dyn Guard>> {
        let guards = self
            .guards
            .read()
            .map_err(|_| AuthError::Token("guard registry lock poisoned".to_string()))?;
        let expected = self.default.clone();
        match guards.get(name) {
            Some(g) => Ok(Arc::clone(g)),
            None => Err(AuthError::GuardMismatch {
                expected,
                actual: name.to_string(),
            }),
        }
    }

    /// Resolve the default guard.
    pub fn default_guard(&self) -> Result<Arc<dyn Guard>> {
        let name = self.default.clone();
        self.guard(&name)
    }

    /// Resolve the default guard name (`expected` in `GuardMismatch`).
    pub fn default_guard_name(&self) -> &str {
        &self.default
    }

    /// Synchronous authorization gate backing the `#[authorize]` macro.
    ///
    /// The macro expands to an `ensure_authorized` wrapper (see the
    /// `rustasea-macros` crate) that calls this helper with the guard name
    /// parsed from `#[authorize("update", User)]` on a `#[middleware("auth:<guard>")]`
    /// handler. Unknown guard names surface the typed `GuardMismatch` error
    /// (TC-M3-02), keeping the fail-closed posture.
    pub fn authorize_gate(&self, guard: &str) -> Result<Arc<dyn Guard>> {
        self.guard(guard)
    }

    /// List registered guard names.
    pub fn guard_names(&self) -> Vec<String> {
        self.guards
            .read()
            .map(|g| g.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Clone for AuthManager {
    /// Clone shares the guard registry (`guards` is `Arc`d).
    fn clone(&self) -> Self {
        Self {
            guards: Arc::clone(&self.guards),
            default: self.default.clone(),
        }
    }
}

impl std::fmt::Debug for AuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthManager")
            .field("guards", &self.guard_names())
            .field("default", &self.default)
            .finish()
    }
}

/// Convenience alias mirroring Laravel's `Auth` facade entry point.
///
/// Not a global: construct with `AuthManager::new()` inside `AppState`.
pub type Auth = AuthManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::{JwtConfig, JwtGuard};
    use crate::verify::PasswordVerifier;

    struct NoopVerifier;

    impl PasswordVerifier for NoopVerifier {
        fn verify(&self, _hash: &str, _password: &str) -> bool {
            true
        }

        fn hash(&self, _password: &str) -> crate::error::Result<String> {
            Err(AuthError::Hash("NoopVerifier does not hash".into()))
        }
    }

    fn jwt_guard() -> Arc<dyn Guard> {
        let cfg = JwtConfig {
            secret: "test-secret-that-is-long-enough".into(),
            access_ttl_secs: 3600,
            refresh_ttl_secs: 86400,
            issuer: "rustasea".into(),
            audience: "rustasea-app".into(),
            allow_login_using_id: true,
        };
        Arc::new(JwtGuard::new(cfg, Arc::new(NoopVerifier)))
    }

    /// A guard registry that knows only `jwt` reports a typed mismatch.
    #[test]
    fn unknown_guard_reports_mismatch() {
        let manager = AuthManager::new();
        manager.register(jwt_guard()).expect("register succeeds");
        let err = manager
            .guard("api")
            .err()
            .expect("unknown guard must error");
        assert_eq!(
            err,
            AuthError::GuardMismatch {
                expected: "jwt".to_string(),
                actual: "api".to_string()
            }
        );
    }

    /// Registered guards resolve by name.
    #[test]
    fn registered_guard_resolves() {
        let manager = AuthManager::new();
        manager.register(jwt_guard()).expect("register succeeds");
        let guard = manager.guard("jwt").expect("registered guard resolves");
        assert_eq!(guard.name(), "jwt");
    }

    /// `Auth::extend` registers into the real registry and its registrar
    /// resolves through the original manager (no detached empty manager).
    #[test]
    fn extend_registrar_shares_real_registry() {
        let manager = AuthManager::new();
        let registrar = manager
            .extend("api", jwt_guard)
            .expect("extend registers api");
        // The registrar's own register lands in the manager's registry; the
        // name was already inserted eagerly by `extend`, so re-registering
        // the same name is a duplicate.
        let err = registrar
            .register(jwt_guard)
            .expect_err("duplicate must error");
        assert_eq!(
            err,
            AuthError::GuardMismatch {
                expected: "jwt".to_string(),
                actual: "api".to_string()
            }
        );
        // The guard registered by `extend` resolves on the original manager.
        let guard = manager.guard("api").expect("api resolves on manager");
        assert_eq!(guard.name(), "jwt");
    }

    /// `Auth::extend` rejects a duplicate name at registration time.
    #[test]
    fn extend_duplicate_name_errors() {
        let manager = AuthManager::new();
        manager.extend("api", jwt_guard).expect("first extend wins");
        let err = manager
            .extend("api", jwt_guard)
            .expect_err("duplicate extend must error");
        assert_eq!(
            err,
            AuthError::GuardMismatch {
                expected: "jwt".to_string(),
                actual: "api".to_string()
            }
        );
    }
}
