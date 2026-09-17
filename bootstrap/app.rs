//! Application bootstrap - `Application::configure` for ExampleApp.
//!
//! Registers the generated service providers, registers the console command
//! surface and the application migrations, and runs the register -> boot DAG
//! before the HTTP kernel starts serving.

use rustasea::foundation::BootError;
use rustasea::Application;

use crate::bootstrap::{commands, providers};

/// Build and boot the application container.
///
/// Returns [`BootError`] when the provider graph contains a cycle or an
/// unresolved dependency, so a misconfigured boot never starts the server.
/// Registering the command surface also registers the framework's queue
/// migrations and the application's own migrations, so `cargo artisan migrate`
/// runs the real schema in this binary.
pub fn configure() -> Result<Application, BootError> {
    let mut app = Application::configure(|_| {});
    for provider in providers::providers() {
        app.provider(provider);
    }
    commands::register_default();
    app.boot()?;
    Ok(app)
}
