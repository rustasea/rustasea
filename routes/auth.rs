//! Auth routes — login, registration, logout, and password confirmation.
//!
//! Mirrors the kit's `routes/auth.php`: every route is named, and the
//! confirm-password screen (`password.confirm`) re-checks the current password
//! before a sensitive action proceeds.

use rustasea::router::Router;

use crate::app::http::controllers::auth_controller::AuthController;

/// Register the auth route table.
pub fn register(table: &mut Router) {
    table
        .get_action("/login", AuthController::show_login)
        .named("login");
    table
        .post_action("/login", AuthController::login)
        .named("login");
    table
        .post_action("/logout", AuthController::logout)
        .named("logout");
    table
        .get_action("/register", AuthController::show_register)
        .named("register");
    table
        .post_action("/register", AuthController::register)
        .named("register");
    table
        .get_action("/confirm-password", AuthController::show_confirm_password)
        .named("password.confirm");
    table
        .post_action("/confirm-password", AuthController::confirm_password)
        .named("password.confirm");
}
