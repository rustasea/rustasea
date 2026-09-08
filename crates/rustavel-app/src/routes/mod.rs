//! Crate-local routes module for the runnable `rustavel-app` binary.
//!
//! Route definitions live at the workspace root in `routes/web.rs` (README
//! layout); this module hosts the crate-local copy so `main.rs` reaches it
//! as `crate::routes::web`.

pub mod web;
