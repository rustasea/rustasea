//! Redirects already-authenticated users away from guest-only screens.

/// Destination for an authenticated user hitting a guest route.
pub const HOME: &str = "/dashboard";

/// Whether the current session is authenticated.
pub fn is_authenticated(user_id: Option<&str>) -> bool {
    user_id.is_some()
}
