//! Test-only askama fixtures for the crate's unit tests.
//!
//! Compiled exclusively under `#[cfg(test)]`; nothing here is public API.

use serde::Deserialize;

/// Askama fragment template resolving through this crate's `askama.toml`.
#[derive(askama::Template, Deserialize)]
#[template(path = "counter.html")]
pub(crate) struct CounterFragment {
    /// Counter value interpolated into the fragment.
    pub(crate) count: u32,
}

/// Askama full-page template resolving through this crate's `askama.toml`.
#[derive(askama::Template, Deserialize)]
#[template(path = "counter_page.html")]
pub(crate) struct CounterPage {
    /// Counter value interpolated into the page.
    pub(crate) count: u32,
}
