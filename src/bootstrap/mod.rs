//! Crate-local bootstrap module — re-exposes the workspace-root
//! `bootstrap/` wiring (README layout) to the runnable `src/main.rs`.
//!
//! The canonical bootstrap sources live at the repository root
//! (`bootstrap/app.rs`, `bootstrap/providers.rs`, `bootstrap/commands.rs`);
//! this module exists so the root binary can reach them as `crate::bootstrap`.

pub mod app;
