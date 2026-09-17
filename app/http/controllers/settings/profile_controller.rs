//! Profile settings handlers.

use axum::response::Response;

use crate::app::http::controllers::controller::Controller;

/// Handles the profile settings screen and updates.
pub struct ProfileController;

impl Controller for ProfileController {}

impl ProfileController {
    /// GET /settings/profile — show the profile form.
    pub async fn edit() -> Response {
        todo!("render the profile settings screen")
    }

    /// PATCH /settings/profile — persist profile changes.
    pub async fn update() -> Response {
        todo!("validate ProfileUpdateRequest and persist the user")
    }
}
