//! Registers application-wide container bindings.
//!
//! Installs the environment-derived policy defaults, mirroring the kit's
//! `AppServiceProvider::configureDefaults()`.
//!
//! # Immutable dates — not applicable
//!
//! `chrono` has no global mutable date default (`DateTime` is an immutable
//! value type and `Utc::now()` returns a fresh value), so the kit's
//! `Date::use(CarbonImmutable::class)` has no analogue here.

use rustasea::validation::PasswordPolicy;
use rustasea::{Application, ServiceProvider};

/// Container key holding the resolved environment name (`String`).
pub const ENVIRONMENT_KEY: &str = "app.environment";

/// Container key holding the installed [`PasswordPolicy`].
pub const PASSWORD_POLICY_KEY: &str = "app.password_policy";

/// Container key holding whether destructive commands are prohibited (`bool`).
pub const DESTRUCTIVE_COMMANDS_PROHIBITED_KEY: &str = "app.prohibits_destructive_commands";

/// Resolve the active environment (`APP_ENV`, then `config/app.toml`).
pub fn resolve_environment() -> String {
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

/// Select the password policy for `environment`.
///
/// Production is strict; every other environment is relaxed.
pub fn password_policy_for(environment: &str) -> PasswordPolicy {
    if rustasea::orm::is_production(environment) {
        PasswordPolicy::production()
    } else {
        PasswordPolicy::default()
    }
}

/// Application service provider.
pub struct AppServiceProvider;

impl ServiceProvider for AppServiceProvider {
    /// Install the environment-derived policy defaults.
    fn register(&self, app: &mut Application) {
        let environment = resolve_environment();
        let policy = password_policy_for(&environment);
        let prohibited = rustasea::orm::is_production(&environment);
        app.container.instance(ENVIRONMENT_KEY, environment);
        app.container.instance(PASSWORD_POLICY_KEY, policy);
        app.container
            .instance(DESTRUCTIVE_COMMANDS_PROHIBITED_KEY, prohibited);
    }

    fn boot(&self, _app: &Application) {}
}
