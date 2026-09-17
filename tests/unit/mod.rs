//! Unit tests — domain actions and concerns in isolation.
//!
//! This module is the crate root of the `unit` test target; `Cargo.toml` wires
//! it with an explicit `[[test]]` entry because cargo does not auto-discover
//! `tests/unit/mod.rs`.

pub mod actions_test;
