//! Dashboard handler for server-rendered variants (blade / livewire).
//!
//! The handler renders `resources/views/dashboard.html` through the registered
//! askama [`ViewEngine`](rustasea::view::ViewEngine).

use axum::response::Response;

use crate::app::http::controllers::controller::Controller;

/// Handles the authenticated dashboard screen.
pub struct DashboardController;

impl Controller for DashboardController {}

impl DashboardController {
    /// GET /dashboard — render the authenticated dashboard view.
    pub async fn index() -> Response {
        todo!("render resources/views/dashboard.html with shared props")
    }
}
