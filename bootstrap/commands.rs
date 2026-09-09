//! CLI command registry — `cargo artisan` console commands.
//!
//! Canonical home per README "Proposed Directory Structure"
//! (`bootstrap/commands.rs`, M5 deliverable). Console commands generated into
//! `app/console/commands/*` are registered here once the `rustasea-cli`
//! registry types ship (M5).

/// Return the CLI commands registered for the application.
///
/// Placeholder until the M5 `CommandRegistry` integration lands.
pub fn commands() -> Vec<&'static str> {
    vec![]
}
