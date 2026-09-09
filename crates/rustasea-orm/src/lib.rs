//! RustaSea ORM — models, fluent query builder, migrations, factories, vector support.
//!
//! Sprint 03 (M2) scope: fluent SQL-building core with typed errors, table naming
//! conventions, and feature-gated driver dialects (Postgres/MySQL/SQLite) plus the
//! `vector` feature for pgvector similarity search.

pub mod builder;
pub mod clause;
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
