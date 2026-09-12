//! RustaSea Inertia server — the Rust-native Inertia protocol implementation.
//!
//! The crate implements the server half of ADR-0002 decision 3: a [`Page<T>`]
//! envelope, the full `X-Inertia*` header contract, partial reloads filtered on
//! the wire prop map, per-request shared props, and `409 Conflict` +
//! `X-Inertia-Location` on an asset-version mismatch.
//!
//! # Wire behavior
//!
//! - A request carrying `X-Inertia: true` receives `application/json` with the
//!   page object; any other request receives the HTML root document with the
//!   page embedded in `data-page` for hydration.
//! - Every rendered response sets `Vary: X-Inertia` for cache correctness and
//!   `X-Inertia: true` on the JSON variant.
//! - A `GET` carrying a stale `X-Inertia-Version` is answered with `409` and
//!   `X-Inertia-Location` so the client hard-navigates.
//! - `X-Inertia-Partial-Component` must match the rendered component for
//!   `X-Inertia-Partial-Data` / `X-Inertia-Partial-Except` to filter props.
//!
//! # Feature flags
//!
//! The protocol surface ([`Page`], [`InertiaRequest`], [`PartialReload`], and
//! the header constants) is always available and WASM-safe, so
//! `rustasea-inertia-client` can share it with `default-features = false`.
//! The `server` feature (enabled by default) adds the axum glue:
//! [`Inertia`], [`InertiaResponse`], [`RootView`], [`InertiaError`], and
//! [`handle_inertia_requests`].

#![deny(missing_docs)]

pub use http;

mod page;
mod request;

pub use page::Page;
pub use request::{InertiaRequest, PartialReload};

#[cfg(feature = "server")]
pub use axum;

#[cfg(feature = "server")]
mod error;
#[cfg(feature = "server")]
mod inertia;
#[cfg(feature = "server")]
mod middleware;
#[cfg(feature = "server")]
mod response;
#[cfg(feature = "server")]
mod root_view;

#[cfg(feature = "server")]
pub use error::InertiaError;
#[cfg(feature = "server")]
pub use inertia::Inertia;
#[cfg(feature = "server")]
pub use middleware::handle_inertia_requests;
#[cfg(feature = "server")]
pub use response::InertiaResponse;
#[cfg(feature = "server")]
pub use root_view::{escape_html_attribute, HtmlShell, RootView};

/// Request/response marker header: marks an Inertia request or JSON response.
pub const X_INERTIA: &str = "X-Inertia";
/// Request header carrying the client's asset version.
pub const X_INERTIA_VERSION: &str = "X-Inertia-Version";
/// Response header holding the hard-navigation target on a version conflict.
pub const X_INERTIA_LOCATION: &str = "X-Inertia-Location";
/// Request header naming the component a partial reload targets.
pub const X_INERTIA_PARTIAL_COMPONENT: &str = "X-Inertia-Partial-Component";
/// Request header listing the prop names a partial reload allows.
pub const X_INERTIA_PARTIAL_DATA: &str = "X-Inertia-Partial-Data";
/// Request header listing the prop names a partial reload excludes.
pub const X_INERTIA_PARTIAL_EXCEPT: &str = "X-Inertia-Partial-Except";
/// Request header listing client-side props to reset rather than merge.
pub const X_INERTIA_RESET: &str = "X-Inertia-Reset";
/// Response header preserving cache correctness across Inertia/HTML variants.
pub const VARY: &str = "Vary";
