//! RustaSea Inertia WASM presentation adapters — the framework half of
//! ADR-0002 decision 4.
//!
//! `rustasea-inertia-client` owns the framework-agnostic Inertia protocol
//! (page parsing, the [`ComponentRegistry`] contract, and navigation decisions).
//! This crate turns that protocol into concrete presentation adapters:
//!
//! - the **`react`** feature maps the React starter-kit variant to **Dioxus**;
//! - the **`vue`** feature maps the Vue variant to **Leptos**.
//!
//! Both adapters share the same core:
//!
//! - [`RouterState`] holds the current [`Page`] and builds the `X-Inertia*`
//!   navigation headers through [`InertiaClient`];
//! - [`mount_page`] / [`mount_installed`] resolve `Page.component` through the
//!   generated [`ComponentRegistry`] (WASM has no reflection, so the scaffolder
//!   emits the map via [`inertia_registry!`](crate::inertia_registry));
//! - [`fetch_page`] performs an Inertia visit in the browser and returns the
//!   [`NavigationOutcome`] (mount in place, hard-navigate, or full reload);
//! - each variant exposes a `RouterProvider`, a `use_router` hook, and a `Link`
//!   component that triggers an Inertia request instead of a full page load.
//!
//! # Feature flags
//!
//! No framework is linked by default. Select exactly one variant:
//!
//! ```text
//! cargo check -p rustasea-inertia-adapters --no-default-features --features react
//! cargo check -p rustasea-inertia-adapters --no-default-features --features vue
//! ```
//!
//! # Build tooling
//!
//! The generated app compiles its `resources/js/` entrypoint to
//! `wasm32-unknown-unknown` with `wasm-bindgen` and bundles it with `trunk`
//! (see `Trunk.toml` and `index.html` in this crate for the reference config).
//!
//! ```rust,no_run
//! use rustasea_inertia_adapters::{
//!     install_registry, inertia_registry, ClientError, Value,
//! };
//!
//! fn mount_login(_props: &Value) -> Result<(), ClientError> {
//!     Ok(())
//! }
//!
//! inertia_registry!(AppRegistry {
//!     "auth/login" => mount_login,
//! });
//!
//! fn boot(data_page: &str) {
//!     install_registry(Box::new(AppRegistry::new()));
//!     let mut state = rustasea_inertia_adapters::RouterState::new();
//!     state.hydrate(data_page).expect("hydrate the initial page");
//! }
//! ```

#![deny(missing_docs)]

mod error;
mod registry;
mod router;

#[cfg(any(feature = "react", feature = "vue"))]
mod browser;

pub use error::AdapterError;
pub use registry::{install_registry, mount_installed, mount_page};
pub use router::RouterState;

#[cfg(any(feature = "react", feature = "vue"))]
pub use browser::{fetch_page, hard_navigate};

// Re-export the client surface so a generated app depends on a single crate.
pub use rustasea_inertia_client::{
    inertia_registry, ClientError, ComponentRegistry, InertiaClient, NavigationOutcome, Page, Value,
};

#[cfg(feature = "react")]
mod dioxus_adapter;
#[cfg(feature = "react")]
pub use dioxus_adapter::{
    use_router as use_dioxus_router, Link as DioxusLink, RouterContext as DioxusRouterContext,
    RouterProvider as DioxusRouterProvider,
};

#[cfg(feature = "vue")]
mod leptos_adapter;
#[cfg(feature = "vue")]
pub use leptos_adapter::{
    use_router as use_leptos_router, Link as LeptosLink, RouterContext as LeptosRouterContext,
    RouterProvider as LeptosRouterProvider,
};
