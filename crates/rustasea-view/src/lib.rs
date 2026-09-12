//! RustaSea View — server-side template rendering for the framework.
//!
//! The crate exposes a small, engine-agnostic surface:
//!
//! - [`ViewEngine`] — an object-safe rendering contract usable as
//!   `Arc<dyn ViewEngine>`, plus a typed `render` convenience on sized engines.
//! - [`ViewResponse`] — an already-rendered HTML body that converts into an
//!   axum response with `Content-Type: text/html; charset=utf-8`.
//! - [`View<T>`] — a thin wrapper around an askama [`askama::Template`] so a
//!   handler can return a compile-time-checked view directly.
//! - [`AskamaEngine`] — the default engine. Templates are compiled into the
//!   binary and registered by name at startup.
//! - [`MinijinjaEngine`] — opt-in runtime engine behind the `runtime-templates`
//!   feature for hot-reload and user-authored templates.
//!
//! # Template discovery
//!
//! Templates live under `resources/views/` at the application root. askama
//! resolves `#[template(path = "...")]` through the application's
//! `askama.toml`:
//!
//! ```toml
//! [general]
//! dirs = ["resources/views"]
//! ```
//!
//! The runtime engine is rooted at [`VIEWS_DIR`] by default.
//!
//! # Auto-escaping
//!
//! HTML auto-escaping is ON by default for both engines. askama escapes every
//! `{{ }}` expression in an `.html` template unless explicitly marked safe, and
//! minijinja selects HTML escaping for `.html`, `.htm`, and `.xml` templates.
//!
//! # Errors
//!
//! Rendering failures become [`ViewError`]. Its
//! [`axum::response::IntoResponse`] implementation returns a generic
//! `500 Internal Server Error` body, so template internals and filesystem
//! paths never reach the client.

#![deny(missing_docs)]

mod askama_engine;
mod error;
mod response;

#[cfg(feature = "runtime-templates")]
mod minijinja_engine;

#[cfg(test)]
mod test_support;

pub use askama;
pub use askama_engine::AskamaEngine;
pub use error::ViewError;
pub use response::{View, ViewResponse};

#[cfg(feature = "runtime-templates")]
pub use minijinja_engine::MinijinjaEngine;

use serde::Serialize;

/// Application-relative directory holding server-side templates.
pub const VIEWS_DIR: &str = "resources/views";

/// Rendering contract implemented by template engines.
///
/// The trait is object-safe, so an engine can be stored in the application
/// container as `Arc<dyn ViewEngine>`. Object-safe callers render from a
/// [`serde_json::Value`] via [`ViewEngine::render_value`]; sized callers use
/// the typed [`ViewEngine::render`] convenience.
pub trait ViewEngine: Send + Sync {
    /// Render the template registered as `name` with a JSON context.
    fn render_value(&self, name: &str, data: &serde_json::Value)
        -> Result<ViewResponse, ViewError>;

    /// Whether the engine can resolve `name`.
    fn contains(&self, name: &str) -> bool;

    /// Render `name` with typed `data` by serializing it to JSON first.
    ///
    /// The `Self: Sized` bound keeps the trait object-safe while preserving the
    /// ergonomic typed call site on concrete engines.
    fn render<T: Serialize>(&self, name: &str, data: &T) -> Result<ViewResponse, ViewError>
    where
        Self: Sized,
    {
        let value = serde_json::to_value(data).map_err(|source| ViewError::data(name, source))?;
        self.render_value(name, &value)
    }
}
