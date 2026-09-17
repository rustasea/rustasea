//! Login, registration, logout, and password-reset handlers.
//!
//! The auth flow is identical across variants; only the response the handler
//! builds differs (askama view vs Inertia page).

use axum::response::Response;

use crate::app::http::controllers::controller::Controller;

/// Handles authentication screens and submissions.
pub struct AuthController;

impl Controller for AuthController {}

impl AuthController {
    /// GET /login — show the login screen.
    pub async fn show_login() -> Response {
        todo!("render the login screen for this variant")
    }

    /// POST /login — authenticate and start the session.
    pub async fn login() -> Response {
        todo!("authenticate, rotate the session id, and redirect")
    }

    /// POST /logout — destroy the session and redirect home.
    pub async fn logout() -> Response {
        todo!("destroy the session and redirect to /")
    }

    /// GET /register — show the registration screen.
    pub async fn show_register() -> Response {
        todo!("render the registration screen for this variant")
    }

    /// POST /register — create the account and authenticate.
    pub async fn register() -> Response {
        todo!("create the user, log in, and redirect")
    }

    /// GET /confirm-password — show the password-confirmation screen.
    pub async fn show_confirm_password() -> Response {
        todo!("render the confirm-password screen for this variant")
    }

    /// POST /confirm-password — re-confirm the current password.
    pub async fn confirm_password() -> Response {
        todo!("verify the current password and mark it confirmed for the session")
    }
}
