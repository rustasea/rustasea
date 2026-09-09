//! RustaSea umbrella crate — re-exports foundation, config, and M0–M4 crates.

pub use rustasea_cache as cache;
pub use rustasea_config as config;
pub use rustasea_events as events;
pub use rustasea_foundation as foundation;
pub use rustasea_queue as queue;
pub use rustasea_schedule as schedule;

pub use config::ConfigLoader;
pub use foundation::{Application, Container, ServiceProvider};
pub use rustasea_auth as auth;
pub use rustasea_cli as cli;
pub use rustasea_http as http;
pub use rustasea_macros as macros;
pub use rustasea_orm as orm;
pub use rustasea_router as router;
pub use rustasea_testing as testing;
pub use rustasea_validation as validation;

/// Queue re-exports for typed job dispatch ergonomics (M4).
pub use queue::{
    async_trait as queue_async_trait, ConcreteJob, DispatchHandle, ErasedJob, FailedJob, Job,
    JobError, JobId, JobOutcome, Queue, QueueDriver, QueueError, QueueRegistry, ShouldRetry,
    ShouldRetryUntil,
};

/// Cache re-exports for store/repository ergonomics (M4).
pub use cache::{CacheError, CacheManager, Lock, LockError, LockGuard, Store};

/// Events re-exports for dispatch ergonomics (M4).
pub use events::{
    async_trait as events_async_trait, Dispatcher, Event, EventError, JobAttempted, Listener,
    QueueBusy,
};

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
    AuthError, AuthManager, AuthUser, Credentials, CsrfError, CsrfLayer, DeserializationAllowList,
    Guard, JwtClaims, JwtConfig, JwtGuard, KeyBy, Limit, MemoryRateLimiter, PreventRequestForgery,
    RateLimiter, SecFetchSite, SessionGuard, SessionPolicy, ThrottleConfig, ThrottleLayer,
    ThrottleService, Token,
};

/// Validation re-exports (`#[validate]` wiring surface).
pub use validation::{ErrorBag, FormRequest, Rules, Validatable, ValidationError};

/// CLI re-exports (`Artisan::call`, `Command` trait, generators).
pub use cli::{
    Artisan, Command, CommandMeta, CommandOutput, CommandRegistry, Generator, GeneratorError, Io,
    Registered, Shutdownable,
};

/// Testing re-exports (`TestCase`, factories, paginator views).
pub use testing::{
    bootstrap_3, factory_registry, paginator_view, reset_factory_sequences, str_factory, TestCase,
    TestConfig, TestError,
};

/// M6 broadcast/storage/search/jsonapi re-exports (Sprint 07).
pub use rustasea_broadcast as broadcast;
pub use rustasea_jsonapi as jsonapi;
pub use rustasea_search as search;
pub use rustasea_storage as storage;

/// M6 AI SDK re-export — only with the `ai` feature (NFR-Sca-02).
#[cfg(feature = "ai")]
pub use rustasea_ai as ai;

/// Broadcast re-exports for channel/SSE ergonomics (M6).
pub use broadcast::{Authorize, BroadcastError, Channel, ShouldBroadcast};

/// Storage re-exports for read-through disk ergonomics (M6).
pub use storage::{
    LocalDisk, ObjectDisk, ReadThrough, Storage, StorageConfig, StorageError, StorageManager,
};

/// Search re-exports for vector ergonomics (M6).
pub use search::{
    MemoryVectorStore, Similarity as VectorSimilarity, Str, VectorDocument, VectorIndex,
    VectorIndexOps, VectorSearch, VectorSearchError,
};

/// JSON:API re-exports for resource ergonomics (M6).
pub use jsonapi::{
    content_type as jsonapi_content_type, Document as JsonApiDocument, JsonApiError,
    JsonApiResource, Link as JsonApiLink, Links as JsonApiLinks, ResourceBuilder, SparseFields,
};

/// AI re-exports only with the `ai` feature (NFR-Sca-02).
#[cfg(feature = "ai")]
pub use ai::{
    adapters, embed, Agent, AgentError, Ai, AiChunk, AiError, AiProvider, AiResponse, Capability,
    InProcessProvider, ProviderCall, StrToEmbeddings, Tool, ToolRegistry,
};
