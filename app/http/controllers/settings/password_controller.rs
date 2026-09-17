//! Password settings handlers.

use axum::response::Response;

use crate::app::http::controllers::controller::Controller;

/// Handles the password settings screen and rotation.
pub struct PasswordController;

impl Controller for PasswordController {}

impl PasswordController {
    /// GET /settings/password — show the password form.
    pub async fn edit() -> Response {
        todo!("render the password settings screen")
    }

    /// PUT /settings/password — rotate the password.
    pub async fn update() -> Response {
        todo!("validate PasswordUpdateRequest, re-hash, and persist")
    }
}
