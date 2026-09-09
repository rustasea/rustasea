//! `#[route]` attribute macro — expansion checks (FR-100).
//!
//! The macro re-emits the annotated handler unchanged and emits a doc-hidden
//! `__RUSTASEA_ROUTE_<Fn>` const carrying `(method, path)` metadata. This
//! test asserts both: the function stays callable and the metadata const is
//! present with the parsed values.

use rustasea::macros::route;

#[route(method = "GET", path = "/users")]
async fn index() -> &'static str {
    "users"
}

#[route(method = "POST", path = "/users")]
async fn store() -> &'static str {
    "stored"
}

#[route(path = "/defaults")]
async fn defaults() -> &'static str {
    "method defaults to GET"
}

/// The handler bodies survive the macro untouched.
#[test]
fn route_macro_preserves_handlers() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    assert_eq!(rt.block_on(index()), "users");
    assert_eq!(rt.block_on(store()), "stored");
    assert_eq!(rt.block_on(defaults()), "method defaults to GET");
}

/// Metadata consts exist with the parsed method + path.
#[test]
fn route_macro_emits_metadata() {
    // The macro emits doc-hidden `__RUSTASEA_ROUTE_<Fn>` consts in the
    // same module; they carry the parsed (method, path) pair.
    assert_eq!(__RUSTASEA_ROUTE_index, ("GET", "/users"));
    assert_eq!(__RUSTASEA_ROUTE_store, ("POST", "/users"));
    // `method` is optional and defaults to GET.
    assert_eq!(__RUSTASEA_ROUTE_defaults, ("GET", "/defaults"));
}
