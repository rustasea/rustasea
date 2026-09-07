# Rustavel — Technical Design Document (TDD)

> **Status:** Final — P6 Planning Docs Finalization
> **Date:** 2026-09-07 · **Finalized:** 2026-09-07  
> **Type:** Technical Design — implementation contract for M0–M6  
> **Parents:** `brd.md` · `prd.md` (FR-000–FR-612, NFRs) · `fsd.md` (FS-M0-01–FS-M6-07) · `architecture.md` · `domain.md` · `database.md` · `api-contracts.md`  
> **Audience:** Engineering, Security, Platform/SRE. This is the buildable handshake between product (BRD/PRD/FSD) and code (workspace crates).

---

## 1. Purpose

This TDD turns the FSD's feature specs into implementation contracts: concrete trait signatures, crate boundaries, error enums, state machines, non-functional budgets, and rollout dependencies. It is intentionally duplicative of `architecture.md`/`domain.md` at interface level so a reader can implement without cross-referencing four docs per crate.

---

## 2. System Overview

Single `tokio` binary with three task groups sharing `AppState: Arc<AppState>`:

```
AppState { app: Application, config: ConfigRegistry, container: Container, db: Db, cache: CacheManager, queue: QueueRegistry, ... }
main
 ├─ axum::Server (Router from rustavel-router)
 ├─ queue workers (deadpool-redis BRPOP / DB poll)
 └─ scheduler ticker (cron eval every 60s; respects schedule_paused flag)
Signal SIGTERM/SIGINT -> drain HTTP -> stop ticker -> drain queue -> flush after-response buffer -> exit 0
```

Crate map and deployment topology: see `architecture.md §3–§7`. Bounded contexts: see `domain.md §2–§5`. Schema per context: see `database.md §2–§7`.

---

## 3. Module Contracts (by Bounded Context)

### BC-0 — Foundation (`rustavel-foundation` + `rustavel-config`)

#### Traits

```rust
trait ServiceProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn depends_on(&self) -> &'static [&'static str] { &[] }
    fn register(&self, c: &mut Container) -> Result<()>;
    fn boot(&self, state: &AppState) -> Result<()>;
}
trait Runner: Send + Sync { async fn run(&self, state: AppState, shutdown: Shutdown) -> Result<()>; }

struct Application {
    providers: Vec<Box<dyn ServiceProvider>>,
    runners: Vec<Box<dyn Runner>>,
    config: ConfigRegistry,
}
impl Application {
    fn configure() -> AppBuilder;
    async fn boot(self) -> Result<AppState>; // validates DAG, register->boot, spawns runners
    fn shutdown(&self) -> ShutdownHandle;
}
enum ContainerError { NotFound { type_name: &'static str }, AlreadyBound { type_name: &'static str } }
struct Container { /* Bind/Singleton/Instance registry */ }
impl Container {
    fn bind<T: Send+Sync+'static>(&mut self, f: impl FnOnce() -> T + Send + Sync + 'static);
    fn singleton<T: Send+Sync+'static>(&mut self, f: impl FnOnce() -> T + Send + Sync + 'static);
    fn instance<T: Send+Sync+'static>(&mut self, v: Arc<T>);
    fn make<T: Send+Sync+'static>(&self) -> Result<Arc<T>, ContainerError>;
    fn make_opt<T: Send+Sync+'static>(&self) -> Option<Arc<T>>;
}
```

Config layering `defaults < config/*.toml < .env < env`, typed `Deserialize`, `ConfigError::Parse { file, line, source }` (see `architecture.md §5`).

### BC-1 — HTTP & Routing (`rustavel-router` + `rustavel-http` + `rustavel-macros`)

```rust
struct Route { method: Method, path: Cow<'static, str>, name: Option<Cow<'static, str>>, handler: Handler, domain: Option<Cow<'static, str>> }
struct RouteRegistry { /* axum::Router + named table + middleware chain */ }
impl RouteRegistry {
    fn get(&mut self, path: impl Into<Cow<'static,str>>, handler: Handler) -> RouteBuilder;
    // post/put/delete/patch/options/any, group(prefix,middleware,children), resource(name,controller)
    fn list(&self) -> Vec<RouteMeta>; // for route:list --json with binding_fields
}
// Proc-macros: #[route(GET, "/users/{user:slug}")] , #[middleware("auth:jwt")]
// Extractors: Json<T>, Query<T>, Path<T>, State<AppState>  (axum-native)
// Middleware: Throttle::per_minute(60).by_ip() | .by_user() | .by_key(fn), Cors::allow_origins([...])
// Responses: Json<T>, View<T> (askama/minijinja), Redirect, EventStream
// Http client: Http::get(url).header(k,v).timeout(d).throw(|resp| bool) -> Result<Response, HttpError>
enum HttpError { Status{ code: u16 }, Timeout{ kind: Connect|Total|Idle }, ThrowCallback{ source: Box<dyn Error> } }
```

