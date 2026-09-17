//! Password validation rules (Fortify `PasswordValidationRules` analogue).
//!
//! Delegates to the shared [`rustasea::validation::PasswordPolicy`] so the
//! generated app and the framework enforce one policy: strict in production
//! (min 12 + mixed case + letters + numbers + symbols), relaxed elsewhere
//! (min 12 only).

use rustasea::validation::PasswordPolicy;

/// Resolve the active environment (`APP_ENV`, then `config/app.toml`).
fn resolve_environment() -> String {
    if let Some(env) = std::env::var("APP_ENV")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return env;
    }
    if let Ok(loader) = rustasea::config::ConfigLoader::load_from(&["config/app"]) {
        if let Ok(env) = loader.get_key::<String>("app_env") {
            let env = env.trim();
            if !env.is_empty() {
                return env.to_string();
            }
        }
    }
    rustasea::orm::PRODUCTION.to_string()
}

/// Select the policy for the current environment.
fn policy() -> PasswordPolicy {
    if rustasea::orm::is_production(&resolve_environment()) {
        PasswordPolicy::production()
    } else {
        PasswordPolicy::default()
    }
}

/// Validate a candidate password, returning a human-readable reason on failure.
pub fn validate(password: &str) -> Result<(), String> {
    match policy().violations(password).into_iter().next() {
        Some(violation) => Err(violation.message),
        None => Ok(()),
    }
}
