//! Route tables mirroring Laravel's `routes/{web,auth,settings,console}.php`.
//!
//! Splitting the single legacy `routes/web.rs` into four concern-scoped tables
//! is ADR-0002 decision 9.

use super::TemplateFile;

/// Route templates (shared by every variant).
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("routes/mod.rs", ROUTES_MOD),
        ("routes/web.rs", WEB),
        ("routes/auth.rs", AUTH),
        ("routes/settings.rs", SETTINGS),
        ("routes/console.rs", CONSOLE),
    ]
}

const ROUTES_MOD: &str = r##"//! Route tables — `web`, `auth`, `settings`, and `console`.

pub mod auth;
pub mod console;
pub mod settings;
pub mod web;

use std::sync::Arc;

use rustasea::http::AppState;

/// Build the application router from every generated route table.
///
/// The shared [`AppState`] is created once during boot in `main` and threaded
/// in here, so every route table serves the same state instead of each
/// constructing a disconnected one.
pub fn router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        .merge(web::routes())
        .merge(auth::routes())
        .merge(settings::routes())
        .with_state(state)
}
"##;

const WEB: &str = r##"//! Web routes — the public landing page and the authenticated dashboard.

use std::sync::Arc;

use axum::routing::get;

use crate::app::http::controllers::dashboard_controller;
use rustasea::http::AppState;

/// Web route table.
pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/", get(welcome))
        .route("/dashboard", get(dashboard_controller::index))
}

/// GET / — public landing page.
async fn welcome() -> axum::response::Response {
    todo!("render the welcome screen for this variant")
}
"##;

const AUTH: &str = r##"//! Auth routes — login, registration, logout, and password reset.

use std::sync::Arc;

use axum::routing::{get, post};

use crate::app::http::controllers::auth_controller;
use rustasea::http::AppState;

/// Auth route table.
pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/login", get(auth_controller::show_login).post(auth_controller::login))
        .route("/logout", post(auth_controller::logout))
        .route(
            "/register",
            get(auth_controller::show_register).post(auth_controller::register),
        )
}
"##;

const SETTINGS: &str = r##"//! Settings routes — profile and password management.

use std::sync::Arc;

use axum::routing::{get, patch, put};

use crate::app::http::controllers::settings::{password_controller, profile_controller};
use rustasea::http::AppState;

/// Settings route table.
pub fn routes() -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route(
            "/settings/profile",
            get(profile_controller::edit).patch(profile_controller::update),
        )
        .route(
            "/settings/password",
            get(password_controller::edit).put(password_controller::update),
        )
}
"##;

const CONSOLE: &str = r##"//! Console route registration — the home of the previously-empty command registry.

use crate::bootstrap::commands;

/// Return the console commands registered by the application.
pub fn register() -> Vec<&'static str> {
    commands::commands()
}
"##;
