//! RustaSea starter-kit scaffolder — the library behind `cargo rustasea new`.
//!
//! The crate implements ADR-0002 decision 6: one shared auth/domain core plus a
//! variant-specific presentation layer. It generates the complete application
//! tree described by the starter-kit blueprint
//! (`.agents/documents/design/starter-kit-architecture.md` §5) for the `blade`,
//! `react`, `vue`, and `livewire` kits.
//!
//! # Guarantees
//!
//! - **No network at generation time.** Every template is embedded in the
//!   binary as a `&'static str`; generation only touches `std::fs`.
//! - **No partial writes on conflict.** A scaffold run fails on the first
//!   existing file unless [`Scaffold::with_force`] is set, so a rejected run
//!   cannot silently half-overwrite a tree.
//! - **Clear, typed errors.** [`ScaffoldError`] distinguishes invalid names,
//!   unknown variants, conflicting files, and I/O failures.
//!
//! # Example
//!
//! ```rust
//! use rustasea_scaffold::{Scaffold, StarterKitVariant};
//!
//! let files = Scaffold::new("my-app", StarterKitVariant::Blade)
//!     .render()
//!     .expect("render");
//! assert!(files.iter().any(|f| f.path == "routes/web.rs"));
//! ```
//!
//! The `cargo-rustasea` binary is a thin CLI over this library; it owns process
//! concerns (argument parsing, exit codes, optional `git init`).

#![deny(missing_docs)]

mod error;
mod name;
mod scaffold;
mod templates;
mod variant;

pub use error::{ScaffoldError, ScaffoldResult};
pub use name::AppName;
pub use scaffold::{resolve_target, Generated, RenderedFile, Scaffold};
pub use variant::StarterKitVariant;
