//! Security settings handlers.
//!
//! The security screen is where password and two-factor controls live in the
//! kit; only the screen itself is scaffolded here. The 2FA and passkey flows
//! are intentionally not generated (no backing infrastructure yet).

use axum::response::Response;

use crate::app::http::controllers::controller::Controller;

/// Handles the security settings screen.
pub struct SecurityController;

impl Controller for SecurityController {}

impl SecurityController {
    /// GET /settings/security — show the security screen.
    pub async fn edit() -> Response {
        todo!("render the security settings screen")
    }
}
