//! Unit tests for the session guard and hardening policy.
use super::*;
use crate::guard::Guard;
use crate::users::{AuthUserRecord, MemoryUserRegistry};
use std::future::Future;

/// Verifier that accepts every password (test double for argon2 cost).
struct AlwaysVerify;

impl PasswordVerifier for AlwaysVerify {
    fn verify(&self, _hash: &str, _password: &str) -> bool {
        true
    }

    fn hash(&self, _password: &str) -> Result<String> {
        Err(AuthError::Hash("AlwaysVerify does not hash".into()))
    }
}

/// Credentials for the seeded `ada@example.com` test user.
fn creds() -> Credentials {
    Credentials {
        email: "ada@example.com".into(),
        password: "s3cr3tPass".into(),
    }
}

/// Guard over an in-memory store with one seeded user and `loginUsingId` on.
fn guard_with_registry() -> (SessionGuard<MemoryStore>, Arc<MemoryUserRegistry>) {
    let registry = Arc::new(MemoryUserRegistry::default());
    registry.seed(AuthUserRecord {
        id: "user-1".into(),
        email: "ada@example.com".into(),
        password_hash: "phc$hash".into(),
        email_verified_at: None,
    });
    let guard = SessionGuard::new(SessionPolicy::default())
        .with_verifier(Arc::new(AlwaysVerify))
        .with_lookup(registry.clone())
        .with_allow_login_using_id(true);
    (guard, registry)
}

/// Minimal single-threaded block-on for tests without a tokio runtime.
fn tokio_test_block_on<F: Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime builds")
        .block_on(fut)
}

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
        prefix: "rustasea_session_".into(),
        ..SessionPolicy::default()
    };
    assert!(policy.validate_prefix().is_err());
}

