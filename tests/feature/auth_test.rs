//! Feature tests for registration, authentication, and the session guard.
//!
//! The validation tests exercise the generated concern directly; the session
//! tests drive the *real* `rustasea::auth::SessionGuard` over the in-memory
//! store, so they cover the login → parse → logout lifecycle the generated auth
//! actions compose without depending on any unfinished scaffold stub.

use std::sync::Arc;

use example_app::app::concerns::password_validation_rules;
use rustasea::auth::users::{AuthUserRecord, MemoryUserRegistry};
use rustasea::auth::verify::{Argon2Verifier, PasswordVerifier};
use rustasea::auth::{AuthError, Credentials, Guard, SessionGuard, SessionPolicy};

/// Registration rejects passwords shorter than the configured minimum.
#[test]
fn registration_rejects_short_passwords() {
    assert!(password_validation_rules::validate("short").is_err());
}

/// Registration accepts a password that meets the minimum length.
#[test]
fn registration_accepts_a_long_password() {
    assert!(password_validation_rules::validate("correct-horse-battery").is_ok());
}

/// The Fortify `registration` gate reads `config/fortify.toml`.
#[test]
fn fortify_registration_gate_is_queryable() {
    // The generated config enables registration, so the gate reports `true`.
    assert!(crate::fortify_feature_enabled("registration"));
    // Unknown feature names fail closed.
    assert!(!crate::fortify_feature_enabled("not-a-real-feature"));
}

/// Build a session guard over the in-memory store with one seeded user.
fn guard_with_user(password: &str) -> SessionGuard {
    let verifier = Argon2Verifier::new();
    let password_hash = verifier.hash(password).expect("argon2 hashing succeeds");
    let registry = Arc::new(MemoryUserRegistry::default());
    registry.seed(AuthUserRecord {
        id: "user-1".to_string(),
        email: "ada@example.com".to_string(),
        password_hash,
        email_verified_at: None,
        timezone: None,
    });
    SessionGuard::new(SessionPolicy::default()).with_lookup(registry)
}

/// Login → parse → logout over the real session guard (registration-gated).
#[tokio::test]
async fn session_guard_login_parse_logout_lifecycle() {
    crate::skip_unless_fortify_has!("registration");

    let guard = guard_with_user("correct-horse-battery");
    let token = guard
        .login(&Credentials {
            email: "ada@example.com".to_string(),
            password: "correct-horse-battery".to_string(),
        })
        .await
        .expect("login succeeds");
    assert_eq!(token.token_type, "Session");

    let principal = guard
        .parse(&token.access_token)
        .await
        .expect("parse resolves the principal");
    assert_eq!(principal.id, "user-1");
    assert_eq!(principal.guard, "session");

    guard
        .logout(&token.access_token)
        .await
        .expect("logout succeeds");
    assert!(
        guard.parse(&token.access_token).await.is_err(),
        "a session replayed after logout must be rejected"
    );
}

/// Wrong credentials are rejected without issuing a session.
#[tokio::test]
async fn session_guard_rejects_bad_credentials() {
    let guard = guard_with_user("correct-horse-battery");
    let error = guard
        .login(&Credentials {
            email: "ada@example.com".to_string(),
            password: "wrong-password".to_string(),
        })
        .await
        .expect_err("bad credentials must be rejected");
    assert_eq!(error, AuthError::BadCredentials);
}
