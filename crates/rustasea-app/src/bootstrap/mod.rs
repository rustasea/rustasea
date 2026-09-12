//! Crate-local bootstrap module for the runnable `rustasea-app` binary.
//!
//! Mirrors the README layout: `bootstrap/app.rs` configures the application
//! (`Application::configure`), `bootstrap/providers.rs` owns the provider
//! registry, and `bootstrap/commands.rs` registers the console command
//! surface. The canonical root `bootstrap/` wiring stays at the workspace
//! root as the scaffold contract; this module hosts the crate-local copy that
//! `main.rs` reaches as `crate::bootstrap`.

pub mod app;
pub mod commands;
pub mod providers;
