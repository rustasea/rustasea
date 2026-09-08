//! Crate-local routes module — re-exposes the workspace-root `routes/`
//! (README layout) to the runnable `src/main.rs`.
//!
//! Route definitions live at the repository root in `routes/web.rs`; this
//! module exists so the root binary can reach them as `crate::routes`.

pub mod web;
