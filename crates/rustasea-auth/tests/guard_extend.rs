//! End-to-end tests for custom guard registration (`Auth::extend`, FR-300).
//!
//! Covers the runtime half of FS-M3-01: a custom `Guard` implementation
//! registered under a name resolves through `AuthManager::guard` /
//! `authorize_gate`, parses a credential into the typed `AuthUser` the guard
//! produces, and unknown names keep failing closed with `GuardMismatch`
//! (TC-M3-02).

use std::sync::Arc;

use rustasea_auth::error::Result;
use rustasea_auth::{AuthError, AuthManager, AuthUser, Credentials, Guard, Token};

/// Minimal API-key guard: `"secret"` is the only valid key.
pub struct ApiKeyGuard;

impl Guard for ApiKeyGuard {
    fn name(&self) -> &str {
        "api"
    }

    fn login<'a>(
        &'a self,
        _creds: &'a Credentials,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move { Err(AuthError::BadCredentials) })
    }

    fn login_using_id<'a>(
        &'a self,
        _user_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move { Err(AuthError::BadCredentials) })
    }

    fn parse<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AuthUser>> + Send + 'a>> {
        let guard = self.name().to_string();
        Box::pin(async move {
            if token == "secret" {
                Ok(AuthUser::new(
                    "user-1",
                    Some("api@example.com"),
                    guard.clone(),
                ))
            } else {
                Err(AuthError::InvalidToken)
            }
        })
    }

    fn refresh<'a>(
        &'a self,
        _token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move { Err(AuthError::BadCredentials) })
    }

    fn logout<'a>(
        &'a self,
        _token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn user<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<AuthUser>>> + Send + 'a>>
    {
        Box::pin(async move { Ok(None) })
    }

    fn id<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>>> + Send + 'a>>
    {
        Box::pin(async move { Ok(None) })
    }
}

/// `Auth::extend("api", || Arc::new(ApiKeyGuard))` registers eagerly and the
/// guard resolves through the manager; `parse` yields the guard's typed user.
#[tokio::test]
async fn extend_custom_guard_registers_and_parses_end_to_end() {
    let manager = AuthManager::new();
    manager
        .extend("api", || Arc::new(ApiKeyGuard))
        .expect("custom guard registers");

    // Resolution through the manager (FS-M3-01) and the authorize gate (#2).
    let guard = manager.guard("api").expect("custom guard resolves");
    assert_eq!(guard.name(), "api");
    assert_eq!(manager.default_guard_name(), "jwt");
    let gated = manager
        .authorize_gate("api")
        .expect("authorize_gate resolves the registered custom guard");
    assert_eq!(gated.name(), "api");

    let user = gated
        .parse("secret")
        .await
        .expect("parse succeeds with the registered factory");
    assert_eq!(user.id, "user-1");
    assert_eq!(user.email.as_deref(), Some("api@example.com"));
    assert_eq!(user.guard, "api");

    // The built-in jwt guard is not registered by `AuthManager::new`, so
    // unknown names keep failing closed with a typed mismatch (TC-M3-02).
    let err = guard.parse("wrong").await.expect_err("bad key rejected");
    assert_eq!(err, AuthError::InvalidToken);
}

/// Unknown guard names surface `GuardMismatch{expected, actual}` from both
/// `guard` and the synchronous `authorize_gate` backing the `#[authorize]`
/// macro (TC-M3-02, FS-M3-06).
#[test]
fn unknown_guard_reports_typed_mismatch_through_authorize_gate() {
    let manager = AuthManager::new();
    let err = match manager.guard("api") {
        Ok(_) => panic!("unregistered guard must error"),
        Err(e) => e,
    };
    assert_eq!(
        err,
        AuthError::GuardMismatch {
            expected: "jwt".to_string(),
            actual: "api".to_string()
        }
    );

    let err = match manager.authorize_gate("api") {
        Ok(_) => panic!("authorize gate fails closed"),
        Err(e) => e,
    };
    assert_eq!(
        err,
        AuthError::GuardMismatch {
            expected: "jwt".to_string(),
            actual: "api".to_string()
        }
    );
    assert_eq!(manager.default_guard_name(), "jwt");
}
