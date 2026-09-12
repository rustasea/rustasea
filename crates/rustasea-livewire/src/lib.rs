//! RustaSea Livewire — the Laravel Livewire analogue for the starter kit
//! (ADR-0002 decision 5): server-rendered **askama** components enhanced with
//! **HTMX** partial swaps and realtime updates over **`rustasea-broadcast`**.
//!
//! # Model
//!
//! Unlike a stateful Livewire protocol that holds the component tree on the
//! server, this crate follows the blueprint's deliberately simpler model
//! (`.agents/documents/design/starter-kit-architecture.md` §D5):
//!
//! 1. A [`Component`] owns a JSON state object, an askama fragment template, an
//!    optional full-page template, and named actions.
//! 2. The browser holds the state and `POST`s it to
//!    `/livewire/:component/actions/:action`. The runtime authorizes the
//!    action, applies an optional shallow [`StatePatch`], runs the handler, and
//!    renders the fragment through a [`rustasea_view::ViewEngine`].
//! 3. The re-rendered fragment is returned for the HTMX swap **and** fanned out
//!    to the component's [`rustasea_broadcast::Channel`] so other subscribers
//!    receive it over SSE ([`sse_from_receiver`]) or WebSocket ([`ws_route`]).
//!
//! # Trust boundary
//!
//! State and patch values arrive from the browser and are untrusted. The
//! runtime always evaluates the [`ActionAuthorizer`] before touching state, but
//! action handlers must still validate any value they consume.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use rustasea_broadcast::BroadcastHub;
//! use rustasea_livewire::{AllowAll, Component, Livewire};
//! use rustasea_view::AskamaEngine;
//!
//! let mut engine = AskamaEngine::new();
//! engine.register::<CounterFragment>("counter");
//!
//! let mut livewire = Livewire::new(
//!     Arc::new(engine),
//!     Arc::new(AllowAll),
//!     BroadcastHub::new(channel_gate),
//! );
//! livewire.register(Component::new("counter", "counter"));
//!
//! let app = rustasea_livewire::routes().with_state(livewire);
//! ```

#![deny(missing_docs)]

mod authorizer;
mod component;
mod error;
mod htmx;
mod realtime;
mod runtime;
mod state;

#[cfg(test)]
mod test_support;

pub use authorizer::{ActionAuthorizer, Actor, AllowAll, DenyAll};
pub use component::{ActionHandler, ActionRequest, Component};
pub use error::{LivewireError, StateError};
pub use htmx::{is_htmx, routes, HX_REQUEST_HEADER};
pub use realtime::{sse_from_receiver, ws_route};
pub use runtime::{ActionResult, Livewire};
pub use state::{ComponentState, StatePatch};
