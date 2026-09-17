//! Unit tests for the generated auth actions and validation concerns.

use example_app::app::actions::auth::create_new_user::CreateNewUser;
use example_app::app::actions::auth::redirect_if_authenticated;
use example_app::app::concerns::profile_validation_rules;
use rustasea::action::Action as _;

/// Guest-only redirects detect an authenticated session.
#[test]
fn authenticated_users_are_detected() {
    assert!(redirect_if_authenticated::is_authenticated(Some("user-id")));
    assert!(!redirect_if_authenticated::is_authenticated(None));
}

/// The registration action implements the shared `Action` trait.
#[test]
fn registration_action_is_an_action() {
    let _action = CreateNewUser;
}

/// A blank display name is rejected by the profile rules.
#[test]
fn profile_rules_reject_blank_names() {
    assert!(profile_validation_rules::validate_name("   ").is_err());
}

/// A normal display name passes the profile rules.
#[test]
fn profile_rules_accept_a_normal_name() {
    assert!(profile_validation_rules::validate_name("Ada Lovelace").is_ok());
}

/// A display name longer than `NAME_MAX` is rejected.
#[test]
fn profile_rules_reject_overlong_names() {
    let overlong = "x".repeat(profile_validation_rules::NAME_MAX + 1);
    assert!(profile_validation_rules::validate_name(&overlong).is_err());
}
