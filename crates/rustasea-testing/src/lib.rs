//! RustaSea testing harness (FS-M5-04, FR-507..509).
//!
//! A `TestCase` implementor provisions isolated stores for one test binary;
//! factory sequences reset between tests so parallel suites never observe a
//! leaked `Str` counter; paginators render `bootstrap-3` views.
//!
//! Feature `containers` (default) wires the Postgres/Redis container helpers
//! (docker-backed) with a 30s health timeout; disabling it keeps the crate
//! compiling with a thin dependency tree for pure logic tests.

#[cfg(feature = "containers")]
pub mod containers;
pub mod error;
pub mod factory;
#[cfg(feature = "postgres")]
pub mod fixtures;
pub mod migration;
pub mod paginator;
pub mod test_case;

#[cfg(feature = "containers")]
pub use containers::{
    postgres_container, redis_container, teardown_all, ContainerError, ContainerHandle,
};
pub use error::{Result, TestError};
pub use factory::{
    factory_registry, register_sequence, reset_factory_sequences, str_factory, StrFactory,
};
#[cfg(feature = "postgres")]
pub use fixtures::{FixtureError, PostgresTestDb};
pub use migration::{migrate_once, MigrateHarness};
pub use paginator::{bootstrap_3, paginator_view};
pub use test_case::{TestCase, TestConfig};
