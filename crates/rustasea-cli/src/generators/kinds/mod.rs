//! Scaffold templates — shared helpers plus the CRUD group
//! (controller/model + migration), matching the FSD FS-M5-02 split.
//!
//! Template source is emitted **pre-formatted**: stable indentation, no
//! trailing whitespace and rustfmt-clean layout so generated files pass
//! `rustfmt --check` and `clippy -- -D warnings` unchanged (C-04).

use std::path::Path;

use crate::error::CliResult;
use crate::generator::{Generated, Generator};

pub mod agent;
pub mod command;
pub mod controller;
pub mod event;
pub mod job;
pub mod listener;
pub mod migration;
pub mod model;
pub mod observer;
pub mod provider;
pub mod seeder;
pub mod test;
pub mod tool;

pub use controller::scaffold as scaffold_controller;
pub use migration::scaffold as scaffold_migration;
pub use model::scaffold as scaffold_model;
pub use model::scaffold_migration as scaffold_model_migration;

/// Path fragment of a scaffold name (`UserController` → `user_controller`).
pub(crate) fn slug(name: &str) -> String {
    Generator::snake(name)
}

/// Current UTC date in `YYYY_MM_DD_HHMMSS` migration prefix form.
pub(crate) fn migration_prefix() -> String {
    chrono::Utc::now().format("%Y_%m_%d_%H%M%S").to_string()
}

/// Resolve and write one pre-formatted scaffold file.
pub(crate) fn write_scaffold(
    root: &Path,
    relative_path: String,
    source: String,
    force: bool,
) -> CliResult<Generated> {
    crate::generator::scaffold(root, &relative_path, source, force)
}
