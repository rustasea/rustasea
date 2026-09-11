# RustaSea — Functional Specification Document (FSD)

> **Status:** Final — P6 Planning Docs Finalization
> **Date:** 2026-09-07 · **Finalized:** 2026-09-07  
> **Parents:** `brd.md` (BR-01 … BR-09) + `prd.md` (FR-000 … FR-612, NFRs)  
> **Research base:** `docs/laravel-13-research.md` (Laravel 13.0.0 + patches thru v13.30.1) + `README.md` §Laravel 13 Feature Map + Goravel v1.18 mapping  
> **Milestone coverage:** M0 – M6. Each feature spec below lists inputs → processing → outputs → errors → NFR linkage → Laravel 13 trace → PRD FR tags.
> **Planning vs as-built:** This document records planning intent, not implementation status. Live status: [`docs/milestones.md`](../../../docs/milestones.md) — the authoritative as-built status source (TASK-003). **File-size exception:** exceeds the 500-line standard; documented in [ADR-0009](../../../docs/adr/ADR-0009-fsd-500-line-exception.md) (stable `path:line` citations preserved).

---

## 1. Document Purpose

FSD decomposes each **product capability** into implementable feature specs with **interfaces, state machines, and error contracts**. It is the handoff to architecture (`design/architecture/*-tech-design.md`) and task decomposition. Each spec is sized so one or two engineering tasks can implement it without further clarification — or it is explicitly flagged `XL → split`.

No code blocks. Interfaces are described in terms of traits/structs/attributes and their behavioral contracts, not file paths.

---

## 2. Conventions

