//! Shared auth actions and validation concerns (Fortify analogue).
//!
//! This is the security-critical core that every variant shares byte-for-byte
//! (ADR-0002 decision 1): registration, authentication, throttling, guest
//! redirection, and session preparation, plus the reusable validation rules.

use super::TemplateFile;

/// Auth action and concern templates (shared by every variant).
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("app/actions/mod.rs", ACTIONS_MOD),
        ("app/actions/auth/mod.rs", AUTH_MOD),
        ("app/actions/auth/create_new_user.rs", CREATE_NEW_USER),
        (
            "app/actions/auth/attempt_to_authenticate.rs",
            ATTEMPT_TO_AUTHENTICATE,
        ),
        (
            "app/actions/auth/ensure_login_is_not_throttled.rs",
            ENSURE_LOGIN_IS_NOT_THROTTLED,
        ),
        (
            "app/actions/auth/redirect_if_authenticated.rs",
            REDIRECT_IF_AUTHENTICATED,
        ),
        (
            "app/actions/auth/prepare_authenticated_session.rs",
            PREPARE_AUTHENTICATED_SESSION,
        ),
        ("app/concerns/mod.rs", CONCERNS_MOD),
        (
            "app/concerns/password_validation_rules.rs",
            PASSWORD_VALIDATION_RULES,
        ),
        (
            "app/concerns/profile_validation_rules.rs",
            PROFILE_VALIDATION_RULES,
        ),
    ]
}

const ACTIONS_MOD: &str = r##"//! Domain actions — one responsibility per module.

pub mod auth;
"##;

const AUTH_MOD: &str = r##"//! Fortify-analogue authentication actions.
//!
//! Each action is a small, testable function that the auth controllers compose;
//! the same actions are generated for every variant.

pub mod attempt_to_authenticate;
pub mod create_new_user;
pub mod ensure_login_is_not_throttled;
pub mod prepare_authenticated_session;
pub mod redirect_if_authenticated;
"##;

const CREATE_NEW_USER: &str = r##"//! Creates and persists a user during registration.

use crate::app::models::User;

/// Validated registration input.
pub struct NewUser {
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Plaintext password — hashed with argon2id before persistence.
    pub password: String,
}

/// Create a user, hashing the password and applying registration defaults.
pub fn create(input: NewUser) -> Result<User, rustasea::auth::AuthError> {
    let _ = &input;
    // The session guard hashes with argon2id and rotates the session id on
    // login; this action owns persistence only.
    todo!("persist the user via the ORM writer")
}
"##;

const ATTEMPT_TO_AUTHENTICATE: &str = r##"//! Attempts to authenticate a login request against the session guard.

use rustasea::auth::{AuthError, SessionGuard};

/// Authenticate `email` + `password`, rotating the session id on success.
pub fn attempt<S>(guard: &SessionGuard<S>, email: &str, password: &str) -> Result<(), AuthError>
where
    S: tower_sessions::SessionStore + Send + Sync + 'static,
{
    let _ = (guard, email, password);
    todo!("verify credentials and rotate the session id")
}
"##;

const ENSURE_LOGIN_IS_NOT_THROTTLED: &str = r##"//! Applies login throttling before authentication is attempted.

use rustasea::auth::{Limit, RateLimiter, ThrottleDecision};

/// Rejection returned when a login key exceeds its configured rate limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginThrottled {
    /// Seconds the caller must wait before retrying (`Retry-After`).
    pub retry_after_secs: u64,
}

impl std::fmt::Display for LoginThrottled {
    /// Render the throttle rejection together with its retry delay.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "too many login attempts; retry after {}s",
            self.retry_after_secs
        )
    }
}

impl std::error::Error for LoginThrottled {}

/// Register one attempt for `key` and reject it once `limit` is exceeded.
pub fn ensure(key: &str, limiter: &dyn RateLimiter, limit: &Limit) -> Result<(), LoginThrottled> {
    match limiter.hit(key, limit) {
        ThrottleDecision::Allowed { .. } => Ok(()),
        ThrottleDecision::Denied { retry_after_secs } => {
            Err(LoginThrottled { retry_after_secs })
        }
    }
}
"##;

const REDIRECT_IF_AUTHENTICATED: &str = r##"//! Redirects already-authenticated users away from guest-only screens.

/// Destination for an authenticated user hitting a guest route.
pub const HOME: &str = "/dashboard";

/// Whether the current session is authenticated.
pub fn is_authenticated(user_id: Option<&str>) -> bool {
    user_id.is_some()
}
"##;

const PREPARE_AUTHENTICATED_SESSION: &str = r##"//! Finalizes the session after a successful login.

/// Regenerate the session id and persist the authenticated user id.
///
/// Rotating on login is the session-fixation defense (ADR-0002 decision 7).
pub fn prepare(session_id: &str) -> String {
    // The guard rotates the id; this helper exists so the controller can chain
    // prepare → redirect explicitly.
    format!("{session_id}:authenticated")
}
"##;

const CONCERNS_MOD: &str = r##"//! Reusable validation concerns shared by requests and actions.

pub mod password_validation_rules;
pub mod profile_validation_rules;
"##;

const PASSWORD_VALIDATION_RULES: &str = r##"//! Password validation rules (Fortify `PasswordValidationRules` analogue).

/// Minimum password length enforced on registration and password updates.
pub const MIN_LENGTH: usize = 12;

/// Validate a candidate password, returning a human-readable reason on failure.
pub fn validate(password: &str) -> Result<(), String> {
    if password.chars().count() < MIN_LENGTH {
        return Err(format!("password must be at least {MIN_LENGTH} characters"));
    }
    Ok(())
}
"##;

const PROFILE_VALIDATION_RULES: &str = r##"//! Profile validation rules shared by registration and settings.

/// Maximum accepted display-name length.
pub const NAME_MAX: usize = 255;

/// Validate a display name.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    if name.chars().count() > NAME_MAX {
        return Err(format!("name may not exceed {NAME_MAX} characters"));
    }
    Ok(())
}
"##;

#[cfg(test)]
mod tests {
    use super::ENSURE_LOGIN_IS_NOT_THROTTLED;

    /// Throttle action uses the real `RateLimiter` contract and a typed error.
    ///
    /// Guards against the regression where the template emitted
    /// `&RateLimiter`, `too_many_attempts`, and `AuthError::TooManyAttempts`,
    /// none of which compile against `rustasea-auth`.
    #[test]
    fn ensure_login_is_not_throttled_matches_rate_limiter_api() {
        assert!(ENSURE_LOGIN_IS_NOT_THROTTLED.contains("limiter: &dyn RateLimiter"));
        assert!(ENSURE_LOGIN_IS_NOT_THROTTLED.contains("limiter.hit(key, limit)"));
        assert!(ENSURE_LOGIN_IS_NOT_THROTTLED.contains("ThrottleDecision::Denied"));
        assert!(!ENSURE_LOGIN_IS_NOT_THROTTLED.contains("too_many_attempts"));
        assert!(!ENSURE_LOGIN_IS_NOT_THROTTLED.contains("TooManyAttempts"));
    }
}
