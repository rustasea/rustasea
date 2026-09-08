//! Provider registry — service providers registered by the application.
//!
//! Canonical home per README "Proposed Directory Structure"
//! (`bootstrap/providers.rs`). `bootstrap/app.rs::configure` consumes this
//! registry; generated `app/providers/*` from `cargo artisan
//! make:provider` are appended here.

/// Return the providers registered for the application boot DAG.
///
/// Placeholder until the CLI generator + provider registry types ship (M5).
pub fn providers() -> Vec<&'static str> {
    vec![]
}