| Convention | Meaning |
|------------|---------|
| `AppState` | Shared application state injected via `axum::extract::State(Arc<AppState>)` — replaces global facades |
| `Job<T>` / `Event<T>` | Generic typed payloads; no `any` |
| `Store` vs `Repository` | `Store` is the low-level driver trait; `Repository` is the ergonomic facade with prefixing/serialization |
| S/M/L/XL | Sizing: S <4h, M 4–8h, L 1–3d, XL >3d (flag for split) |
| `#[attr]` | Proc-macro declarative attribute (Laravel 13 #7 parity) |

---

## 3. Feature Specifications

### 3.1 M0 — Bootstrap & Core (`rustasea-foundation`, `rustasea-config`)

#### FS-M0-01 — Application Boot & Provider Lifecycle — *Must*

- **PRD FRs:** FR-000, FR-003, FR-008 (BR-01) · **Laravel 13:** #20 `Manager::extend` closure binding · **Size:** M
- **Inputs:** `bootstrap/app.rs` provider registry; `config/*.toml` (+ env overlay); CLI signal (`SIGTERM`/`SIGINT`)
- **Processing:**
  - `Application::configure()` collects `providers: Vec<Box<dyn ServiceProvider>>`.
  - `register()` phase: each provider registers bindings into `Container` (no I/O, no cross-provider reads).
  - `boot()` phase: DAG-resolved order; validates no cycles; each provider receives `&AppState`. Deferred providers (e.g., `withScheduling`) not booted until first use.
  - `Runner` lifecycle spawns HTTP, Queue, Schedule runners only after `boot` succeeds.
- **Outputs:** `Arc<AppState>` available to all handlers/middleware; `Application::running()` true; `route:list`/`config:show` introspectable
- **Errors:** `BootError::Cycle { chain }` · `BootError::MissingDependency { provider }` · `ConfigError::Parse { file, line, source }`
- **State diagram (text):** `Idle → Registering → Booting → Running → Draining(shutdown) → Stopped` · `Booting` fails → `Failed` (no retry without reconfigure)
- **NFRs:** NFR-Per-01 (<2s to ready), NFR-Rel-01 (graceful drain), NFR-Usa-02 (diagnostic errors)
- **Dependencies:** None (M0 is root)
- **Edge cases:** duplicate provider name → last registration wins with warning; missing `config/app.toml` falls back to defaults without crash

#### FS-M0-02 — Service Container (`Bind` / `Singleton` / `Instance` / `Make`) — *Must*

- **PRD FRs:** FR-002, FR-006 (BR-01) · **Size:** M
- **Inputs:** Trait object registration (`Bind`, `Singleton(dyn FnOnce() -> T)`, `Instance(T)`); `Make::<T>` resolution request
- **Processing:**
  - `Bind` resolves fresh on each `Make`; `Singleton` caches `Arc<T>` on first `Make`; `Instance` returns provided `Arc<T>` verbatim.
  - `Manager::extend` closures are bound to the manager instance (so `self` inside closure resolves correctly — #20 fix).
  - Nullable-class defaults model as `Make::<Option<T>>` → `None` when unbound (mirrors Laravel 13 `Container::call` change).
- **Outputs:** `Arc<T>` for `Singleton`/`Instance` (pointer-equality stable); fresh `T` for `Bind`
- **Errors:** `ContainerError::NotFound { type_name }` · `ContainerError::AlreadyBound { type_name }` (strict mode)
- **Invariants:** Singleton `Arc::ptr_eq` across calls; resolving inside `register` cannot read not-yet-registered bindings → compile-time guidance via `ServiceProvider` phase flag
- **NFRs:** NFR-Sca-02 (single-crate `Make` works), NFR-Usa-02 (typed errors)
- **Goravel mapping:** `Bind`/`Singleton`/`Instance`/`Make` directly from Goravel; divergences: trait objects + `Arc`, no `reflect`

#### FS-M0-03 — Layered Config (`config/*.toml` + env overlay + `.env`) — *Must*

- **PRD FRs:** FR-001, FR-007 (BR-01) · **Size:** S
- **Inputs:** `config/*.toml` files; process env; `.env` file via `dotenvy`; typed `Config` structs deriving `Deserialize`
- **Processing:** Layer merge: defaults < file < `.env` < process env. Each file parsed with file/line context. Env keys mapped `APP_PORT` → `app.port`. Invalid TOML surfaced with file+line; missing file is non-fatal (defaults remain).
- **Outputs:** `config::App`, `config::Database`, `config::Cache` etc. as typed structs via `AppState::config::<T>()`
- **Errors:** `ConfigError::Parse { file, line, source }` · `ConfigError::Missing { key }` (when required field absent)
- **NFRs:** NFR-Usa-02 (no panics), NFR-Com-02 (DB config drives driver selection)

#### FS-M0-04 — Graceful Shutdown — *Must*

- **PRD FRs:** FR-004 (BR-01) · **Size:** S
- **Inputs:** `SIGTERM`/`SIGINT` via `tokio::signal`; `shutdown_timeout` from `config.app.shutdown_timeout_secs`
- **Processing:** Signal → `Application::shutdown()` → stop accepting new HTTP connections → drain in-flight requests up to timeout → stop schedule ticker → drain queue workers (ack/nack) → exit `0`.
- **Outputs:** Clean exit code `0` on success; `ShutdownTimeout` diagnostic if drain exceeded
- **NFRs:** NFR-Rel-01
- **Edge cases:** timeout expiry forces exit with `1` after logging outstanding task count

---

### 3.2 M1 — Routing & HTTP (`rustasea-router`, `rustasea-http`)

#### FS-M1-01 — HTTP Routing (methods, groups, `resource`) — *Must*

- **PRD FRs:** FR-100, FR-101 (BR-02) · **Laravel 13:** baseline · **Size:** M
- **Inputs:** Route declarations via `Route::get`/`post`/`put`/`delete`/`patch`/`options`/`any` or `#[route(method, path)]`; group `prefix`/`name`/`middleware`; `resource("users", UserController)` macro
- **Processing:** Routes registered into `axum::Router` with `tower` layer stack. Group prefix concatenation with slash normalization. `resource` expands to `index/create/store/show/edit/update/destroy` with conventional names.
- **Outputs:** `Router` mounted at `AppState`; named route table for `route(name)` resolution
- **Errors:** `RouteError::Conflict { method, path }` · `RouteError::InvalidPattern { path }`
- **Edge cases:** trailing-slash normalization; OPTIONS auto-added for `any`; duplicate named route → error at boot

#### FS-M1-02 — Domain-Aware Routing — *Must*

- **PRD FRs:** FR-102 (BR-02) · **Laravel 13:** #19 · **Size:** S
- **Inputs:** Route with `domain("{tenant}.example.com")` and non-domain route `/docs`
- **Processing:** Router is split: domain-matched routes evaluated before non-domain routes. Catch-all `*.example.com/*` never shadows explicit non-domain routes. Host extracted from `Host` header / `Forwarded`.
- **Outputs:** Deterministic handler selection; domain param extraction via `Path`
- **Errors:** `RoutingError::AmbiguousDomain` when two domain routes match same host+path (requires ordering tie-break)
- **Tests:** See `bdd-scenarios.md @routing-validation`

#### FS-M1-03 — Route Introspection (`route:list`) — *Must*

- **PRD FRs:** FR-103, FR-109 (BR-02/BR-06) · **Laravel 13:** #20 · **Size:** S
- **Inputs:** Registered route table + binding field metadata (e.g., `{user:slug}`)
- **Processing:** `cargo rustasea route:list [--json]` serializes each route as `{ method, path, name, middleware[], binding_fields[] }`. Binding fields derived from `{param:field}` syntax.
- **Outputs:** Human table (default) + JSON (with `--json`)
- **NFRs:** NFR-Mai-01 (observability)

#### FS-M1-04 — Middleware Stack (`throttle`, `cors`, etc.) — *Must*

- **PRD FRs:** FR-104, FR-306 · **Size:** M
- **Inputs:** `#[middleware("throttle:60,1")]` or programmatic `Route::middleware(Throttle::per_minute(60).by_ip())`; `Cors::allow_origins(["https://app.example.com"])`
- **Processing:** `tower-http` layers applied per-route and per-group. `Throttle` uses keyed buckets (`by_ip`, `by_user`, `by_key(fn)`). `Cors` emits `Access-Control-Allow-Origin` only for allow-listed origins.
- **Outputs:** `429` with `Retry-After` when throttled; CORS headers when allowed
- **Errors:** `MiddlewareError::Unknown { name }`
- **Edge cases:** `X-Forwarded-For` behind proxy — trusted-proxy list required or IP is `None` and request not throttled by IP

#### FS-M1-05 — Typed Request Extractors & Responses — *Must*

- **PRD FRs:** FR-105, FR-106 (BR-02) · **Size:** S
- **Inputs:** Handler signatures `Json<T>`, `Query<T>`, `Path<T>`, `State<AppState>`; `validator` derive rules; response `Json<T>`/`View<T>`/`Redirect`
- **Processing:** Extractors deserialize via `serde`; validation errors collected into `ErrorBag` and returned as `422` JSON before handler body executes. Responses set correct `Content-Type`.
- **Outputs:** `422` `ErrorBag` JSON on validation failure; `200` typed JSON/view on success
- **NFRs:** NFR-Usa-02 (structured validation errors)

#### FS-M1-06 — HTTP Client (`reqwest` wrapper, `throw`, timeouts, idle timeout) — *Must*

- **PRD FRs:** FR-107, FR-108 (BR-02) · **Laravel 13:** #18 · **Size:** M
- **Inputs:** `Http::get(url).header(k,v).timeout(d).throw(|resp| predicate)`
- **Processing:** `reqwest` under the hood; `throw` callback inspects `Response` and converts to `HttpError` when predicate true. Idle timeout watches inter-byte silence, not total time. `FakeInvokedProcess::stop` / `ensureNotTimedOut` for testing fakes.
- **Outputs:** `Result<Response, HttpError>` with `status`, `body`, `headers`
- **Errors:** `HttpError::Status { code }` · `HttpError::Timeout { kind: Connect|Total|Idle }`
- **Edge cases:** `throw` callback that itself throws → wrapped as `HttpError::ThrowCallback`

---

### 3.3 M2 — ORM & Database (`rustasea-orm`, `rustasea-macros`)

#### FS-M2-01 — Connection & Driver Abstraction — *Must*

- **PRD FRs:** FR-200, FR-205 (BR-03) · **Size:** M
- **Inputs:** `config.database { driver, url, pool { min, max, idle_timeout } }`; `DATABASE_URL` env override
- **Processing:** `deadpool`/`sqlx` pool per driver (Postgres uses `PgPool`, MySQL `MySqlPool`, SQLite `SqlitePool`). Pool size validation. `db.transaction(|tx| async { ... })` for atomic blocks.
- **Outputs:** `Db` handle; `Transaction` with commit/rollback; pool health check endpoint
- **NFRs:** NFR-Sca-01 (100 concurrent), NFR-Com-02

#### FS-M2-02 — `#[derive(Model)]` & Relations — *Must*

- **PRD FRs:** FR-201, FR-202, FR-206 (BR-03) · **Laravel 13:** #13 · **Size:** L
- **Inputs:** `#[derive(Model)] struct User { id: Uuid, name: String, posts: HasMany<Post> }` with attributes `#[table("users")]`, `#[soft_delete]`, `#[cast]`
- **Processing:** Macro generates `id`, timestamps, soft-delete `deleted_at`, table name (`snake_plural` default), relation helpers `with("posts")`, and `serde` round-trip that serializes eager-loaded relations under `relations` key and restores on `deserialize`.
- **Outputs:** Compile-time `Model` impl with `table_name()`, `query()`, `relations()` metadata
- **Errors:** `ModelError::MissingPrimaryKey` (compile error via macro) · `ModelError::RelationNotFound { name }`
- **Edge cases:** composite keys deferred; relation cycle (`User → Post → User`) depth-limited to 3 to prevent infinite serialize

#### FS-M2-03 — Query Builder (fluent, paginate, locks, scopes, raw) — *Must*

- **PRD FRs:** FR-202, FR-203, FR-205, FR-210 (BR-03) · **Laravel 13:** #15 · **Size:** L
- **Inputs:** Builder chain `Model::query().where("status","active").orWhere(...).orderBy(...).paginate(15)`
- **Processing:** Methods: `where`/`orWhere`/`whereJsonContains`/`whereJson`/`find`/`first`/`firstOrFail`/`create`/`save`/`update`/`delete`/`forceDelete`/`paginate`/`cursor`/`chunkBy`/`orWhereKey`/`orWhereKeyNot`/`whereBinary`/`StraightJoin`/`insertOrIgnoreReturning`/`saveOrIgnore`/`refreshForUpdate`/`toSql`/`toRawSql`/`forUpdate`/`sharedLock`/`scope`/`raw`. `chunkBy` paginates by indexed cursor key to avoid offset OOM.
- **Outputs:** Typed `Vec<T>` / `Paginated<T>` / `CursorPage<T>` / `Option<T>`
- **Errors:** `QueryError::NotFound` (`firstOrFail`) · `QueryError::InvalidCursor`
- **NFRs:** NFR-Rel-02

#### FS-M2-04 — Upsert & Delete Strictness — *Must*

- **PRD FRs:** FR-204 (BR-03) · **Laravel 13:** #14 · **Size:** S
- **Inputs:** `upsert(rows, unique_by: &[&str], update: &[&str])`
- **Processing:** When `unique_by.is_empty()` returns `Err(UpsertError::EmptyUniqueBy)` — never silently no-op. MySQL `DELETE … JOIN … ORDER BY/LIMIT` compiled; previously-silent ignore cases now throw `DeleteError::Unsupported`.
- **Outputs:** `UpsertResult { inserted, updated }`
- **Edge cases:** empty `rows` → `Ok({0,0})`; `unique_by` referencing non-unique column → DB error surfaced verbatim

#### FS-M2-05 — Migrations & Seeders — *Must*

- **PRD FRs:** FR-208 (BR-03) · **Size:** M
- **Inputs:** `cargo rustasea make:migration create_users_table` scaffold; `migrate`/`migrate:fresh`/`migrate:fresh --seed` args
- **Processing:** Versioned files `YYYY_MM_DD_HHMMSS_name.rs` with `up`/`down`. `migrate` runs pending ups in order, records in `migrations` table. `migrate:fresh` drops all then re-ups. Seeders run via `Seeder::run(&mut conn)` idempotently.
- **Outputs:** Migration table state; `migrate:status` introspection
- **Errors:** `MigrationError::AlreadyApplied` · `MigrationError::Irreversible { name }`
- **NFRs:** NFR-Rel-02 (idempotence)

#### FS-M2-06 — Factories & Vector Extension — *Must* (vector is M2-initial; full in M6)

- **PRD FRs:** FR-207, FR-209 (BR-03/BR-07) · **Laravel 13:** #6 + #20 · **Size:** M
- **Inputs:** `UserFactory::create(5)`; `whereVectorSimilarTo("embedding", &vec, limit)`; `Blueprint::vector("embedding", 1536)`
- **Processing:** Factories derive `Factory` impl with `definition() -> T` + `sequence` + `state`. `Str` factory sequences reset per test via `TestCase` hook. `vector` column maps to `pgvector` `vector(1536)`; `whereVectorSimilarTo` emits `ORDER BY embedding <=> $1 LIMIT $2` (cosine). Missing `pgvector` extension → migration fast-fails with `PgVectorError::ExtensionMissing`.
- **Outputs:** Factory-created rows; nearest-neighbor rows
- **Edge cases:** dimension mismatch (1536 vs 768) → DB error `VectorDimensionMismatch`; MariaDB behind `mariadb-vector` feature flag

---

### 3.4 M3 — Auth, Middleware & Validation (`rustasea-auth`, `rustasea-validation`)

#### FS-M3-01 — Guards (JWT + Session, `Auth::extend`) — *Must*

- **PRD FRs:** FR-300, FR-301 (BR-04) · **Size:** M
- **Inputs:** `Auth::guard("jwt")` / `Auth::guard("session")`; credentials `{ email, password }`; `Auth::extend("custom", |app| MyGuard::new(...))`
- **Processing:** JWT via `jsonwebtoken` (HS256, `argon2` password verify). `login` returns `Token { access, refresh }`; `parse(token)` → `Claims { sub, exp }`; `refresh` rotates. Session via `tower-sessions` store keyed by cookie. `Auth::extend` registers custom `Guard` impl at boot.
- **Outputs:** `AuthUser { id, email, guard }`
- **Errors:** `Error::GuardMismatch { expected, actual }` · `Error::InvalidToken` · `Error::ExpiredToken` · `Error::BadCredentials`

#### FS-M3-02 — Origin-Aware CSRF (`PreventRequestForgery` + `Sec-Fetch-Site`) — *Must*

- **PRD FRs:** FR-302 (BR-04) · **Laravel 13:** #11 · **Size:** S
- **Inputs:** `POST`/`PUT`/`DELETE`/`PATCH` request with `X-CSRF-TOKEN` header or `_token` field + `Sec-Fetch-Site` header
- **Processing:** Token verified first; then `Sec-Fetch-Site: cross-site` triggers origin allow-list check (`allowed_origins` from `config.app.csrf_origins`). Missing `Sec-Fetch-Site` (older browsers) degrades to token-only. `GET`/`HEAD`/`OPTIONS` exempt.
- **Outputs:** `403 CsrfError::UntrustedOrigin` or pass
- **NFRs:** NFR-Sec-01
- **Edge cases:** `Sec-Fetch-Site: none` (direct navigation) treated as same-origin

#### FS-M3-03 — Session & Cache Hardening — *Must*

- **PRD FRs:** FR-303, FR-304 (BR-04) · **Laravel 13:** #12 · **Size:** S
- **Inputs:** `Cache::put(k, v)` / `Session::put(k, v)` where `v: Serialize`
- **Processing:** Serialization gated to JSON by default (`session.serialization = "json"`). Deserialization checks `serializable_classes` allow-list before `from_str`. Prefixes hyphenated (`-cache-`, `-session-`).
- **Outputs:** Stored JSON entries with prefixed keys
- **Errors:** `SerializationError::NotAllowed { type_name }`
- **NFRs:** NFR-Sec-02

#### FS-M3-04 — Rate Limiter (`limit.perMinute().by(ip)`) — *Must*

- **PRD FRs:** FR-306 (BR-04) · **Size:** S
- **Inputs:** `limit.per_minute(60).by_ip() | .by_user() | .by_key(|req| req.ip().to_string())`
- **Processing:** `Throttle` middleware with in-memory or Redis bucket. `429` with `Retry-After` seconds to window reset. Behind proxy requires `trusted_proxies`.
- **Outputs:** Throttled or passed request
- **NFRs:** NFR-Mai-01 (metrics log throttle hits)

#### FS-M3-05 — Validation & `ErrorBag` (`#[validate]`, strict rules) — *Must*

- **PRD FRs:** FR-307, FR-308, FR-309 (BR-04) · **Laravel 13:** #19 · **Size:** M
- **Inputs:** `#[validate] struct CreateUser { #[validate(length(min=3))] name: String, #[validate(email)] email: String, #[validate(contains_strict = "admin")] role: String }`
- **Processing:** `validator` derive + `rustasea-validation` strict helpers. `in_array`/`contains`/`doesnt_contain` compare with `==` and type check — `"1" != 1`. Failures aggregate into `ErrorBag { field -> Vec<ValidationError> }`. `FormRequest`-equivalent trait `Validatable`.
- **Outputs:** `422` with `ErrorBag` JSON on failure; `Ok(T)` into handler on success
- **Errors:** `ValidationError::StrictMismatch { expected, actual }`
- **Edge cases:** `null` vs missing field — `Option<T>` distinction preserved

#### FS-M3-06 — Declarative Attributes (`#[middleware]`, `#[authorize]`) — *Must*

- **PRD FRs:** FR-305 (BR-04/BR-06) · **Laravel 13:** #7 · **Size:** S
- **Inputs:** `#[middleware("auth:jwt", "throttle:60,1")]` on handler or controller impl; `#[authorize("update", User)]`
- **Processing:** Proc-macro expands to `tower::Layer` wiring at router build time and `Authorize` call before handler body.
- **Outputs:** Route registered with middleware chain; authorization gate evaluated

---

### 3.5 M4 — Queue, Cache, Scheduling & Events (`rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule`)

#### FS-M4-01 — Typed Job System & Retry (`Job<T>`, `ShouldRetry`, `#[tries]` etc.) — *Must*

- **PRD FRs:** FR-400 (BR-05) · **Laravel 13:** #7 · **Size:** M
- **Inputs:** `#[tries(3)] #[backoff(10)] #[timeout(30)] struct SendEmail { to: String, body: String } impl Job for SendEmail { async fn handle(self) -> Result<()> }`
- **Processing:** Job payload serialized via `serde_json` into `jobs` table or `deadpool-redis` list. Worker `handle` called; `ShouldRetry` / `ShouldRetryUntil` traits control retry; `#[tries]`/`#[backoff]`/`#[timeout]` attributes configure retries; `#[failOnTimeout]` forces failure. Backoff is exponential if `#[backoff]` is scalar, fixed if array.
- **Outputs:** Job `Succeeded` / `Failed` / `Retrying { attempt, backoff }` state
- **Errors:** `JobError::Timeout` · `JobError::MaxAttemptsExceeded`
- **State (text):** `Pending → Reserved → Processing → Succeeded | Failed → Retrying → Pending (loop) → DeadLetter(failed_jobs)`

#### FS-M4-02 — Queue Routing (`Queue::route`) & Drivers — *Must*

- **PRD FRs:** FR-401, FR-402 (BR-05) · **Laravel 13:** #4 + #8 · **Size:** M
- **Inputs:** `Queue::route::<ProcessPodcast>(connection: "redis", queue: "podcasts")` at boot; `ProcessPodcast::dispatch(payload).onQueue("urgent").delay(60s)`
- **Processing:** Routing registry is `OnceLock<HashMap<TypeId, Route>>` after `boot` — no races. Per-dispatch `onQueue`/`onConnection` override registry. Drivers: `sync` (immediate), `database` (polls `jobs` table), `redis` (BRPOP). `chain([JobA, JobB])` executes sequentially; batch aggregates. `failed_jobs` persisted with `exception`, `failed_at`.
- **Outputs:** Job enqueued to correct `connection`/`queue`; `queue:failed` lists, `queue:retry {id}` re-queues
- **Errors:** `QueueError::DuplicateRoute { type_name }` · `QueueError::UnknownConnection`
- **Edge cases:** routing for not-yet-registered job type → `NotFound` until provider registers

#### FS-M4-03 — Dispatcher & Cloud Metrics — *Must*

- **PRD FRs:** FR-406 (events) + FR-409 (metrics) (BR-05) · **Laravel 13:** #16 + #8 · **Size:** S
- **Inputs:** `Dispatcher::dispatch(event)`; `Dispatcher::dispatchAfterResponse(event)`; `Queue::pendingSize(conn, queue)`
- **Processing:** `dispatchAfterResponse` buffers until HTTP response sent, then flushes. Metrics `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` read from driver backend (`LLEN`/`ZRANGE` for redis, `SELECT COUNT` for database). `JobAttempted { exception }` and `QueueBusy { connectionName }` events have renamed fields vs Laravel 12.
- **Outputs:** After-response event delivery; metric integers + RFC3339 timestamp

#### FS-M4-04 — Cache (`Store`/`Repository`, `touch`, `Lock`, Hardening) — *Must*

- **PRD FRs:** FR-403, FR-404, FR-405 (BR-05) · **Laravel 13:** #5 + #12 · **Size:** M
- **Inputs:** `Cache::store("redis").put("k","v", 60s)`; `Cache::touch("k", 120s)`; `Cache::lock("billing", 10s).get()`
- **Processing:** `Store` trait requires `get`/`put`/`forget`/`flush`/`increment`/`decrement` + `touch`. `Repository` adds `remember`/`forever`/`pull`/`has`/`withContext`. `touch` translates to `EXPIRE` (redis) / TTL reset (moka) without value fetch. `Lock` uses Redis `SET NX EX` or `moka` entry guard. JSON serialization default with hyphenated prefix from M3 applies to cached values.
- **Outputs:** Cached value `Option<V>`; `touch` `bool` (false if key missing); `LockGuard` RAII
- **Errors:** `CacheError::StoreUnavailable` · `LockError::AlreadyHeld`
- **NFRs:** NFR-Per-03, NFR-Sec-02
- **Edge cases:** `touch` on missing key → `false` (not error); driver that doesn't implement `touch` must return `Unsupported` at compile via trait bound

#### FS-M4-05 — Scheduler & Pause/Resume — *Must*

- **PRD FRs:** FR-407, FR-408, FR-410 (BR-05) · **Laravel 13:** #10 · **Size:** M
- **Inputs:** `Schedule::command("emails:send").daily().at("08:00").skipIfStillRunning().onOneServer()`; CLI `schedule:pause`/`schedule:resume`/`schedule:list`/`schedule:run`
- **Processing:** `tokio-cron-scheduler` / `cron` parsing; `schedule:run` tick loop evaluated each minute. `onOneServer` acquires distributed `Cache::lock`. `pause` sets `schedule_paused` flag in cache/DB; tick loop checks flag before dispatching. Emits `SchedulePaused`/`ScheduleResumed` events. `withScheduling` deferred until first `schedule:run` tick.
- **Outputs:** Scheduled jobs dispatched; `pause` stops dispatch, `resume` restarts
- **State (text):** `Running ↔ Paused`; `SkippedIfStillRunning` suppresses tick; `OnOneServer` suppresses on other nodes
- **Edge cases:** pause while job mid-execution — running job completes, next tick suppressed; resume idempotent

---

### 3.6 M5 — DX, CLI & Testing (`rustasea-cli`, `rustasea-macros`, `rustasea-testing`)

#### FS-M5-01 — CLI (`cargo rustasea`, `list`, `clap`+`xtask`) — *Must*

- **PRD FRs:** FR-500, FR-502, FR-503, FR-505 (BR-06) · **Size:** M
- **Inputs:** `cargo rustasea <command> [args] [--json]` via `xtask` binary `cargo-xtask`; `#[command]` proc-macro on `struct SendEmailsCommand`
- **Processing:** `clap` derive for each `Command` with typed `Args`/`Flags`; `list` enumerates registered commands with `#[usage]`/`#[help]`/`#[hidden]`. Prompts `ask`/`secret`/`confirm`/`choice`/`multiSelect` via `dialoguer`; `table`/`progressBar`/`spinner` via `indicatif`/`comfy-table`. `Artisan::call("migrate", args)` invokes command in-process without subprocess.
- **Outputs:** Exit code `0` on success; `--json` machine output; `Artisan::call` returns `CommandOutput`
- **Edge cases:** unknown command suggests `did you mean?` via `strsim`

#### FS-M5-02 — `make:*` Generators — *Must*

- **PRD FRs:** FR-501, FR-502, FR-506 (BR-06) · **Laravel 13:** #7 attrs + #2 agents · **Size:** L (XL split: by generator group)
- **Inputs:** `cargo rustasea make:controller UserController --resource`; `make:model Post -m`; `make:agent SupportAgent`
- **Processing (per generator):**

  | Generator | Output path | Template content |
  |-----------|-------------|------------------|
  | `make:controller` | `app/http/controllers/{name}.rs` | Controller struct with `index`/`store`/`show`/`update`/`destroy` if `--resource` |
  | `make:model` | `app/models/{name}.rs` | `#[derive(Model)]` struct + `Factory` + migration if `-m` |
  | `make:provider` | `app/providers/{name}.rs` | `ServiceProvider` impl with `register`/`boot` |
  | `make:command` | `app/console/commands/{name}.rs` | `Command` with `signature` + `handle` + `#[usage]` |
  | `make:job` | `app/jobs/{name}.rs` | `Job` impl + `#[tries]` scaffold |
  | `make:event` | `app/events/{name}.rs` | `Event` struct |
  | `make:listener` | `app/listeners/{name}.rs` | `Listener` with `Queue { enable }` flag |
  | `make:observer` | `app/observers/{name}.rs` | Observer on model lifecycle |
  | `make:test` | `tests/feature/{name}_test.rs` | `TestCase` harness + factory usage |
  | `make:seeder` | `database/seeders/{name}.rs` | `Seeder::run` |
  | `make:agent` | `app/ai/agents/{name}.rs` | `Agent` scaffold (M6) |
  | `make:tool` | `app/ai/tools/{name}.rs` | `Tool` impl scaffold (M6) |
- **Outputs:** Generated `.rs` files that are `rustfmt` + `clippy -D warnings` clean
- **NFRs:** NFR-Usa-03, NFR-Per-04
- **Errors:** `GeneratorError::AlreadyExists { path }` without `--force`
- **Split note:** `L` overall; decomposes into `make:crud` (controller/model) vs `make:async` (job/event/listener) vs `make:ai` (agent/tool)

#### FS-M5-03 — Declarative Attributes Bundle — *Must*

- **PRD FRs:** FR-506 (BR-06) · **Laravel 13:** #7 · **Size:** M
- **Inputs:** Attributes on jobs/commands/handlers: `#[tries(3)]`, `#[backoff(10)]`, `#[timeout(30)]`, `#[failOnTimeout]`, `#[withoutBroadcasting]`, `#[middleware(...)]`, `#[authorize(...)]`, `#[usage("...")]`, `#[help("...")]`, `#[hidden]`, `#[repairToolCalls]`
- **Processing:** `rustasea-macros` proc-macros expand to trait impls / registry entries at compile time; no reflection.
- **Outputs:** Compile-time configured behavior for decorated items
- **Edge cases:** conflicting `#[tries]` + `ShouldRetryUntil` explicit impl → attribute wins with warning

#### FS-M5-04 — Test Harness (`TestCase`, `testcontainers`, `.env.testing`) — *Must*

- **PRD FRs:** FR-507, FR-508, FR-509 (BR-06) · **Laravel 13:** #20 · **Size:** M
- **Inputs:** `struct MyTest: TestCase { fn setup(&mut self) -> AppState }`; per-package `.env.testing`; `Factory::create` calls
- **Processing:** `TestCase` trait sets up isolated Postgres (random port via `testcontainers` `Postgres` image) + Redis; runs `migrate` once per test binary; resets `Str` factory sequences between tests; loads `.env.testing` overlay. Paginator views registered for `bootstrap-3` style. Teardown kills containers.
- **Outputs:** Isolated `AppState` per test; `cargo test` pass with parallel isolation
- **NFRs:** NFR-Rel-03
- **Edge cases:** container startup timeout (30s) → `TestError::ContainerTimeout`; port collision handled by random allocation

---

### 3.7 M6 — Advanced (`rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai`)

#### FS-M6-01 — Broadcasting (WebSocket + SSE + Channel Auth) — *Must*

- **PRD FRs:** FR-600, FR-601, FR-610 (BR-07) · **Laravel 13:** #16 `eventStream` · **Size:** M
- **Inputs:** `impl ShouldBroadcast for UserCreated { fn broadcastOn(&self) -> Channel { Channel::Private("chat.1") } }`; WebSocket client `ws://host/broadcasting/auth`; SSE `Response::eventStream(stream)`
- **Processing:** WebSocket via `axum::extract::ws` + `tokio-tungstenite`; channel auth checks `Authorize` gate; SSE sets `Content-Type: text/event-stream` and streams `event:` chunks. `ShouldBroadcast` serialized via `serde`.
- **Outputs:** Authenticated channel subscription; streamed events (`event: token`, `data: {...}`)
- **Errors:** `BroadcastError::Unauthorized { channel }`
- **Edge cases:** slow consumer backpressure via `tokio::sync::mpsc` bounded channel — overflow `Lagged`

#### FS-M6-02 — Storage Read-Through + Path Confinement — *Must*

- **PRD FRs:** FR-603, FR-611 (BR-07) · **Laravel 13:** #9 · **Size:** M
- **Inputs:** `Storage::disk("s3")` primary + `fallback: "local"` config; `Storage::get("a/b.txt")`; `Storage::path("a/b.txt")`
- **Processing:** `read` tries primary; on `NotFound` falls through to fallback; optional `copy_back: true` writes to primary. `path()` canonicalizes and verifies result is under disk root — `../` traversal returns `StorageError::PathTraversal`. Disks backed by `object_store` (S3/GCS/Azure) or `tokio::fs` (local).
- **Outputs:** File bytes or `NotFound`; canonical absolute path under root
- **NFRs:** NFR-Sec-03 (fuzz `..` payloads)
- **Edge cases:** fallback also `NotFound` → surfaced as `NotFound`; `path` on S3 disk returns virtual path (not filesystem path)

#### FS-M6-03 — JSON:API Resources — *Must*

- **PRD FRs:** FR-604 (BR-07) · **Laravel 13:** #3 · **Size:** M
- **Inputs:** `UserResource::new(user).include("posts").fields(["name","email"])` with model relations
- **Processing:** `JsonApiResource` trait serializes `{ data: { type, id, attributes, relationships, links }, included: [...], links, meta }`. Sparse fieldsets filter `attributes`; `include=posts` eager-loads relations; headers set `Content-Type: application/vnd.api+json`.
- **Outputs:** JSON:API document with correct content-type
- **Edge cases:** include for not-loaded relation → `JsonApiError::RelationNotLoaded` (requires `with("posts")`)

#### FS-M6-04 — Queued Notifications (`#[deleteWhenMissingModels]`) — *Should*

- **PRD FRs:** FR-605 (BR-07) · **Laravel 13:** #17 · **Size:** S
- **Inputs:** `#[deleteWhenMissingModels] struct WelcomeNotification { user: User }`
- **Processing:** Before queue send, checks `user.exists()`; if soft-deleted/missing, job skipped not retried.
- **Outputs:** Skipped job with `NotificationSkipped { reason: MissingModel }` log

#### FS-M6-05 — AI SDK (Provider-Agnostic Trait, 12 Providers) — *Must* (feature-flagged)

- **PRD FRs:** FR-606 (BR-07) · **Laravel 13:** #1 · **Size:** L
- **Inputs:** `Ai::provider("anthropic").text(prompt).stream(true).tools([SearchDocs])`
- **Processing:** Trait `AiProvider` covers `text`/`image`/`audio`/`embeddings`/`reranking`/`files`/`vector_stores`. Adapters per provider: `openai`, `anthropic`, `gemini`, `azure`, `bedrock`, `groq`, `xai`, `deepseek`, `mistral`, `ollama`, `openrouter`, `openai_compatible`. Each adapter feature-flagged (`features = ["openai"]`). Errors typed per provider but surfaced as `AiError::Provider { name, source }`.
- **Outputs:** `AiResponse { text, usage, tool_calls }` or streaming `AiChunk` iterable
- **Errors:** `AiError::UnsupportedCapability { provider, capability }`
- **NFRs:** NFR-Sca-02 (`rustasea-ai` opt-in; core does not pull AI deps)
- **Edge cases:** streaming with provider that doesn't support streaming → `UnsupportedCapability`

#### FS-M6-06 — AI Agents (Tools, Streaming, Broadcast, Queue, MCP, Sub-Agents) — *Must*

- **PRD FRs:** FR-607, FR-608, FR-609, FR-610 (BR-07) · **Laravel 13:** #2 · **Size:** XL → split
- **Inputs:** `#[derive(Agent)] struct SupportAgent { tools: [ToolSearch], middleware: [Logging], sub_agents: [KnowledgeAgent] }`
- **Processing:** `Agent` contract: `prompt: String -> Stream<AiChunk>`. Tools are structs implementing `Tool { name, schema: JsonSchema, call(args: Json) -> Json }`. Deferred loaders `SimilaritySearch`/`FileStorage`/`ToolSearch` inject context on demand. Anonymous agents via closure `Ai::agent(|a| a.tool(MyTool))`. Middleware wraps `prompt -> next -> output`. Sub-agents invoked as tools. Broadcasting streams chunks over WebSocket; queueing enqueues tool calls as Jobs. MCP discovery via `mcp` feature flag registers MCP-provided tools.
- **Outputs:** Streamed response chunks (`event: token`), structured output JSON, queued tool job IDs
- **Errors:** `AgentError::ToolNotFound` · `AgentError::McpUnavailable` (when MCP flag off)
- **Split note:** `XL` — split into FS-M6-06a (Agent+Tool core), FS-M6-06b (streaming/broadcast/queue), FS-M6-06c (MCP + sub-agents + deferred loaders)

#### FS-M6-07 — Vector Search Full (`Str::toEmbeddings`, `dropVectorIndex`, Similarity) — *Must*

- **PRD FRs:** FR-602 (BR-07) · **Laravel 13:** #6 full · **Size:** S (builds on FS-M2-06)
- **Inputs:** `Str::toEmbeddings("hello", provider: "openai")`; `Schema::table("docs", |t| t.dropVectorIndex("embedding"))`
- **Processing:** `toEmbeddings` calls `AiProvider::embeddings(text)`. `dropVectorIndex` emits `DROP INDEX` for `pgvector` ivfflat/hnsw. M6 extends FS-M2-06's initial vector support with indexing strategies and embedding-provider integration.
- **Outputs:** Embedding `Vec<f32>`; index dropped
- **Edge cases:** embedding dim mismatch with column type → `VectorDimensionMismatch`

---

## 4. Cross-Cutting Specifications

### 4.1 Error Handling

| Principle | Detail |
|-----------|--------|
| Typed errors | Every feature returns `thiserror`-derived typed enums with `code` + `hint` + `source` chain; no bare `unwrap` in framework crates (`#[deny(clippy::unwrap_used)]`) |
| Contract evolution | New trait methods (e.g., `touch`, `dispatchAfterResponse`) are additive with default impl returning `Unsupported` to avoid breaking custom drivers |
| Serialization errors | All serialization paths use JSON by default; deserialization allow-list enforced before `serde` instantiation |

### 4.2 Security Cross-Cutting

| Concern | Features Involved | Control |
|---------|-------------------|---------|
| CSRF `Sec-Fetch-Site` | FS-M3-02 | Origin allow-list check layered on token |
| Path traversal | FS-M6-02 | `Storage::path()` canonicalization confinement |
| Allow-list deserialization | FS-M3-03, FS-M4-04 | `serializable_classes` check before `deserialize` |
| `argon2` hashing | FS-M3-01 | Per-password salt, constant-time verify |

### 4.3 Observability Cross-Cutting

| Signal | Feature | Mechanism |
|--------|---------|-----------|
| `route:list` + binding fields | FS-M1-03 | Route table introspection |
| `show:model` / `ModelInspector` | FS-M1-03 | Model metadata object |
| Queue metrics | FS-M4-03 | `Queue` trait metrics (pending/delayed/reserved/oldest) |
| Schedule events | FS-M4-05 | `SchedulePaused`/`ScheduleResumed` events |
| Throttle/Rate-limit logs | FS-M3-04 | Metrics emission on `429` |

---

## 5. Dependency Graph (Features → Prerequisites)

```text
M0: FS-M0-01..04 (no prereq)
  → M1: FS-M1-01 (→FS-M1-02..06)
  → M2: FS-M2-01..06 (requires FS-M0-03 for DB config)
  → M3: FS-M3-01..06 (requires FS-M1-04/05, FS-M2-01)
  → M4: FS-M4-01..05 (requires FS-M0-02 container, FS-M2-01, FS-M3-03)
  → M5: FS-M5-01..04 (requires FS-M0-01 + all M1–M4 registries)
  → M6: FS-M6-01..07 (requires FS-M1-06, FS-M2-06, FS-M4-01..03, FS-M5-01..02)
```

No cycles. M5 aggregates all prior milestones because CLI generators scaffold across domains. M6 builds on nearly every prior feature but is feature-flagged so core ships without it.

---

## 6. Traceability Summary

Each FS traces to PRD FR(s) and a Laravel 13 feature #. Full matrix is in `prd.md` §8; this FSD mirrors it per-feature. Gherkin tags are in `bdd-scenarios.md` per `@tag`.

| FSD Feature | PRD FRs | Laravel 13 # | Milestone |
|-------------|---------|--------------|-----------|
| FS-M0-01 | FR-000/003/008 | #20 | M0 |
| FS-M0-02 | FR-002/006 | — (container) | M0 |
| FS-M0-03 | FR-001/007 | — | M0 |
| FS-M0-04 | FR-004 | — | M0 |
| FS-M1-01..06 | FR-100..109 | #18/#19/#20 | M1 |
| FS-M2-01..06 | FR-200..210 | #6/#13/#14/#15 | M2 |
| FS-M3-01..06 | FR-300..311 | #11/#12/#19/#7 | M3 |
| FS-M4-01..05 | FR-400..410 | #4/#5/#8/#10/#16 | M4 |
| FS-M5-01..04 | FR-500..509 | #7/#20 | M5 |
| FS-M6-01..07 | FR-600..612 | #1/#2/#3/#9/#16/#17 | M6 |

> **Sizing roll-up:** S×≈12, M×≈14, L×≈4, XL×1 (FS-M6-06 → split into 3). Consistent with `prd.md` §7 cost estimate (26–36 dev-weeks).

---

## 7. Out-of-Scope Reaffirmation

Same as `brd.md` §4 + `prd.md` §1. Any out-of-scope item requires an ADR to promote.

---

## 8. Traceability — Requirements → Architecture → Tests (P6)

| FS | PRD FRs | Architecture (crate / design doc) | Tests (BDD tag → QA artifacts) |
|----|---------|-----------------------------------|--------------------------------|
| FS-M0-01 | FR-000/003/008 | `rustasea-foundation`: `architecture.md §2-3` (BC-0, DAG, `Application::configure`) · `tdd.md BC-0` | `@foundation` · `application/testing/stubs/m0-foundation.stub.rs` + `test-plan.md` BC-0 |
| FS-M0-02 | FR-002/006 | `rustasea-foundation::Container` (part of foundation, not a separate crate; see the [canonical crate inventory](../application/modules/manifest.md#canonical-crate-inventory-source-of-truth)) / `architecture.md §3` + `tdd.md BC-0` + [ADR-0007 AppState](../../../docs/adr/ADR-0007-appstate-over-facades.md) | `@container` · same M0 stub |
| FS-M0-03 | FR-001/007 | `rustasea-config` · `architecture.md §5` (layered config) | `@foundation` (layered config scenario) |
| FS-M0-04 | FR-004 | `rustasea-foundation` Runner/Shutdown · `capacity.md §2 S-08` | `@foundation` graceful shutdown |
| FS-M1-01..06 | FR-100..109 | `rustasea-router`/`rustasea-http`/`rustasea-macros` · `architecture.md BC-1` · `tdd.md BC-1` · [ADR-0003 axum](../../../docs/adr/ADR-0003-axum-vs-actix.md) · `api-contracts.md §1` | `@routing`, `@routing-validation`, `@observability-tooling`, `@http-client-process` · `contracts/route-list.schema.json` + snapshot |
| FS-M2-01..06 | FR-200..210 | `rustasea-orm` · `architecture.md BC-2` · `tdd.md BC-2` · `database.md §2-5` · [ADR-0004 sqlx/sea-orm](../../../docs/adr/ADR-0004-sqlx-vs-sea-orm.md) + [ADR-0008 vector](../../../docs/adr/ADR-0008-vector-feature-flag.md) | `@orm`, `@query-builder-additions`, `@upsert-delete`, `@collection-serialization`, `@vector-search` · `fixtures/vector-dim.json` + `m2-orm.stub.rs` |
| FS-M3-01..06 | FR-300..311 | `rustasea-auth`/`rustasea-validation` · `architecture.md BC-3` · `tdd.md BC-3` · `api-contracts.md §2` · `fixtures/csrf-matrix.json` + `allowlist-corpus.json` + `jwt-claims.schema.json` | `@auth`, `@csrf-origin`, `@cache-session-hardening`, `@attributes`, `@throttle` |
| FS-M4-01..05 | FR-400..410 | `rustasea-queue`/`rustasea-cache`/`rustasea-events`/`rustasea-schedule` · `architecture.md BC-4` · `tdd.md BC-4` · `database.md §2 (jobs/failed_jobs/cache)` · `api-contracts.md §3` · `capacity.md §3` · `contracts/job-payload.schema.json` | `@queue-routing`, `@queue`, `@cache-touch`, `@contracts-expansion`, `@schedule`, `@queue-metrics` + `job-payload` snapshot |
| FS-M5-01..04 | FR-500..509 | `rustasea-cli`/`rustasea-testing` · `architecture.md BC-5` · `component-inventory.md` · `tdd.md BC-5` · `api-contracts.md §4` | `@cli`, `@generators`, `@testing` · full CLI matrix in `test-cases.md` |
| FS-M6-01..07 | FR-600..612 | `rustasea-broadcast`/`rustasea-storage`/`rustasea-search`/`rustasea-ai` · `architecture.md BC-6` · `tdd.md BC-6` · `api-contracts.md §5` · `database.md §2 (vector/storage)` · `fixtures/path-traversal.corpus.json` + `contracts/jsonapi.schema.json` + snapshots | `@broadcast`, `@storage-readthrough`, `@jsonapi`, `@ai-sdk`, `@ai-agents`, `@vector-search` · `cross-nfr-contracts.stub.rs` |

*P6 traceability — every FS → FR → crate/ADR/schema/fixture/BDD tag is enumerated; gaps close via `manifest.md` module index and `qa-design.md`/`test-plan.md`.*

---

*Finalized P6 · `user-stories.md` (stories per FS with EARS AC) and `bdd-scenarios.md` (Gherkin per @tag). Tests MUST follow `test-generation/rules/bdd-gherkin.md` (business language, atomic scenarios, Scenario Outline + Examples, no technical terms).*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