/// Cache prefixes must use the hyphenated `-cache-` marker (TC-M3-07).
#[test]
fn cache_prefix_validation_rejects_underscores() {
    assert!(SessionPolicy::validate_cache_prefix("rustasea-cache-").is_ok());
    let err = SessionPolicy::validate_cache_prefix("rustasea_cache_").expect_err("rejected");
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

/// The user key is derived from the hyphenated prefix.
#[test]
fn user_key_keeps_session_marker() {
    assert_eq!(SessionPolicy::default().user_key(), "rustasea-session-user");
}

/// The guard is stateless: `user`/`id` never resolve from shared state.
///
/// A shared guard serves every concurrent request, so exposing a "current"
/// identity would leak one request's principal into another. Identity is
/// carried by request extensions instead (see `SessionGuard::user`).
#[test]
fn guard_user_and_id_are_stateless() {
    let guard = SessionGuard::new(SessionPolicy::default());
    assert!(tokio_test_block_on(guard.user())
        .expect("user resolves")
        .is_none());
    assert!(tokio_test_block_on(guard.id())
        .expect("id resolves")
        .is_none());
}

/// Login persists the user in the store and `parse` resolves the identity.
#[test]
fn login_persists_session_and_parse_resolves() {
    let (guard, _registry) = guard_with_registry();
    let token = tokio_test_block_on(guard.login(&creds())).expect("login succeeds");
    assert_eq!(token.token_type, "Session");

    let id: Id = token
        .access_token
        .parse()
        .expect("token carries a session id");
    let stored = tokio_test_block_on(guard.session_store().load(&id))
        .expect("store load")
        .expect("session record present");
    let key = guard.policy().user_key();
    let value = stored.data.get(&key).cloned().expect("user key present");
    let user: SessionUser = serde_json::from_value(value).expect("stored user decodes");
    assert_eq!(user.id, "user-1");
    assert_eq!(user.email.as_deref(), Some("ada@example.com"));

    let principal = tokio_test_block_on(guard.parse(&token.access_token)).expect("parse resolves");
    assert_eq!(principal.id, "user-1");
    assert_eq!(principal.guard, "session");
}

/// Logout destroys the stored record so a replayed cookie cannot resume it.
#[test]
fn logout_destroys_session_and_replay_fails() {
    let (guard, _registry) = guard_with_registry();
    let token = tokio_test_block_on(guard.login(&creds())).expect("login succeeds");
    let old: Id = token.access_token.parse().expect("session id");
    assert!(tokio_test_block_on(guard.session_store().load(&old))
        .expect("load")
        .is_some());

    tokio_test_block_on(guard.logout(&token.access_token)).expect("logout succeeds");

    // Destroyed: the old id no longer resolves in the store.
    assert!(tokio_test_block_on(guard.session_store().load(&old))
        .expect("load")
        .is_none());
    // Rotation: the dead id is rejected on replay.
    let err = tokio_test_block_on(guard.parse(&token.access_token)).expect_err("replay rejected");
    assert_eq!(err, AuthError::InvalidToken);
    // Stateless: no in-process identity survives (or leaks) after logout.
    assert!(tokio_test_block_on(guard.user())
        .expect("user resolves")
        .is_none());
}

/// Every login mints a distinct id (session-fixation defense).
#[test]
fn login_rotates_session_id_each_time() {
    let (guard, _registry) = guard_with_registry();
    let first = tokio_test_block_on(guard.login(&creds())).expect("first login");
    let second = tokio_test_block_on(guard.login(&creds())).expect("second login");
    assert_ne!(first.access_token, second.access_token);
}

/// Refresh retains the payload but rotates the session id.
#[test]
fn refresh_rotates_session_id() {
    let (guard, _registry) = guard_with_registry();
    let token = tokio_test_block_on(guard.login(&creds())).expect("login succeeds");
    let rotated = tokio_test_block_on(guard.refresh(&token.access_token)).expect("refresh");
    assert_ne!(rotated.access_token, token.access_token);

    let old: Id = token.access_token.parse().expect("session id");
    assert!(tokio_test_block_on(guard.session_store().load(&old))
        .expect("load")
        .is_none());
    let principal = tokio_test_block_on(guard.parse(&rotated.access_token)).expect("parse");
    assert_eq!(principal.id, "user-1");
}

/// A forged/unknown session id is rejected with `InvalidToken`.
#[test]
fn forged_session_id_is_rejected() {
    let (guard, _registry) = guard_with_registry();
    let forged = Id::default().to_string();
    let err = tokio_test_block_on(guard.parse(&forged)).expect_err("forged id rejected");
    assert_eq!(err, AuthError::InvalidToken);
    let err = tokio_test_block_on(guard.logout("not-a-session-id")).expect_err("garbage rejected");
    assert_eq!(err, AuthError::InvalidToken);
}

/// Two sessions resolved through the same shared guard keep distinct
/// identities: concurrent requests cannot contaminate each other.
#[test]
fn concurrent_sessions_do_not_share_identity() {
    let registry = Arc::new(MemoryUserRegistry::default());
    registry.seed(AuthUserRecord {
        id: "user-1".into(),
        email: "ada@example.com".into(),
        password_hash: "phc$hash".into(),
        email_verified_at: None,
    });
    registry.seed(AuthUserRecord {
        id: "user-2".into(),
        email: "grace@example.com".into(),
        password_hash: "phc$hash".into(),
        email_verified_at: None,
    });
    // One guard instance, shared exactly as `AuthManager` shares `Arc<dyn Guard>`.
    let guard = Arc::new(
        SessionGuard::new(SessionPolicy::default())
            .with_verifier(Arc::new(AlwaysVerify))
            .with_lookup(registry)
            .with_allow_login_using_id(true),
    );

    let ada = tokio_test_block_on(guard.login(&Credentials {
        email: "ada@example.com".into(),
        password: "s3cr3tPass".into(),
    }))
    .expect("ada login");
    let grace = tokio_test_block_on(guard.login(&Credentials {
        email: "grace@example.com".into(),
        password: "s3cr3tPass".into(),
    }))
    .expect("grace login");

    // Interleave both parses on the SAME guard; each token resolves to its
    // own principal regardless of ordering.
    let (a, g) = tokio_test_block_on(async {
        tokio::join!(
            guard.parse(&ada.access_token),
            guard.parse(&grace.access_token),
        )
    });
    assert_eq!(a.expect("ada parses").id, "user-1");
    assert_eq!(g.expect("grace parses").id, "user-2");
    // And the guard exposes no "last" identity at all.
    assert!(tokio_test_block_on(guard.user())
        .expect("user resolves")
        .is_none());
}

/// `login_using_id` is fail-closed unless explicitly enabled.
#[test]
fn login_using_id_disabled_by_default() {
    let guard = SessionGuard::new(SessionPolicy::default());
    let err = tokio_test_block_on(guard.login_using_id("user-1")).expect_err("disabled");
    assert!(matches!(err, AuthError::Disabled(_)));
}
