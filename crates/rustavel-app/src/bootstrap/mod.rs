//! Crate-local bootstrap module for the runnable `rustavel-app` binary.
//!
//! Mirrors the README layout: `bootstrap/app.rs` configures the application
//! (`Application::configure`), and the canonical root `bootstrap/` wiring
//! (`app.rs`, `providers.rs`, `commands.rs`) stays at the workspace root as
//! the scaffold contract. This module re-exposes the crate-local `app`
//! module to `main.rs` as `crate::bootstrap`.

pub mod app;
