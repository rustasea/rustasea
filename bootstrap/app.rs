//! Application bootstrap — `Application::configure` for the runnable app.
//!
//! Mirrors Laravel's `bootstrap/app.php` (README "Proposed Directory
//! Structure"): configures the foundation container, registers the providers
//! from [`providers`], registers the console command surface, and boots the
//! register→boot DAG before the HTTP kernel takes over.

use rustasea::foundation::BootError;
use rustasea::Application;

use crate::bootstrap::{commands, providers};

/// Build and boot the application.
///
/// Returns [`BootError`] when the provider graph contains a cycle or an
/// unresolved dependency, so a misconfigured boot never starts the server.
/// Registering the default command surface also registers the framework's
/// queue migrations.
pub fn configure() -> Result<Application, BootError> {
    let mut app = Application::configure(|_| {});
    for provider in providers::providers() {
        app.provider(provider);
    }
    commands::register_default();
    app.boot()?;
    Ok(app)
}
