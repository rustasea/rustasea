//! Application bootstrap — `Application::configure` for the runnable app.
//!
//! Mirrors Laravel's `bootstrap/app.php` (README "Proposed Directory
//! Structure"): configures the foundation container, registers service
//! providers, and boots the register→boot DAG before the HTTP kernel takes
//! over. `src/main.rs` calls [`configure`]; providers/routes/schedule/events
//! registries land in `bootstrap/providers.rs` + `bootstrap/commands.rs`.

use rustavel::Application;

/// Placeholder provider scoped to the app scaffold.
///
/// Filled in as framework services (config, ORM, auth, queue, …) ship; today
/// it only demonstrates provider participation in the boot DAG.
struct AppServiceProvider;

impl rustavel::ServiceProvider for AppServiceProvider {
    fn register(&self, _app: &mut Application) {}

    fn boot(&self, _app: &Application) {}
}

/// Build and boot the application.
pub fn configure() -> Application {
    let mut app = Application::configure(|_| {});
    app.provider(AppServiceProvider);
    app.boot();
    app
}