Domain-route precedence invariant: domain routes evaluated before non-domain (FSD FS-M1-02). `route:list --json` shape defined in `api-contracts.md §1`.

### BC-2 — Data & ORM (`rustavel-orm` + `rustavel-macros`)

```rust
#[derive(Model)] // generates id/created_at/updated_at/deleted_at, table_name(), relations(), serde preservation
struct User { id: Uuid, name: String, email: String, posts: HasMany<Post> }

trait Model: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn table_name() -> &'static str;
    fn query() -> QueryBuilder<Self> where Self: Sized;
}

struct QueryBuilder<T> { /* sqlx state */ }
impl<T: Model> QueryBuilder<T> {
    fn where_eq(self, col: &str, val: impl Serialize) -> Self;
    fn or_where(self, col: &str, val: impl Serialize) -> Self;
    fn where_vector_similar_to(self, col: &str, embedding: &[f32], limit: usize) -> Self; // vector ext
    // + orWhere/whereJson/whereBinary/chunkBy/orWhereKey/paginate/cursor/forUpdate/toSql/toRawSql ...
    async fn first(self) -> Result<Option<T>, QueryError>;
    async fn first_or_fail(self) -> Result<T, QueryError>; // NotFound
    async fn create(self, data: T) -> Result<T, QueryError>;
    async fn paginate(self, per_page: usize) -> Result<Paginated<T>, QueryError>;
}
// Migrations: Versioned up/down in database/migrations/*.rs ; sqlx::migrate! execution
// Factories: Factory<T>::create(n) with definition() + sequence + Str reset per TestCase
enum QueryError { NotFound, InvalidCursor, UpsertEmptyUniqueBy, VectorDimensionMismatch{ expected: usize, actual: usize }, ExtensionMissing{ extension: &'static str } }
```

Vector Blueprint: `Blueprint::vector("embedding", 1536)` and `dropVectorIndex("embedding")`; distance operators `<=>`/` <->`; HNSW/IVFFLAT strategies (see `database.md §4–5`).

### BC-3 — Identity & Access (`rustavel-auth` + `rustavel-validation`)

```rust
trait Guard: Send + Sync {
    async fn login(&self, creds: &Credentials) -> Result<Token, AuthError>;
    async fn parse(&self, token: &str) -> Result<AuthUser, AuthError>;
    async fn refresh(&self, token: &str) -> Result<Token, AuthError>;
    async fn logout(&self, token: &str) -> Result<(), AuthError>;
}
struct AuthManager { /* guard registry */ }
impl AuthManager {
    fn guard(&self, name: &str) -> Result<&dyn Guard, AuthError>; // GuardMismatch on miss
    fn extend(&mut self, name: &'static str, factory: impl Fn(&AppState) -> Box<dyn Guard> + Send + Sync + 'static);
}
enum AuthError { GuardMismatch{ expected: String, actual: String }, BadCredentials, InvalidToken, ExpiredToken }

struct PreventRequestForgery { allowed_origins: Vec<String> } // checks Sec-Fetch-Site + token; GET/HEAD/OPTIONS exempt

// Validation
trait Validatable: DeserializeOwned { fn validate(&self) -> Result<(), ErrorBag>; }
struct ErrorBag { fields: HashMap<String, Vec<ValidationError>> } // 422 JSON on failure
// #[validate] wires validator derive + strict helpers (in_array strict); #[middleware("auth:jwt")], #[authorize("update", User)]
```

Session hardening: `session.serialization = "json"` default, `serializable_classes` allow-list checked before `Deserialize`, hyphenated `-cache-`/`-session-` prefixes (see `architecture.md §5`).

### BC-4 — Async Workloads (`rustavel-queue` + `rustavel-cache` + `rustavel-events` + `rustavel-schedule`)

