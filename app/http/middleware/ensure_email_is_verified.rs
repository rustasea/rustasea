//! Rejects authenticated-but-unverified users from protected routes.

use axum::response::Response;

/// Whether the user may pass the `verified` middleware.
pub fn passes(email_verified_at: Option<&str>) -> bool {
    email_verified_at.is_some()
}

/// Middleware entry point (wired by the router once middleware binding lands).
pub async fn handle() -> Response {
    todo!("redirect unverified users to /verify-email")
}
