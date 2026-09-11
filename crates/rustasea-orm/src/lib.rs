//! RustaSea ORM — models, fluent query builder, migrations, factories, vector support.
//!
//! Sprint 03 (M2) scope: fluent SQL-building core with typed errors, table naming
//! conventions, and feature-gated driver dialects (Postgres/MySQL/SQLite) plus the
//! `vector` feature for pgvector similarity search.

pub mod blueprint;
pub mod builder;
pub mod clause;
pub mod db;
pub mod eager;
pub mod error;
pub mod execution;
pub mod factory;
pub mod m2;
pub mod migration;
pub mod model;
pub mod model_ops;
pub mod naming;
pub mod relations;
pub mod scopes;
pub mod tx;
pub mod types;
pub mod value;
pub mod vector;

pub use blueprint::Blueprint;
pub use builder::{Executor, Lock, OrderDirection, QueryBuilder, Raw};
pub use db::DbPool;
pub use eager::EagerPlan;
pub use error::{OrmError, Result, UpsertError};
pub use execution::{
    chunk_by, count_sql, raw, raw_sql, sum_sql, to_row_count_sql, transaction, Links, PageMeta,
    PaginationMeta, Paginator,
};
pub use factory::{Factory, FactoryState, SequenceFactory, SqlSeeder, User};
pub use m2::{InsertBuilder, ModelScopes, UpsertBuilder};
pub use migration::{
    register_migration, register_seeder, registered_migrator, Migration, MigrationError,
    MigrationRecord, Migrator, Seeder,
};
pub use model::{Model, Relation, RelationKind, Relations, SoftDeletes, Timestamps};
pub use model_ops::ModelOps;
pub use naming::snake_plural;
pub use scopes::{Scope, ScopeRegistry};
pub use tx::{Transaction, TransactionError};
pub use types::{ColumnType, JsonFilter};
pub use value::Value;
pub use vector::{VectorMetric, VectorSimilarity};
