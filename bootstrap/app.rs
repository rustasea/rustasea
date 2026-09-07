//! Application bootstrap — configure providers and bindings.

use rustavel::Application;

struct AppServiceProvider;

impl rustavel::ServiceProvider for AppServiceProvider {
    fn register(&self, _app: &mut Application) {}

    fn boot(&self, _app: &Application) {}
}

/// Build the application.
pub fn build() -> Application {
    let mut app = Application::configure(|_| {});
    app.provider(AppServiceProvider);
    app.boot();
    app
}
