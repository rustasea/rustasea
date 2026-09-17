//! ExampleApp — RustaSea application library.
//!
//! The module tree mirrors the Laravel layout: `app/` holds domain actions,
//! concerns, HTTP, models, and providers; `bootstrap/` wires the kernel;
//! `routes/` owns the route tables; `database/` holds migrations, factories,
//! and seeders.

pub mod app;
pub mod bootstrap;
pub mod database;
pub mod routes;
