//! Feature test suite — end-to-end route → database → response (GAP-017).
//!
//! Requires Docker and a Postgres image, so the whole target is compiled only
//! with the `integration` cargo feature and every test is additionally marked
//! `#[ignore = "requires docker"]`. The default `cargo test -p rustasea`
//! therefore compiles an empty target and never touches Docker.
//!
//! Run the suite explicitly:
//!
//! ```text
//! cargo test -p rustasea --features integration -- --ignored
//! ```

#![cfg(feature = "integration")]

mod route_to_db;
