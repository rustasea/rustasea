//! Web routes — the public landing page and the authenticated dashboard.
//!
//! `home` is public; `dashboard` sits behind the `auth` + `verified` guards,
//! mirroring the kit's `Route::get('dashboard', ...)->middleware(['auth',
//! 'verified'])->name('dashboard')`.

use rustasea::router::Router;

use crate::app::http::controllers::dashboard_controller::DashboardController;

/// Register the web route table.
pub fn register(table: &mut Router) {
    table.get_action("/", welcome).named("home");

    table.group(|group| {
        group.middleware("auth").middleware("verified");
        group
            .get_action("/dashboard", DashboardController::index)
            .named("dashboard");
    });
}

/// GET / — public landing page.
async fn welcome() -> axum::response::Response {
    todo!("render the welcome screen for this variant")
}
