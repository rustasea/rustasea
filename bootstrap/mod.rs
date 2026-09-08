//! Application bootstrap — wiring for providers, routing, and CLI commands.
//!
//! Mirrors the canonical `bootstrap/` layout in `README.md`:
//! `app.rs` (Application::configure), `providers.rs` (provider registry),
//! and `commands.rs` (CLI command registry).

pub mod app;
pub mod commands;
pub mod providers;
