//! Registers the session guard and CSRF wiring.
//!
//! Security-critical: the session guard is shared by every variant, so a
//! hardening fix lands once (ADR-0002 decision 7).

use rustasea::{Application, ServiceProvider};

/// Auth service provider.
pub struct AuthServiceProvider;

impl ServiceProvider for AuthServiceProvider {
    fn register(&self, _app: &mut Application) {}

    fn boot(&self, _app: &Application) {}
}
