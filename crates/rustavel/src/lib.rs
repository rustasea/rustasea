//! Rustavel umbrella crate — re-exports foundation, config, and M0–M4 crates.

pub use rustavel_cache as cache;
pub use rustavel_config as config;
pub use rustavel_events as events;
pub use rustavel_foundation as foundation;
pub use rustavel_queue as queue;
pub use rustavel_schedule as schedule;

pub use config::ConfigLoader;
pub use foundation::{Application, Container, ServiceProvider};
pub use rustavel_auth as auth;
pub use rustavel_http as http;
pub use rustavel_macros as macros;
pub use rustavel_orm as orm;
pub use rustavel_router as router;
pub use rustavel_validation as validation;

/// Queue re-exports for typed job dispatch ergonomics (M4).
pub use queue::{
    ConcreteJob, DispatchHandle, ErasedJob, FailedJob, Job, JobError, JobId, JobOutcome, Queue,
    QueueDriver, QueueError, QueueRegistry, ShouldRetry, ShouldRetryUntil,
};

/// Cache re-exports for store/repository ergonomics (M4).
pub use cache::{CacheError, CacheManager, Lock, LockError, LockGuard, Store};

/// Events re-exports for dispatch ergonomics (M4).
pub use events::{Dispatcher, Event, EventError, JobAttempted, Listener, QueueBusy};

/// Schedule re-exports for scheduler ergonomics (M4).
pub use schedule::{
    Schedule, ScheduleBuilder, ScheduleCommand, ScheduleError, SchedulePaused, ScheduleResumed,
    ScheduleState, Scheduler, SchedulerStatus,
};

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
