//! Settings routes — profile, password, and security management.
//!
//! Mirrors the kit's `routes/settings.php`: `/settings` redirects to the profile
//! screen, the profile/password screens sit behind `auth`, and the security
//! screen additionally requires `verified` and a recent `password.confirm`.

use rustasea::router::Router;

use crate::app::http::controllers::settings::password_controller::PasswordController;
use crate::app::http::controllers::settings::profile_controller::ProfileController;
use crate::app::http::controllers::settings::security_controller::SecurityController;

/// Register the settings route table.
pub fn register(table: &mut Router) {
    table.group(|group| {
        group.middleware("auth");

        // `/settings` is a convenience redirect (302) to the profile screen.
        group
            .redirect("/settings", "/settings/profile")
            .named("settings");

        group
            .get_action("/settings/profile", ProfileController::edit)
            .named("profile.edit");
        group
            .patch_action("/settings/profile", ProfileController::update)
            .named("profile.edit");
        group
            .get_action("/settings/password", PasswordController::edit)
            .named("password.edit");
        group
            .put_action("/settings/password", PasswordController::update)
            .named("password.edit");

        // The security screen is the most sensitive: it requires a verified
        // account *and* a recently confirmed password (kit parity).
        group.group(|security| {
            security.middleware("verified").middleware("password.confirm");
            security
                .get_action("/settings/security", SecurityController::edit)
                .named("security.edit");
        });
    });
}
