//! Application domain layer.
//!
//! Mirrors Laravel's `app/` directory: auth actions, shared concerns, HTTP
//! handlers, models, and service providers.

pub mod actions;
pub mod concerns;
pub mod http;
pub mod models;
pub mod providers;
