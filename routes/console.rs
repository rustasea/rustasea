//! Console route registration — the home of the previously-empty command registry.

use crate::bootstrap::commands;

/// Return the console commands registered by the application.
pub fn register() -> Vec<&'static str> {
    commands::commands()
}
