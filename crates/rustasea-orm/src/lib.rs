//! RustaSea ORM — models, fluent query builder, migrations, factories, vector support.
//!
//! Sprint 03 (M2) scope: fluent SQL-building core with typed errors, table naming
//! conventions, and feature-gated driver dialects (Postgres/MySQL/SQLite) plus the
//! `vector` feature for pgvector similarity search.

// Fail fast with a clear, actionable message when no driver feature is selected.
// Without a driver, `DbPool` would have zero variants and the crate would
// otherwise degrade into eight cryptic `error[E0004]: non-exhaustive patterns`
// diagnostics from the pool's `match self` blocks. Gating the module keeps the
// driver-less build to this single diagnostic.
#[cfg(not(any(feature = "postgres", feature = "mysql", feature = "sqlite")))]
compile_error!(
    "At least one database driver feature must be enabled: 'postgres', 'mysql', or 'sqlite'."
);

pub mod builder;
pub mod clause;
#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub mod database;
pub mod error;
pub mod execution;
pub mod factory;
pub mod m2;
pub mod migration;
pub mod model;
pub mod naming;
pub mod scopes;
pub mod tx;
pub mod types;
pub mod value;
pub mod vector;

pub use builder::{Lock, OrderDirection, QueryBuilder, Raw, TransactionStub};
#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub use database::{
    bind_database, Database, DatabaseConfig, DatabaseServiceProvider, DbPool, DbRow, PoolConfig,
    DATABASE_BINDING,
};
pub use error::{OrmError, Result, UpsertError};
pub use execution::{
    chunk_by, count_sql, raw, raw_sql, sum_sql, to_row_count_sql, transaction, PageMeta, Paginator,
};
pub use factory::{Factory, Seeder, SequenceFactory, SqlSeeder, User};
pub use m2::{InsertBuilder, ModelScopes, UpsertBuilder};
pub use migration::{Migration, MigrationError, Migrator};
pub use model::{Model, Relation, RelationKind, SoftDeletes, Timestamps};
pub use naming::snake_plural;
pub use scopes::{Scope, ScopeRegistry};
pub use tx::{Transaction, TransactionError};
pub use types::{ColumnType, JsonFilter};
pub use value::Value;
pub use vector::VectorSimilarity;
