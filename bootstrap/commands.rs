//! CLI command registry — `cargo artisan` console commands.
//!
//! Canonical home per README "Proposed Directory Structure"
//! (`bootstrap/commands.rs`). Commands generated into
//! `app/console/commands/*` are appended here; the framework's built-ins are
//! registered by [`register_default`].

/// Register the framework's built-in console commands into the process-wide
/// registry.
///
/// Idempotent per command name: later registrations replace earlier ones.
pub fn register_default() {
    rustasea::cli::load_default_commands();
}

/// Return the command signatures currently registered for the application.
///
/// Reads the process-wide registry after [`register_default`] so the manifest
/// never drifts from the actual built-in surface.
pub fn commands() -> Vec<String> {
    rustasea::cli::Artisan::list(false)
        .into_iter()
        .map(|meta| meta.name)
        .collect()
}