```rust
trait Job: Serialize + DeserializeOwned + Send + Sync + 'static {
    async fn handle(self) -> Result<(), JobError>;
}
trait ShouldRetry { fn should_retry(&self, attempt: u32, err: &JobError) -> bool; }
trait ShouldRetryUntil { fn retry_until(&self) -> Option<DateTime<Utc>>; }
// Attributes: #[tries(3)] #[backoff(10)] #[timeout(30)] #[failOnTimeout]

struct QueueRegistry { /* OnceLock<HashMap<TypeId, Route>> */ }
impl QueueRegistry {
    fn route<J: Job>(&mut self, connection: &'static str, queue: &'static str) -> Result<(), QueueError>; // DuplicateRoute on second
    async fn dispatch<J: Job>(&self, job: J) -> Result<JobId, QueueError>; // respects registry + onQueue override
    async fn pending_size(&self, conn: &str, queue: &str) -> Result<usize, QueueError>;
    async fn creation_time_of_oldest_pending_job(&self, conn: &str, queue: &str) -> Result<Option<DateTime<Utc>>, QueueError>;
}
enum JobError { Timeout, MaxAttemptsExceeded, Exception(String) }
enum QueueError { DuplicateRoute{ type_name: &'static str }, UnknownConnection, StoreUnavailable }

// Store trait (extensible by custom drivers without breaking change)
#[async_trait]
trait Store: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn put(&self, key: &str, val: Vec<u8>, ttl: Duration) -> Result<(), CacheError>;
    async fn forget(&self, key: &str) -> Result<(), CacheError>;
    async fn flush(&self) -> Result<(), CacheError>;
    async fn increment(&self, key: &str, n: i64) -> Result<i64, CacheError>;
    async fn decrement(&self, key: &str, n: i64) -> Result<i64, CacheError>;
    async fn touch(&self, key: &str, ttl: Duration) -> Result<bool, CacheError> { Ok(false) } // default Unsupported
    // Repository adds remember/forever/pull/has/withContext/store(name)
}
struct Lock { key: String, ttl: Duration }
impl Lock {
    async fn get(&self) -> Result<Option<LockGuard>, CacheError>;    // SET NX EX
    async fn block(&self, wait: Duration) -> Result<LockGuard, CacheError>; // retries until wait expires
}
enum CacheError { StoreUnavailable, LockAlreadyHeld }

// Events
#[async_trait]
trait Event: Serialize + DeserializeOwned + Send + Sync + 'static {}
#[async_trait]
trait Listener<E: Event>: Send + Sync { const QUEUE: bool = false; async fn handle(&self, event: E) -> Result<(), EventError>; }
struct Dispatcher;
impl Dispatcher {
    async fn dispatch<E: Event>(&self, event: E) -> Result<(), EventError>;
    async fn dispatch_after_response<E: Event>(&self, event: E) -> Result<(), EventError>; // buffered until Response sent
}
// Renames: JobAttempted { exception } (not exceptionOccurred), QueueBusy { connectionName }

// Scheduler
struct Schedule { /* cron + modifiers */ }
impl Schedule {
    fn command(cmd: &'static str) -> ScheduleBuilder;
}
struct ScheduleBuilder { /* daily().at("08:00").skipIfStillRunning().onOneServer() */ }
// Commands: schedule:list / schedule:run / schedule:pause / schedule:resume ; emits SchedulePaused/Resumed events
```

### BC-5 — Developer Platform (`rustavel-cli` + `rustavel-macros` + `rustavel-testing`)

- CLI via `clap` derive + `cargo xtask` (`cargo-xtask` bin). Commands implement `Command { fn signature()->&str; async fn handle(args, io) -> ExitCode }` with typed `Args`/`Flags`. Declarative `#[usage]`/`#[help]`/`#[hidden]`.
- Generators `make:*` (controller/model/provider/command/job/event/listener/observer/test/seeder/agent/tool) each produce `rustfmt`+`clippy -D warnings` clean `.rs` files; model with `-m` also emits a migration.
- `Artisan::call(cmd, args) -> CommandOutput` is in-process invocation (no subprocess).
- `TestCase` trait sets up isolated Postgres (random port via `testcontainers`) + Redis; runs `migrate` once per test binary; resets `Str` factory sequences between tests; loads `.env.testing` overlay; teardown kills containers.

