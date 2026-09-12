//! Test suites mirroring Laravel's `tests/{Feature,Unit}` split.
//!
//! Grouped as `tests/feature/` and `tests/unit/` modules; each suite references
//! the shared core so the security-critical auth rules are covered for every
//! variant.

use super::TemplateFile;

/// Test-suite templates (shared by every variant).
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("tests/feature/mod.rs", FEATURE_MOD),
        ("tests/feature/auth_test.rs", AUTH_TEST),
        ("tests/unit/mod.rs", UNIT_MOD),
        ("tests/unit/actions_test.rs", ACTIONS_TEST),
    ]
}

const FEATURE_MOD: &str = r##"//! Feature tests — HTTP-level behaviour of the generated routes.

pub mod auth_test;
"##;

const AUTH_TEST: &str = r##"//! Feature tests for registration and authentication rules.

use @@app_snake@@::app::concerns::password_validation_rules;

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
"##;

const UNIT_MOD: &str = r##"//! Unit tests — domain actions and concerns in isolation.

pub mod actions_test;
"##;

const ACTIONS_TEST: &str = r##"//! Unit tests for the generated auth actions.

use @@app_snake@@::app::actions::auth::redirect_if_authenticated;

/// Guest-only redirects detect an authenticated session.
#[test]
fn authenticated_users_are_detected() {
    assert!(redirect_if_authenticated::is_authenticated(Some("user-id")));
    assert!(!redirect_if_authenticated::is_authenticated(None));
}
"##;
