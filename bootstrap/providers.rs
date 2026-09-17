//! Provider registry - service providers registered by the application.
//!
//! This registry is populated by the starter kit (previously empty) and is the
//! registration site for providers generated with `cargo rustasea make:provider`.

use rustasea::ServiceProvider;

use crate::app::providers::{AppServiceProvider, AuthServiceProvider};

/// Providers wired into the boot DAG, in registration order.
///
/// Order is the tie-breaker for providers without `dependencies()`; the
/// foundation `Application::boot` topologically sorts them regardless.
pub fn providers() -> Vec<Box<dyn ServiceProvider>> {
    vec![Box::new(AppServiceProvider), Box::new(AuthServiceProvider)]
}