### BC-6 — Intelligence & Delivery (`rustavel-broadcast` + `rustavel-storage` + `rustavel-search` + `rustavel-ai`)

```rust
#[async_trait]
trait AiProvider: Send + Sync {
    async fn text(&self, prompt: &str) -> Result<AiResponse, AiError>;
    async fn image(&self, prompt: &str) -> Result<AiImage, AiError>;
    async fn audio(&self, prompt: &str) -> Result<AiAudio, AiError>;
    async fn embeddings(&self, text: &str) -> Result<Vec<f32>, AiError>;
    async fn rerank(&self, query: &str, docs: &[String]) -> Result<Vec<RankedDoc>, AiError>;
    async fn files(&self, op: FileOp) -> Result<FileRef, AiError>;
    async fn vector_stores(&self, op: VectorStoreOp) -> Result<VectorStore, AiError>;
    // per-method UnsupportedCapability -> AiError::UnsupportedCapability { provider, capability }
}
struct AiRegistry { /* 12 adapters: openai, anthropic, gemini, azure, bedrock, groq, xai, deepseek, mistral, ollama, openrouter, openai_compatible — each feature-flagged */ }

#[async_trait]
trait Tool: Send + Sync { fn name(&self) -> &'static str; fn schema(&self) -> serde_json::Value; async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>; }
#[async_trait]
trait Agent: Send + Sync {
    async fn prompt(&self, input: String) -> Result<impl Stream<Item=AiChunk>, AgentError>;
    // deferred loaders: SimilaritySearch / FileStorage / ToolSearch inject context before tool call
    // middleware: wrap prompt->next->output ; sub-agents invoked as tools ; anonymous via Ai::agent(|a| a.tool(...))
}

// Broadcast
trait ShouldBroadcast: Serialize + Send + Sync { fn broadcast_on(&self) -> Channel; }
enum Channel { Public(String), Private(String), Presence(String) } // auth gate checks Authorize
// axum::extract::ws + tokio-tungstenite ; Response::eventStream(stream) -> Content-Type: text/event-stream

// Storage
struct StorageManager { disks: HashMap<String, Disk> }
impl StorageManager {
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError>; // read-through primary->fallback + optional copy_back
    fn path(&self, path: &str) -> Result<PathBuf, StorageError>;       // canonicalizes, enforces starts_with(disk_root)
}
enum StorageError { NotFound, PathTraversal, StoreUnavailable }

// JSON:API
trait JsonApiResource: Serialize {
    fn fields(&mut self, fields: &[&str]) -> &mut Self;    // sparse fieldsets
    fn include(&mut self, relation: &str) -> &mut Self;     // must be eager-loaded or RelationNotLoaded
    fn to_response(self) -> Response; // Content-Type: application/vnd.api+json
}
```

---

## 4. Non-Functional Budgets

| NFR | Target | Measurement | Gate |
|-----|--------|-------------|------|
| Cold boot to ready (listening) | <2s on CI (2 vCPU) with 5 providers | `cargo run` wall-clock until `Application::running()` | M0 tagged |
| HTTP p95 (no DB) | <50ms at 1k RPS localhost | `GET /users` axum hello via `criterion`/`oha` bench | M1 |
| Cache `get` p95 | <5ms (memory `moka`) / <20ms (redis localhost) | `testcontainers` bench | M4 |
| `cargo check` after `make:*` | <10s incremental | `cargo check --timings` | M5 |
| Shutdown drain | ≤ `shutdown_timeout` (default 10s) | in-flight request completes | M0/BC-0 |
| Single-crate check | `cargo check -p rustavel-router` pulls no `sqlx`/`async-openai` | `cargo tree` in CI | Always |
| Workspace acyclicity | no cycle | `xtask check-cycles` | CI |

Security gates enumerated in `architecture.md §5` and `prd.md NFR-Sec-*`; path confinement fuzzed.

---

## 5. Dependencies & Rollout

Strict milestone DAG (no cycles) — see `fsd.md §5` and `architecture.md §3`:

```
M0 (BC-0) — no prereq → M1 (BC-1) + M2 (BC-2) → M3 (BC-3) → M4 (BC-4) → M5 (BC-5) → M6 (BC-6)
Tagged releases: 0.1 (M0) → 0.2 (M1) → 0.3 (M2) → 0.4 (M3) → 0.5 (M4) → 0.6 (M5) → 1.0 (M6)
```

