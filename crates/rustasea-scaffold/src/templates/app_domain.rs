//! Domain layer: models and service providers.
//!
//! Emits `app/mod.rs`, the Eloquent-style `User` model, and the two providers
//! that `bootstrap/app.rs` registers into the boot DAG.

use super::TemplateFile;

/// Domain templates (shared by every variant).
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("app/mod.rs", APP_MOD),
        ("app/models/mod.rs", MODELS_MOD),
        ("app/models/user.rs", USER),
        ("app/providers/mod.rs", PROVIDERS_MOD),
        (
            "app/providers/app_service_provider.rs",
            APP_SERVICE_PROVIDER,
        ),
        (
            "app/providers/auth_service_provider.rs",
            AUTH_SERVICE_PROVIDER,
        ),
    ]
}

const APP_MOD: &str = r##"//! Application domain layer.
//!
//! Mirrors Laravel's `app/` directory: auth actions, shared concerns, HTTP
//! handlers, models, and service providers.

pub mod actions;
pub mod concerns;
pub mod http;
pub mod models;
pub mod providers;
"##;

const MODELS_MOD: &str = r##"//! Eloquent-style application models.

pub mod user;

pub use user::User;
"##;

const USER: &str = r##"//! `users` model — the shared auth core's user entity.

use chrono::{DateTime, Utc};
use rustasea::orm::{Model, Timestamps};
use uuid::Uuid;

/// Application user account.
///
/// The same model is generated for every variant; only the presentation layer
/// differs (ADR-0002 decision 1).
#[derive(Debug, Clone, Model)]
#[model(table = "users")]
pub struct User {
    /// Primary key.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Unique, verified email address.
    pub email: String,
    /// Argon2id password hash — never the plaintext.
    pub password: String,
    /// Set once the email address is verified.
    pub email_verified_at: Option<DateTime<Utc>>,
    /// Soft-delete marker.
    pub deleted_at: Option<DateTime<Utc>>,
    /// Created/updated timestamps.
    #[model(timestamps)]
    pub timestamps: Timestamps,
}
"##;

const PROVIDERS_MOD: &str = r##"//! Service providers registered into the application boot DAG.

pub mod app_service_provider;
pub mod auth_service_provider;

pub use app_service_provider::AppServiceProvider;
pub use auth_service_provider::AuthServiceProvider;
"##;

const APP_SERVICE_PROVIDER: &str = r##"//! Registers application-wide container bindings.

use rustasea::{Application, ServiceProvider};

/// Application service provider.
pub struct AppServiceProvider;

impl ServiceProvider for AppServiceProvider {
    fn register(&self, _app: &mut Application) {}

    fn boot(&self, _app: &Application) {}
}
"##;

const AUTH_SERVICE_PROVIDER: &str = r##"//! Registers the session guard and CSRF wiring.
//!
//! Security-critical: the session guard is shared by every variant, so a
//! hardening fix lands once (ADR-0002 decision 7).

use rustasea::{Application, ServiceProvider};

/// Auth service provider.
pub struct AuthServiceProvider;

impl ServiceProvider for AuthServiceProvider {
    fn register(&self, _app: &mut Application) {}

    fn boot(&self, _app: &Application) {}
}
"##;
