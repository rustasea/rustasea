//! Rustavel umbrella crate — re-exports foundation, config, and M0–M3 crates.

pub use rustavel_config as config;
pub use rustavel_foundation as foundation;

pub use config::ConfigLoader;
pub use foundation::{Application, Container, ServiceProvider};
pub use rustavel_auth as auth;
pub use rustavel_http as http;
pub use rustavel_macros as macros;
pub use rustavel_orm as orm;
pub use rustavel_router as router;
pub use rustavel_validation as validation;

pub use orm::{
    Migration, Migrator, Model, OrmError, Paginator, QueryBuilder, Relation, Result as OrmResult,
    ScopeRegistry, SoftDeletes, Timestamps, UpsertError,
};

/// Auth re-exports for handler ergonomics (`Auth::guard`, guards, CSRF).
pub use auth::{
    AuthError, AuthManager, AuthUser, Credentials, CsrfError, CsrfLayer, Guard, JwtClaims,
    JwtConfig, JwtGuard, KeyBy, Limit, MemoryRateLimiter, PreventRequestForgery, RateLimiter,
    SecFetchSite, SessionGuard, SessionPolicy, ThrottleConfig, ThrottleLayer, ThrottleService,
    Token,
};

/// Validation re-exports (`#[validate]` wiring surface).
pub use validation::{ErrorBag, FormRequest, Rules, Validatable, ValidationError};