Public M0 at `0.1.0` with `cargo rustavel new` is the adoption gate (see `validation.md §6` conditions). M6 work cannot start before M2 success criteria (vector primitive + serde round-trip + migrations) are green.

---

## 6. Risks & Mitigations

See `prd.md §7` (9 risks with L×I scoring; top-5 R-01 ORM duality, R-05 queue routing, R-06 pgvector availability, etc.) and `validation.md §4–5`. This TDD inherits those mitigations and adds: contract evolution via trait default impls, `pgvector` guarded by `has_extension("vector")` + feature flag, `cargo tree` incremental-adoption CI, and `schedule:pause` idempotence.

---

## 7. Verification

- Every crate's public surface is one of the trait/struct signatures above or a proc-macro attribute — no hidden globals.
- Cross-refs: HTTP routes (§BC-1) map to `api-contracts.md §1`; vector/queue/cache map to `api-contracts.md §2–§4`; Auth/CSRF map to `api-contracts.md §3`.
- Typed errors carry `code`+`hint`+`source` chain per FSD §4.1; `#[deny(clippy::unwrap_used)]` in every framework crate.

## 8. Traceability — Requirements → Architecture → Tests (P6)

| BC | TDD Contract | FSD FS | PRD FRs | BR | Architecture | Design | API Contract | Tests (BDD tag) | QA Artifacts |
|----|--------------|--------|---------|----|--------------|--------|--------------|-----------------|--------------|
| BC-0 | Application, Container, ServiceProvider, Runner, Shutdown | FS-M0-01..04 | FR-000..008 | BR-01 | architecture.md §2-3 (BC-0 DAG) | domain.md BC-0 | api-contracts.md §1 (M0) | @foundation, @container, @observability-tooling | application/testing/stubs/m0-foundation.stub.rs |
| BC-1 | Route, RouteRegistry, Middleware, Extractors, Http client, HttpError | FS-M1-01..06 | FR-100..109 | BR-02 | BC-1 + ADR-001 | domain.md BC-1 | api-contracts.md §1 | @routing, @routing-validation, @observability-tooling, @http-client-process | contracts/route-list.schema.json + snapshot |
| BC-2 | Model, QueryBuilder, vector, migrations, factories | FS-M2-01..06 | FR-200..210 | BR-03 | BC-2 + ADR-002 + ADR-006 | domain.md BC-2 + database.md | api-contracts.md §2 | @orm, @query-builder-additions, @upsert-delete, @collection-serialization, @vector-search | fixtures/vector-dim.json, allowlist, csrf-matrix |
| BC-3 | Guard, AuthManager, PreventRequestForgery, Validatable, ErrorBag | FS-M3-01..06 | FR-300..311 | BR-04 | BC-3 | domain.md BC-3 | api-contracts.md §2 | @auth, @csrf-origin, @cache-session-hardening, @attributes, @throttle | jwt-claims.schema.json, fixtures/csrf-matrix, allowlist-corpus |
| BC-4 | Job, QueueRegistry, Store, Lock, Event, Dispatcher, Schedule | FS-M4-01..05 | FR-400..410 | BR-05 | BC-4 | domain.md BC-4 + database.md (jobs/cache) + capacity.md | api-contracts.md §3 | @queue-routing, @queue, @cache-touch, @contracts-expansion, @schedule, @queue-metrics | job-payload.schema.json, cross-nfr stub |
| BC-5 | CLI, generators make:*, Artisan::call, TestCase | FS-M5-01..04 | FR-500..509 | BR-06 | BC-5 + component-inventory.md | domain.md BC-5 | api-contracts.md §4 | @cli, @generators, @testing | stubs/m5-cli-testing |
| BC-6 | AiProvider, Agent/Tool, ShouldBroadcast, StorageManager, JsonApiResource | FS-M6-01..07 | FR-600..612 | BR-07 | BC-6 | domain.md BC-6 + database.md (vector) | api-contracts.md §5 | @broadcast, @storage-readthrough, @jsonapi, @ai-sdk, @ai-agents | jsonapi.schema.json, path-traversal.corpus, model-inspector schemas |

Coverage gates: every FR (000–612) has a TDD contract; every BC has ≥1 BDD @tag; every contract has a `contracts/*.schema.json` or `fixtures/*.json` and a `stubs/*.stub.rs`. No FR without a test tag (prd.md §8 matrix is source of truth; this table mirrors it).
