//! RustaSea Inertia WASM client — the browser half of ADR-0002 decision 3.
//!
//! The crate holds the framework-agnostic protocol logic that a generated
//! `resources/js/` entrypoint links into its WASM bundle:
//!
//! - parses the [`Page<T>`] envelope from a `data-page` attribute or a JSON
//!   response body;
//! - resolves `Page.component` through a generated [`ComponentRegistry`]
//!   (WASM has no reflection, so the scaffolder emits the `match` explicitly —
//!   see [`inertia_registry!`](crate::inertia_registry));
//! - decides how to react to a response: mount in place, hard-navigate on a
//!   `409`, or perform a full document load;
//! - builds the `X-Inertia*` headers for a navigation or partial reload.
//!
//! The concrete component implementations (Dioxus for the `react` variant,
//! Leptos for `vue`) are supplied by the generated registry, not by this crate.
//! It deliberately avoids DOM or framework dependencies so it compiles to
//! `wasm32-unknown-unknown` with no host-specific baggage.
//!
//! ```rust
//! use rustasea_inertia_client::{inertia_registry, ClientError, ComponentRegistry, Value};
//!
//! fn mount_login(_props: &Value) -> Result<(), ClientError> {
//!     Ok(())
//! }
//!
//! inertia_registry!(AppRegistry {
//!     "auth/login" => mount_login,
//! });
//!
//! fn main() {
//!     let registry = AppRegistry::new();
//!     assert!(registry.mount("auth/login", &Value::Null).is_ok());
//! }
//! ```

#![deny(missing_docs)]

mod client;
mod error;
mod registry;

pub use client::{InertiaClient, NavigationOutcome};
pub use error::ClientError;
pub use registry::ComponentRegistry;
pub use rustasea_inertia::Page;

/// JSON value type re-exported for registry mount signatures and the
/// [`inertia_registry!`](crate::inertia_registry) macro.
pub use serde_json::Value;
