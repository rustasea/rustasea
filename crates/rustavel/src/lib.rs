//! Rustavel umbrella crate — re-exports foundation and config.

pub use rustavel_config as config;
pub use rustavel_foundation as foundation;

pub use config::ConfigLoader;
pub use foundation::{Application, Container, ServiceProvider};
pub use rustavel_http as http;
pub use rustavel_macros as macros;
pub use rustavel_orm as orm;
pub use rustavel_router as router;

pub use orm::{
    Migration, Migrator, Model, OrmError, Paginator, QueryBuilder, Relation, Result as OrmResult,
    ScopeRegistry, SoftDeletes, Timestamps, UpsertError,
};
