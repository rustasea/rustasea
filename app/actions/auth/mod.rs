//! Fortify-analogue authentication actions.
//!
//! Each action is a small, testable function that the auth controllers compose;
//! the same actions are generated for every variant.

pub mod attempt_to_authenticate;
pub mod create_new_user;
pub mod ensure_login_is_not_throttled;
pub mod prepare_authenticated_session;
pub mod redirect_if_authenticated;
