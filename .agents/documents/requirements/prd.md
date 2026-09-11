# RustaSea — Product Requirements Document (PRD)

> **Status:** Final — P6 Planning Docs Finalization
> **Date:** 2026-09-07 · **Finalized:** 2026-09-07  
> **Parent:** `brd.md` (BR-01 … BR-09)  
> **Research base:** `docs/laravel-13-research.md` (Laravel 13.0.0 2026-03-17; patches thru v13.30.1) + `README.md` §Laravel 13 Feature Map  
> **Coverage:** 20 Laravel 13 features → FR mapping; milestones M0–M6; MoSCoW priority
> **Planning vs as-built:** This document records planning intent, not implementation status. Live status: [`docs/milestones.md`](../../../docs/milestones.md) — the authoritative as-built status source (TASK-003).

---

## 1. Purpose and Scope

This PRD translates the BRD's business requirements into **numbered, testable functional requirements (FR)** and **non-functional requirements (NFR)** per milestone. Every FR is sourced to a stakeholder need or Laravel 13 feature, carries a MoSCoW priority, and traces to a BDD scenario (see `bdd-scenarios.md`) and a user story (see `user-stories.md`).

**Out-of-scope reaffirmed:** Filament/Nova admin, hosting PaaS, full Blade runtime parity, PHP bridge (see `brd.md` §4).

---

## 2. Definitions

| Term | Meaning |
|------|---------|
| `AppState` | `Arc`-shared application state injected via `axum::extract::State`, replacing Goravel/Laravel global facades |
| `Job<T>` | Typed queue job with generic payload `T`; `handle(self)` is the execution entrypoint |
| `Store` | Cache `Store` trait (analogous to Laravel `Cache\Store`) |
| `touch` | Cache TTL extension without re-reading value (Laravel 13 #5) |
| `route:list` | CLI introspection of registered routes with middleware + binding fields (#20) |
| `whereVectorSimilarTo` | Vector similarity query over `pgvector` / MariaDB vectors (#6) |
| `JsonApiResource` | JSON:API spec resource object with sparse fieldsets, inclusion, links, headers (#3) |

---

## 3. Functional Requirements by Milestone

### M0 — Bootstrap & Core (depends on: —)

> Goal: bootable skeleton with layered config, container, providers, graceful shutdown.

| # | Requirement | Source | Priority | Laravel 13 Feature | Acceptance Hint (EARS) |
|---|-------------|--------|----------|--------------------|------------------------|
| FR-000 | The system **shall** provide a `foundation::Application` that bootstraps via `Application::configure()` with ordered service providers. | BR-01 · SH-01 | Must | Manager `extend` closure binding semantics — closures bound to manager (#20) | Given `bootstrap/app.rs` registers providers, When `cargo run` executes, Then app boots and providers fire `register`→`boot` in DAG order |
| FR-001 | The system **shall** load layered config from `config/*.toml` (or `.yaml`) with env overlay and `.env` via `dotenvy`, typed through `serde`. | BR-01 · SH-01/03 | Must | Typed `config`+`serde` (Rust adaptation) | Given `config/app.toml` + `.env` both set `APP_PORT`, When app boots, Then `.env` value wins and `config::App` deserializes without error |
| FR-002 | The system **shall** provide a service container supporting `Bind` (transient), `Singleton` (once), and `Instance` (value) with type-safe `Make<T>` resolution. | BR-01 · SH-01 | Must | `Container::call` nullable-class defaults → `null` (#20 breaking) — Rust models as `Option<T>` | Given a `Singleton::<CacheStore>` is registered, When `Make::<CacheStore>` is called twice, Then same `Arc` is returned (`ptr_eq`) |
| FR-003 | The system **shall** enforce provider lifecycle `register` → `boot` with explicit DAG ordering via `Relationship()`-equivalent and a `Runner` lifecycle (HTTP/Queue/Schedule). | BR-01 · SH-01 | Must | Goravel reference; `withScheduling` deferred (#10) | Given provider B declares `depends_on = [A]`, When boot order resolves, Then A boots before B or boot fails with cycle error |
| FR-004 | The system **shall** shut down gracefully on `SIGTERM`/`SIGINT` via `tokio` signal handling, draining in-flight HTTP/queue tasks up to a configurable timeout. | BR-01 · SH-03 | Must | Rust runtime requirement | Given app serves a 5s request, When `SIGTERM` arrives, Then request completes or timeout error is emitted, no data loss |
| FR-005 | The system **shall** scaffold a new application via `cargo rustasea new <app>` producing `bootstrap/app.rs`, `config/`, `routes/web.rs`, `.env.example`, and workspace `Cargo.toml`. | BR-01 · SH-02 | Must | Artisan analogue | Given `cargo rustasea new demo` runs, When workspace is listed, Then `demo/bootstrap/app.rs` exists and `cargo check` passes |
| FR-006 | The system **shall** bind `Manager::extend` closures to the manager instance so `self` resolves correctly inside driver extensions (mirrors Laravel 13 fix). | BR-01 | Must | #20 `Manager::extend` closures now bound | Given a custom `Cache` driver registered via `extend`, When driver closure captures manager, Then `self` resolves to manager instance |
| FR-007 | The system **shall** expose `.env` + layered config errors as typed diagnostics with file/line context (not panics). | BR-01 · SH-03 | Should | operability | Given `config/database.toml` has invalid TOML, When boot starts, Then typed `ConfigError::Parse { file, line, source }` is returned |
| FR-008 | The system **shall** support an `Application` singleton resolvable from any provider via `AppState` injection (no `static mut`). | BR-01 · SH-01 | Must | No facades doctrine | Given provider `boot` receives `AppState`, When it calls `state.app()`, Then same `Application` instance is observed |

> M0 NFR highlights in §4.

---

### M1 — Routing & HTTP (depends on: M0)

> Goal: expressive routing with typed extractors, groups, and introspection.

| # | Requirement | Source | Priority | Laravel 13 Feature | Acceptance Hint |
|---|-------------|--------|----------|--------------------|-----------------|
| FR-100 | The system **shall** provide HTTP method helpers `get`/`post`/`put`/`delete`/`patch`/`options`/`any` over `axum` with `#[route]` proc-macro ergonomics. | BR-02 · SH-02 | Must | Routing baseline | Given `Route::get("/users", [UserController, "index"])` equivalent, When `GET /users` is requested, Then controller action runs and returns `200` |
| FR-101 | The system **shall** support route groups with prefix, name prefix, and middleware stacking, plus a `resource` helper generating CRUD routes. | BR-02 | Must | Goravel parity | Given a group with `prefix("/api/v1")` and 3 routes, When `route:list` runs, Then all routes show with `/api/v1` prefix |
| FR-102 | The system **shall** prioritize domain routes before non-domain routes so catch-all subdomains do not shadow non-domain routes. | BR-02 | Must | #19 domain routing priority | Given `*.example.com/*` and `/docs` both match `docs.example.com/docs`, When request arrives, Then domain route wins deterministically |
| FR-103 | The system **shall** expose `cargo rustasea route:list` showing path, method, name, middleware stack, and **binding fields** (#20). | BR-02 · BR-06 | Must | #20 route introspection | Given a route with `{user:slug}` binding, When `route:list --json` runs, Then output includes `binding_fields: ["slug"]` |
| FR-104 | The system **shall** provide a middleware stack including `throttle`/`cors`/`TrimStrings`-equivalent, composable via `tower`/`tower-http`. | BR-02 · SH-03 | Must | Baseline + `throttle` | Given route with `throttle(60, per_minute).by(ip)`, When 61st request in 60s arrives, Then `429 Too Many Requests` with `Retry-After` |
| FR-105 | The system **shall** provide typed request extractors (`Json<T>`, `Query<T>`, `Path<T>`, `State<AppState>`) with `serde` validation surfacing. | BR-02 · SH-02 | Must |Typed ergonomics | Given handler `fn(Json(CreateUser): Json<CreateUser>)`, When payload fails `validator` rules, Then `422` with `ErrorBag` JSON is returned |
| FR-106 | The system **shall** provide `Json` and `View` (`askama`/`minijinja`) typed responses with correct `Content-Type` and status. | BR-02 | Must | Baseline | Given handler returns `Json(user)`, When observed over wire, Then `Content-Type: application/json` and serialized `user` body |
| FR-107 | The system **shall** provide an HTTP client wrapping `reqwest` with `throw` callbacks, retry, and `CarbonInterval`-style timeout configuration. | BR-02 | Must | #18 HTTP Client & Process | Given `Http::get(url).throw(|resp| resp.status().is_server_error()).timeout(Duration::from_secs(5))`, When server returns `500`, Then callback throws typed `HttpError` |
| FR-108 | The system **shall** support process execution with idle-timeout exceptions and `FakeInvokedProcess::stop`/`ensureNotTimedOut` semantics behind `cargo rustasea` internal tooling. | BR-02 | Should | #18 Process | Given process with 5s idle timeout, When process idles >5s, Then `ProcessIdleTimeout` error is surfaced |
| FR-109 | The system **shall** emit `ModelInspector`-compatible metadata for `show:model`-equivalent CLI inspection. | BR-02 · BR-06 | Should | #20 `show:model` | Given model `User` with `#[derive(Model)]`, When `cargo rustasea show:model User` runs, Then attributes, relations, casts are listed |

---

### M2 — ORM & Database (depends on: M0, M1)

> Goal: fluent, type-safe DB layer.

| # | Requirement | Source | Priority | Laravel 13 # | Acceptance Hint |
|---|-------------|--------|----------|--------------|-----------------|
| FR-200 | The system **shall** provide a query builder over `sqlx`/`sea-orm` with drivers for Postgres, MySQL, SQLite supporting `sqlx::migrate!`. | BR-03 | Must | Baseline + #14 | Given Postgres DSN vs SQLite file, When builder runs `where("id", 1).first()`, Then driver-specific SQL is generated |
| FR-201 | The system **shall** support `#[derive(Model)]` with `id`/`created_at`/`updated_at`/`deleted_at` (soft delete), snake_plural table convention, casts, and relations. | BR-03 | Must | #13 eager-relation serialization | Given `User` with `has_many posts`, When `User::with("posts").find(1)` runs, Then posts are eager-loaded |
| FR-202 | The system **shall** support fluent `where`/`orWhere`/`whereJsonContains`/`whereJson` + `find`/`first`/`firstOrFail` + `create`/`save`/`update`/`delete`/`forceDelete`. | BR-03 | Must | Baseline | Given soft-deleted row, When `find(1)` runs without `withTrashed`, Then `None` is returned |
| FR-203 | The system **shall** support `paginate`/`cursor`/`chunkBy`/`orWhereKey`/`orWhereKeyNot`/`whereBinary`/`StraightJoin`/`insertOrIgnoreReturning`/`saveOrIgnore`/`refreshForUpdate`. | BR-03 | Must | #15 builder additions | Given 10k rows, When `chunkBy("id", 500, |chunk| ...)` runs, Then chunks of 500 are yielded without OOM |
| FR-204 | The system **shall** enforce `upsert` with non-empty `uniqueBy` — throwing `UpsertError::EmptyUniqueBy` when violated — and support MySQL `DELETE … JOIN … ORDER BY/LIMIT` compilation. | BR-03 | Must | #14 upsert/delete | Given `upsert(rows, unique_by: [])`, When executed, Then error is returned, not silently skipped |
| FR-205 | The system **shall** provide `toSql`/`toRawSql`, pessimistic locks (`forUpdate`/`sharedLock`), scopes, transactions, and raw queries. | BR-03 | Must | Baseline | Given transaction with `forUpdate` inside `db.transaction(|tx| ...)`, When concurrent writer attempts update, Then it blocks until commit |
| FR-206 | The system **shall** preserve eager-loaded relations through `serde` collection round-trip (serialize → deserialize restores relations). | BR-03 | Must | #13 collection serialization | Given `users` with eager `posts`, When `serde_json::to_string` then `from_str` runs, Then `posts` relation survives |
| FR-207 | The system **shall** provide `vector` Blueprint column type, `whereVectorSimilarTo` query, `dropVectorIndex`, and `Str::toEmbeddings` trait with `pgvector` driver; MariaDB vector behind feature flag. | BR-03 · BR-07 | Must | #6 vector search (M2 initial; M6 full) | Given `products.vector` with 1536-dim embeddings, When `whereVectorSimilarTo("vector", &query_emb, limit: 10)` runs, Then top-10 nearest rows returned by cosine distance |
| FR-208 | The system **shall** provide `cargo rustasea make:migration` / `migrate` / `migrate:fresh` wrapping versioned, reversible migrations. | BR-03 · BR-06 | Must | Baseline | Given new migration file generated, When `migrate` then `migrate:fresh --seed` runs, Then DB ends at same version after re-run |
| FR-209 | The system **shall** provide seeders and factories via `Factory::create`, with `Str` factories and per-test resets. | BR-03 · BR-06 | Must | #20 Str factory resets | Given `UserFactory::create(5)` in a test, When next test runs, Then sequence counters reset |
| FR-210 | The system **shall** support `PDO FETCH`-like fetch modes abstraction behind the builder (typed Row mapping). | BR-03 | Could | #15 | Given `fetch_mode: FetchMode::Assoc`, When query runs, Then rows returned as associative maps |

---

### M3 — Auth, Middleware & Validation (depends on: M1, M2)

> Goal: hardened auth/validation at Laravel 13 security defaults.

| # | Requirement | Source | Priority | Laravel 13 # | Acceptance Hint |
|---|-------------|--------|----------|--------------|-----------------|
| FR-300 | The system **shall** provide JWT guard (`jsonwebtoken`) with `login`/`loginUsingId`/`parse`/`refresh`/`logout` + session guard (`tower-sessions`). | BR-04 · SH-02 | Must | Baseline | Given valid credentials, When `Auth::guard("jwt").login(&creds)` runs, Then JWT token string returned, `parse(token)` yields `user.id` |
| FR-301 | The system **shall** support `Auth::extend` for custom guards and return typed `Error::GuardMismatch` on guard mismatch. | BR-04 | Must | #11 context | Given `Auth::guard("api")` called where only `"jwt"` registered, When `user()` runs, Then `GuardMismatch { expected, actual }` error |
| FR-302 | The system **shall** protect routes with origin-aware CSRF via `PreventRequestForgery` checking `Sec-Fetch-Site` (not just token), rejecting cross-site `POST` without valid origin. | BR-04 · SH-07 | Must | #11 forgery protection | Given `POST /form` with `Sec-Fetch-Site: cross-site` and invalid origin, When middleware runs, Then `403` with `CsrfError::UntrustedOrigin` |
| FR-303 | The system **shall** use JSON session serialization by default and hyphenated cache/Redis prefixes (`-cache-` vs `_cache_`) matching Laravel 13.#12 behavior. | BR-04 · SH-07 | Must | #12 cache & session hardening | Given default config, When session cookie emitted, Then `serialization = "json"` and cache prefix contains `-cache-` |
| FR-304 | The system **shall** enforce a `serializable_classes` allow-list for cache/session deserialization, rejecting unlisted types. | BR-04 · SH-07 | Must | #12 | Given `serializable_classes: ["App::UserDto"]`, When cached `AdminDto` is deserialized, Then error is returned |
| FR-305 | The system **shall** provide `#[middleware]` and `#[authorize]` proc-macro attributes for declarative route/controller protection. | BR-04 · BR-06 | Must | #7 expanded attributes | Given `#[middleware("auth:jwt")]` on handler, When unauthenticated request arrives, Then `401` |
| FR-306 | The system **shall** provide a rate limiter `limit.perMinute(n).by(ip)` mapping to `Throttle` middleware with per-key buckets. | BR-04 | Must | Middleware | Given `limit.per_minute(10).by_ip()` on route, When 11th request in 60s, Then `429` + `Retry-After` |
| FR-307 | The system **shall** perform strict comparison for `in_array`/`contains`/`doesnt_contain` validation rules (no loose equality). | BR-04 | Must | #19 strict validation | Given `in_array: [1, "1"]` strict, When input is `1` (int) vs `"1"` (str), Then only exact type matches pass |
| FR-308 | The system **shall** return `ErrorBag` per form request so multiple field errors coexist keyed by field. | BR-04 | Must | #19 `ErrorBag` | Given form with `email`+`password` errors, When validation fails, Then `ErrorBag` contains both `email` and `password` keys |
| FR-309 | The system **shall** provide `#[validate]` proc-macro wiring `validator` derive rules to handler extractors. | BR-04 · BR-06 | Must | #7 | Given `#[validate] struct CreateUser { #[validate(length(min=3))] name: String }`, When payload `name="ab"` submitted, Then validation fails before handler body |
| FR-310 | The system **shall** provide CORS middleware via `tower-http` with allow-listed origins. | BR-04 | Should | Baseline | Given `Origin: https://evil.com` not in allow-list, When preflight runs, Then `403` or no `Access-Control-Allow-Origin` |
| FR-311 | The system **shall** support `markEmailAsUnverified` semantics via auth trait hook. | BR-04 | Should | #16 `MustVerifyEmail::markEmailAsUnverified` | Given verified user, When `markEmailAsUnverified` called, Then `email_verified_at` cleared |

---

### M4 — Queue, Cache, Scheduling & Events (depends on: M0, M2, M3)

> Goal: observable async workloads.

| # | Requirement | Source | Priority | Laravel 13 # | Acceptance Hint |
|---|-------------|--------|----------|--------------|-----------------|
| FR-400 | The system **shall** provide typed `Job` trait with `handle(self)` and retry contracts `ShouldRetry`/`ShouldRetryUntil` + declarative `#[tries(n)]`/`#[backoff(secs)]`/`#[timeout(secs)]`. | BR-05 · SH-02 | Must | #7 attrs; baseline | Given `#[tries(3)] #[backoff(10)] struct SendEmail`, When job fails twice, Then retried with 10s backoff, third failure goes to failed_jobs |
| FR-401 | The system **shall** support central queue routing `Queue::route::<Job>(connection:, queue:)` registry, with per-dispatch `onQueue`/`onConnection` override. | BR-05 | Must | #4 queue routing | Given `Queue::route::<ProcessPodcast>(queue: "podcasts")`, When `ProcessPodcast::dispatch(payload)`, Then routed to `podcasts` queue without explicit `onQueue` |
| FR-402 | The system **shall** support drivers `sync` + `database` + `redis` (`deadpool-redis`) with `dispatch`/`dispatchSync`/`chain`/`delay`/`onQueue`/`onConnection` + batch dispatch + `queue:failed`/`queue:retry` and `failed_jobs` table. | BR-05 | Must | Baseline | Given chain `[JobA, JobB, JobC]` where JobB fails, When chain runs, Then JobC not executed, failure recorded |
| FR-403 | The system **shall** add `Cache::touch(key, ttl)` to `Store` + `Repository` traits to extend TTL without `get`/`set`, emitting `CacheTouchFailed` only on store errors. | BR-05 | Must | #5 `Cache::touch` | Given cached `k` with TTL 60s, When `touch("k", 120s)` after 30s, Then TTL extends to 120s from touch point |
| FR-404 | The system **shall** provide cache stores `memory` (`moka`) + `redis` behind shared `Store` trait with `get`/`put`/`add`/`remember`/`forever`/`forget`/`flush`/`increment`/`decrement`/`pull`/`has` + `withContext`/`store(name)`. | BR-05 | Must | Baseline + #12 hardening | Given `Cache::store("redis").put("k","v", 60s)`, When `Cache::store("memory").get("k")`, Then `None` (stores isolated) |
| FR-405 | The system **shall** provide `Lock` with atomic `get`/`block`/`release` over both stores. | BR-05 | Must | Cache | Given two workers contending for `Lock("billing")`, When worker A holds lock, Then worker B `block(5s)` waits or times out |
| FR-406 | The system **shall** provide `Event` trait + `Listener` with `Queue { enable: true }` for async handling, `dispatch` + `dispatchAfterResponse`, and `JobAttempted { exception }` / `QueueBusy { connectionName }` events with renamed fields. | BR-05 | Must | #16 contract expansion | Given `Listener { queue: Queue { enable: true } }`, When event dispatched, Then listener enqueued as job, not run inline |
| FR-407 | The system **shall** schedule via `schedule:list`/`schedule:run` with frequencies `daily`/`cron`/`everyMinute`/`skipIfStillRunning`/`onOneServer` (distributed lock). | BR-05 | Must | Baseline | Given job with `everyMinute` + `skipIfStillRunning`, When previous run still active at tick, Then new tick skipped |
| FR-408 | The system **shall** support `schedule:pause`/`schedule:resume` CLI commands emitting `SchedulePaused`/`ScheduleResumed` events. | BR-05 | Must | #10 pause/resume | Given scheduler running, When `schedule:pause` executes, Then jobs stop firing and `SchedulePaused` event emitted |
| FR-409 | The system **shall** expose Cloud queue metrics `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` via `Queue` trait. | BR-05 · SH-03 | Must | #8 Cloud facade & metrics | Given queue depth 42, When `pendingSize("redis", "podcasts")` called, Then `42` returned; `creationTimeOfOldestPendingJob` returns RFC3339 timestamp |
| FR-410 | The system **shall** defer `withScheduling` (provider registration) until first schedule tick, mirroring Laravel 13's deferred semantics. | BR-05 | Should | #10 `withScheduling` deferred | Given provider registers schedule in `boot`, When app boots without scheduler runner, Then schedule not polled until `schedule:run` triggered |

---

### M5 — DX, CLI & Testing (depends on: M0–M4)

> Goal: Laravel-like DX loop.

| # | Requirement | Source | Priority | Laravel 13 # | Acceptance Hint |
|---|-------------|--------|----------|--------------|-----------------|
| FR-500 | The system **shall** provide `cargo rustasea` CLI via `clap` (derive) + `cargo xtask` with `list` command and typed args/flags per command. | BR-06 | Must | Artisan analogue | Given `cargo rustasea list --json`, When run, Then JSON with command names + signatures emitted |
| FR-501 | The system **shall** provide `make:*` generators: `controller`, `model`, `provider`, `command`, `job`, `event`, `listener`, `observer`, `test`, `seeder`, `agent`, `tool` producing `rustfmt`-clean, `clippy`-clean code. | BR-06 · SH-02 | Must | `make:*` parity; #2 agents | Given `cargo rustasea make:controller UserController`, When file listed, Then `app/http/controllers/user_controller.rs` exists, formatted |
| FR-502 | The system **shall** support typed command args/flags via proc-macro plus declarative `#[usage]`/`#[help]`/`#[hidden]` for commands. | BR-06 | Must | #7 Artisan attrs | Given `#[usage("app:send {user}")]` on command, When `list` runs, Then usage string rendered |
| FR-503 | The system **shall** provide interactive prompts `ask`/`secret`/`confirm`/`choice`/`multiSelect` and output helpers `table`/`progressBar`/`spinner` via `dialoguer`/`indicatif`. | BR-06 | Must | Artisan DX | Given command invoking `confirm("Proceed?")`, When answered `n`, Then command aborts |
| FR-504 | The system **shall** support graceful shutdown for commands via `Shutdownable` trait on long-running workers. | BR-06 | Must | `Shutdownable` | Given `queue:work` running, When `SIGTERM`, Then worker drains then exits with code 0 |
| FR-505 | The system **shall** expose programmatic `Artisan::call(command, args)` for in-process invocation. | BR-06 | Should | Artisan | Given `Artisan::call("migrate", vec![])` in test, When called, Then migration runs in-process |
| FR-506 | The system **shall** provide declarative attributes `#[middleware]`/`#[authorize]`/`#[tries]`/`#[backoff]`/`#[timeout]`/`#[failOnTimeout]`/`#[withoutBroadcasting]` etc. matching Laravel 13 #7 surface. | BR-06 | Must | #7 expanded attributes | Given `#[tries(3)] struct MyJob`, When dispatched and fails, Then retry semantics from attribute applied |
| FR-507 | The system **shall** provide `TestCase` harness with per-package `.env.testing` and isolated DB/cache via `testcontainers` + `sqlx::test`, torn down after suite. | BR-06 · SH-01 | Must | Testing | Given two test files both use `TestCase`, When run in parallel, Then isolated Postgres ports do not collide |
| FR-508 | The system **shall** ensure `Str` factories reset between tests so sequence counters do not leak. | BR-06 | Must | #20 Str factories reset | Given factory sequence in test A increments to 10, When test B runs, Then sequence starts at 1 |
| FR-509 | The system **shall** ship paginator views (renamed `bootstrap-3`-equivalent) for HTML pagination. | BR-06 | Should | #20 paginator views | Given `paginate(15)` with view `bootstrap-3`, When rendered, Then paginator HTML emitted |

---

### M6 — Advanced: Broadcasting, Search, Filesystem, AI SDK, Real-time (depends on: M1–M5)

> Goal: differentiate; complete Laravel 13 parity on advanced surface.

| # | Requirement | Source | Priority | Laravel 13 # | Acceptance Hint |
|---|-------------|--------|----------|--------------|-----------------|
| FR-600 | The system **shall** provide WebSocket broadcasting via `axum::extract::ws` + `tokio-tungstenite`, with channel auth and `ShouldBroadcast` trait. | BR-07 | Must | Broadcasting | Given client subscribes to `private-chat.1` without auth, When message broadcast, Then `403` auth rejection |
| FR-601 | The system **shall** provide SSE via `Response::eventStream` (see `ResponseFactory::eventStream` #16). | BR-07 | Must | #16 SSE | Given `Response::eventStream(stream)`, When client connects, Then `Content-Type: text/event-stream` and chunks streamed |
| FR-602 | The system **shall** provide `whereVectorSimilarTo` + `Str::toEmbeddings` + `dropVectorIndex` with `pgvector`/`async-openai` trait and feature-gated embeddings. | BR-07 · SH-04 | Must | #6 full; #2 `SimilaritySearch` | Given `Document` with `vector` column, When `whereVectorSimilarTo("vector", &embedding, limit:5)` runs, Then 5 nearest returned |
| FR-603 | The system **shall** provide read-through `Storage` (primary + fallback disk, optional copy-back) with `Storage::path()` confined to disk root (no path traversal). | BR-07 | Must | #9 read-through FS | Given `readThrough { primary: "s3", fallback: "local" }` and file only on `local`, When `Storage::get("a/b.txt")`, Then content read from fallback |
| FR-604 | The system **shall** provide `JsonApiResource` with sparse fieldsets (`fields[users]=name,email`), relationship inclusion (`include=posts`), links, and `Content-Type: application/vnd.api+json` + JSON:API headers. | BR-07 | Must | #3 JSON:API | Given `UserResource::new(user).include("posts")`, When `to_response()` called, Then JSON:API document with `data`, `included`, `links` |
| FR-605 | The system **shall** support queued notifications that respect `#[deleteWhenMissingModels]` (skip send if model deleted before send). | BR-07 | Should | #17 mail/notifications | Given queued notification for deleted `User`, When queue worker processes it, Then job skipped, not retried |
| FR-606 | The system **shall** provide AI SDK provider-agnostic trait over 12 providers: OpenAI, Anthropic, Gemini, Azure, Bedrock, Groq, xAI, DeepSeek, Mistral, Ollama, OpenRouter, OpenAI-Compatible with text/image/audio/embeddings/reranking/files/vector-stores. | BR-07 · SH-04 | Must | #1 AI SDK | Given `Ai::provider("anthropic").text(prompt)`, When called with same `Agent`, Then same trait method resolves per provider |
| FR-607 | The system **shall** support AI Agents with tools, structured output, streaming, broadcasting, queueing, sub-agents, middleware, anonymous agents, and deferred loaders `SimilaritySearch`/`FileStorage`/`ToolSearch`. | BR-07 · SH-04 | Must | #2 Agents | Given `Agent` with `Tool SearchDocs` + `middleware(Logging)`, When prompted, Then tools invoked, output streamed, middleware observed |
| FR-608 | The system **shall** support `make:agent` / `make:tool` generators producing typed agent/tool scaffolds with `#[derive(Tool)]` equivalent. | BR-07 · SH-02 | Must | #2 make:agent/tool | Given `cargo rustasea make:agent SupportAgent`, When checked, Then `app/ai/agents/support_agent.rs` generated, compiles |
| FR-609 | The system **shall** support MCP (Model Context Protocol) for agent tool discovery. | BR-07 | Should | #2 MCP | Given MCP server registered, When agent lists tools, Then MCP-provided tools included |
| FR-610 | The system **shall** stream AI agent responses, with broadcast over WebSocket and queueing of tool calls. | BR-07 | Must | #2 streaming/broadcast/queue | Given agent streaming 1k tokens, When subscriber listens on WebSocket, Then chunks arrive in order with `event: token` |
| FR-611 | The system **shall** enforce `Storage::path()` confinement — any resolved path must stay under disk root, rejecting `../` traversal with `StorageError::PathTraversal`. | BR-07 · SH-07 | Must | #9 `Storage::path()` confinement | Given `Storage::path("../../etc/passwd")`, When validated, Then error, not resolved path |
| FR-612 | The system **shall** ship with degraded-mode AI so `rustasea-ai` crate is opt-in behind feature flag (core does not pull AI deps). | BR-07 · BR-08 | Must | Architectural | Given workspace with only `rustasea-router`, When `cargo check` runs, Then no `async-openai` in dep tree |

---

## 4. Non-Functional Requirements

| # | Requirement | Category | Priority | Metric / Target | Source |
|---|-------------|----------|----------|-----------------|--------|
| NFR-Per-01 | Cold boot to ready (listening) | Performance | Must | <2s on CI (2 vCPU) for M0 scaffold with 5 providers | BR-01 · operability |
| NFR-Per-02 | HTTP p95 latency (no DB) | Performance | Must | <50ms p95 for `GET /users` (axum hello, 1k RPS, localhost) | BR-02 |
| NFR-Per-03 | Cache latency | Performance | Should | `Cache::get` p95 <5ms (memory) / <20ms (redis, localhost) | BR-05 |
| NFR-Per-04 | Generated code compile | Performance | Must | `cargo check` after `make:*` <10s incremental | BR-06 |
| NFR-Sec-01 | CSRF origin-aware | Security | Must | `Sec-Fetch-Site: cross-site` without valid origin → `403`; token-only bypass rejected | #11 · SH-07 |
| NFR-Sec-02 | Session serialization | Security | Must | Default `json`; deserialization allow-list enforced; invalid prefix rejected | #12 · SH-07 |
| NFR-Sec-03 | Path confinement | Security | Must | `Storage::path()` traversal → `PathTraversal` error; fuzz test with `..` payloads | #9 |
| NFR-Sec-04 | Password hashing | Security | Must | `argon2` with per-password salt; constant-time verification | BR-04 |
| NFR-Rel-01 | Graceful shutdown | Reliability | Must | In-flight requests drained up to `shutdown_timeout` (default 10s); queue workers ack/nack correctly | BR-01 · SH-03 |
| NFR-Rel-02 | Migration idempotence | Reliability | Must | `migrate` re-run is no-op; `migrate:fresh` is reversible | BR-03 |
| NFR-Rel-03 | Lock liveness | Reliability | Must | `Lock::block` respects timeout; deadlock detection via lease expiry | BR-05 |
| NFR-Usa-01 | DX Likert | Usability | Should | ≥4/5 on "feels like Laravel" from 5+ ex-Laravel Rust developers at M1/M5 dog-food | BR-02 · BR-06 |
| NFR-Usa-02 | Error diagnostics | Usability | Must | Every `anyhow`-style error carries structured `code` + `hint` + `source` chain; no bare `unwrap` in framework crates | BR-01 |
| NFR-Usa-03 | Generated code quality | Usability | Must | `rustfmt` + `clippy -- -D warnings` clean on all `make:*` output | C-04 |
| NFR-Sca-01 | Connection pooling | Scalability | Must | `deadpool` pools sized via config; handles 100 concurrent queue workers + 1k HTTP concurrency in bench | BR-05 |
| NFR-Sca-02 | Workspace modularity | Scalability | Must | Single-crate check (`rustasea-router` only) passes; dep tree does not pull ORM/queue/AI | BR-08 |
| NFR-Com-01 | MSRV | Compatibility | Must | Rust 1.88+, edition 2021; `tokio` 1.x | C-01 |
| NFR-Com-02 | Supported DBs | Compatibility | Must | Postgres (primary), MySQL, SQLite; vectors only on Postgres/MariaDB (feature-flagged) | BR-03 |
| NFR-Mai-01 | Observability parity | Maintainability | Should | `route:list` binding fields + `show:model` + queue metrics (#8) all via typed traits, not ad-hoc logging | #20 · #8 |

---

## 5. Constraints and Assumptions (Normative)

| # | Constraint | Applies To |
|---|------------|------------|
| C-01 | Rust 1.88+ stable, `tokio` everywhere | All |
| C-02 | No global `static mut` facades — `AppState` via `axum::extract::State` | All |
| C-03 | No `any`/`interface{}` for domain payloads (`Job<T>`, `Event<T>`) | M4, M6 |
| C-04 | `rustfmt` + `clippy -D warnings` clean on all generated code | M5, M6 |
| C-05 | Incremental adoption — each crate `cargo check`-clean standalone | All |

**Assumptions:** See `brd.md` §4 (A-01 … A-04); additionally, `pgvector` extension availability is assumed on target Postgres (gated behind migration check), and 12-provider list matches Laravel docs (source 3) at doc date — new providers post-RFC are additive only.

---

## 6. Prioritization (MoSCoW by Milestone)

| MoSCoW | Milestones | Rationale |
|--------|------------|-----------|
| **Must** | M0, M1, M2, M3, M4, M5-core | Shippable Laravel-parity framework; blocks the value prop |
| **Should** | M6-full (AI/broadcast/JSON:API/storage); M5 `Artisan::call`/`paginator` | Differentiator — ships as `Should` so Must block is not gated on AI provider churn |
| **Could** | `FR-210` fetch modes; `FR-611` extra FS drivers | Nice-to-have, deferred |
| **Won't (now)** | Nova/Filament admin, Blade runtime, PaaS, PHP bridge | `brd.md` §4 Won't |

**Cost of delay:** M0 > M1 > M2 > M3 > M4 > M5 > M6. M0/M1 have highest cost of delay (block everything). M6 has lowest must-delay (product can ship without AI) but highest differentiation value.

---

## 7. Risk Register (L×I, max 25) and Top-5 Critical

| # | Risk | Category | L (1-5) | I (1-5) | Score | Early Warning | Mitigation | Owner |
|---|------|----------|---------|---------|-------|---------------|------------|-------|
| R-01 | ORM duality (`sqlx` vs `sea-orm`) doubles maintenance + proc-macro complexity | Technical | 3 | 5 | **15** | M2 estimates inflate >30%; PR review time >2× | Choose primary (`sqlx`) after M0 spike; `sea-orm` as optional compat layer behind feature flag; ADR before M2 | Tech Lead |
| R-02 | 12-provider AI trait drifts as providers version APIs (auth, streaming, tool schema) | Technical | 4 | 3 | **12** | Provider changelog breakage within 30d of tag | Provider trait versioned per-semver; per-provider adapter crate; feature-flag each provider | M6 lead |
| R-03 | `Sec-Fetch-Site` CSRF diverges from evolving browser `Fetch Metadata` spec | Technical | 2 | 5 | **10** | Browser beta changes spec field semantics | Gate on spec version in tests; allow `Sec-Fetch-Site` as signal not sole gate; keep token as primary | Security |
| R-04 | Cache `touch` on Redis Cluster vs standalone diverges (`touch` not in Redis Cluster spec uniformly) | Technical | 3 | 3 | **9** | `EXPIRE` failure on cluster shards | Redis driver abstracts `EXPIRE` fallback; test matrix: standalone + cluster | M4 lead |
| R-05 | `Queue::route` wrong-queue delivery due to registry ordering or race during provider boot | Technical | 3 | 4 | **12** | E2E routed-job arrives on wrong queue in CI | Registry is `OnceLock` after `boot`; attempt duplicate `route` returns error; E2E per-job routing test | M4 lead |
| R-06 | Vector search `pgvector` extension missing in target Postgres (managed DBs) | Operational | 3 | 4 | **12** | `CREATE EXTENSION vector` fails in migration | Migration guarded behind `vector` feature flag + `has_extension("vector")` check; doc lists managed-DB workaround | M2/M6 lead |
| R-07 | Scope creep: M6 AI/broadcast/JSON:API drags Must block past target | Business | 3 | 4 | **12** | M6 stories enter M0–M5 sprints | Strict MoSCoW gate; M6 labeled Should; RFC required to promote M6 FR to Must | PM |
| R-08 | `Str` factory / `testcontainers` state leaks across parallel tests | Technical | 3 | 3 | **9** | Flaky test with sequence drift | Test harness resets `Str` factories per test + per-worker PG ports (`testcontainers` random ports) | M5 lead |
| R-09 | Hyphenated cache/Redis prefixes break existing Rust `redis` deployments | Operational | 2 | 4 | **8** | Cache miss storm after upgrade | Config preserves `CACHE_PREFIX`/`REDIS_PREFIX` override; migration guide documents prefix change | M4 lead |

### Top 5 Critical (by Score; ties = higher I)

1. **R-01 (15) ORM duality** — Mitigation: spike + ADR pre-M2; gate commit to `sqlx`-primary with `sea-orm` shim. Owner: Tech Lead.
2. **R-05 / R-06 / R-07 (12, I=4)** queue routing, pgvector availability, scope creep — see rows above for per-risk mitigation.
3. **R-02 (12, I=3)** provider drift — per-provider feature-flag + semver per adapter.

### Cost Estimate (Planning-Gate Consistency)

| Milestone | Complexity | Est. Effort (dev-weeks, 2-person team) | Assumptions |
|-----------|------------|----------------------------------------|-------------|
| M0 | Medium | 3–4 | `config`+`dotenvy` + container + provider DAG + `xtask` scaffold |
| M1 | Medium-High | 3–4 | `axum` ergonomics + `route:list` + middleware + reqwest client |
| M2 | High | 4–6 | `sqlx` builder + derive macros + `pgvector` + migration tooling |
| M3 | Medium | 3–4 | JWT+session + `Sec-Fetch-Site` CSRF + `#[validate]`/`ErrorBag` |
| M4 | High | 4–6 | Typed queue/cache/events/schedule + redis + `touch`/`Lock`/`pause` |
| M5 | Medium | 3–4 | `clap` CLI + 10+ generators + `TestCase`/`testcontainers` |
| M6 | Very High | 6–8 | Broadcast/SSE + storage read-through + JSON:API + 12-provider AI |
| **Total** | | **26–36 dev-weeks** (~13–18 weeks wall-clock @2 devs, parallel partially) | +20% contingency standard (30–40% on M6) |

> Rates/timelines are estimates with `team_size = 2` assumption per `product-planning` rule; actuals require velocity calibration after M0. These supersede no formal contract — flagged as assumption.

---

## 8. Traceability Matrix

> Every Laravel 13 feature (#) appears at least once. Every milestone with FR coverage. Every FR points to BDD Feature tag in `bdd-scenarios.md`.

| Laravel 13 Feature (from `docs/laravel-13-research.md` & `README.md`) | Type | PRD FRs | Milestone | BR | BDD Tags (in `bdd-scenarios.md`) |
|---|---|---|---|---|---|
| #1 AI SDK (`laravel/ai`, 12 providers, text/image/audio/embeddings/reranking/files/vector stores) | NEW | FR-606, FR-612 | M6 | BR-07 | `@ai-sdk` |
| #2 AI Agents (tools, structured output, streaming, broadcast, queueing, MCP, sub-agents, middleware, `make:agent`/`make:tool`, deferred loaders) | NEW | FR-607, FR-608, FR-609, FR-610 | M6 | BR-07 | `@ai-agents` |
| #3 JSON:API Resources (`JsonApiResource`, sparse fieldsets, inclusion, links, headers) | NEW | FR-604 | M6 | BR-07 | `@jsonapi` |
| #4 Queue Routing by Class (`Queue::route()`) | NEW | FR-401, FR-402, FR-409* | M4 | BR-05 | `@queue-routing` |
| #5 Cache `touch()` (extend TTL without get/set, `Store`+`Repository` contracts) | NEW | FR-403, FR-404, FR-405 | M4 | BR-05 | `@cache-touch` |
| #6 Semantic / Vector Search (`whereVectorSimilarTo`, `toEmbeddings`, `dropVectorIndex`, `vector` column) | NEW | FR-207 (M2 initial) + FR-602 (M6 full) | M2 + M6 | BR-03 / BR-07 | `@vector-search` |
| #7 Expanded PHP Attributes (declarative) (`#[Middleware]`, `#[Authorize]`, `#[Tries]`, `#[Backoff]`, `#[Timeout]`, Eloquent/events/resource attrs, `#[Usage]`/`#[Help]`/`#[Hidden]`) | NEW | FR-305, FR-309, FR-400, FR-502, FR-506 (+ FR-501 generators) | M3/M4/M5 | BR-04/05/06 | `@attributes` |
| #8 Laravel Cloud Facade & Cloud Queue metrics (`pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob`, `managedQueues`) | NEW | FR-409, FR-402 | M4 | BR-05 | `@queue-metrics` |
| #9 Read-through Filesystem (primary + fallback disk, `Storage::path()` confinement) | NEW | FR-603, FR-611 | M6 | BR-07 | `@storage-readthrough` |
| #10 Schedule Pause / Resume (`schedule:pause`/`resume`, `SchedulePaused`/`Resumed`, `withScheduling` deferred) | NEW | FR-408, FR-410, FR-407 | M4 | BR-05 | `@schedule-pauseresume` |
| #11 Request Forgery Protection (origin-aware `Sec-Fetch-Site`) | IMPROVED | FR-302, FR-301 | M3 | BR-04 | `@csrf-origin` |
| #12 Cache & Session Hardening (JSON serialization, allow-list `serializable_classes`, hyphenated prefixes) | IMPROVED | FR-303, FR-304, FR-404 | M3/M4 | BR-04 | `@cache-session-hardening` |
| #13 Eloquent Collection Serialization (relations survive `serialize`) | IMPROVED | FR-206, FR-201 | M2 | BR-03 | `@collection-serialization` |
| #14 Database Upsert & Delete Improvements (strict `uniqueBy`, MySQL `DELETE JOIN` support) | IMPROVED | FR-204, FR-200 | M2 | BR-03 | `@upsert-delete` |
| #15 Eloquent / Query Builder Additions (`insertOrIgnoreReturning`, `saveOrIgnore`, `refreshForUpdate`, `whereBinary`, `chunkBy`, `orWhereKey`, `StraightJoin`, PDO fetch modes, `Arr::dot($depth)`) | IMPROVED | FR-203, FR-205 | M2 | BR-03 | `@query-builder-additions` |
| #16 Event / Queue Contract Expansion (`JobAttempted::$exception`, `QueueBusy::$connectionName`, `Dispatcher::dispatchAfterResponse`, `ResponseFactory::eventStream`, `MustVerifyEmail::markEmailAsUnverified`) | IMPROVED | FR-406 (dispatchAfterResponse + JobAttempted/QueueBusy) + FR-601 (eventStream) + FR-311 | M4/M6 | BR-05 | `@contracts-expansion` |
| #17 Mail / Notification Defaults (password-reset subject, `#[DeleteWhenMissingModels]`) | IMPROVED | FR-605 | M6 | BR-07 | `@mail-notifications` |
| #18 HTTP Client & Process (`throw` callbacks, `CarbonInterval` timeouts, idle-timeout, `FakeInvokedProcess::stop`) | IMPROVED | FR-107, FR-108 + FR-601 (eventStream adjacency) | M1 + M6 | BR-02 | `@http-client-process` |
| #19 Routing & Validation (domain priority, strict `in_array`/`contains`/`doesnt_contain`, `ErrorBag`) | IMPROVED | FR-102 (domain) + FR-307 (strict) + FR-308 (ErrorBag) + FR-309 | M1/M3 | BR-02/04 | `@routing-validation` |
| #20 Observability & Tooling (`ModelInspector`/`show:model`, `route:list` binding fields, paginator views `bootstrap-3`, `Str` factory resets, `Manager::extend` closure binding) | IMPROVED | FR-109 + FR-103 + FR-006 + FR-508 + FR-509 | M1/M5 + M0 | BR-01/02/06 | `@observability-tooling` |

*FR-409 covers metrics; FR-401 covers routing.

> Additionally: **Container `call` semantics** (#20 adjacency) traced via FR-002/FR-008; `Manager::extend` via FR-006 — both M0 Must items.

### Milestone Coverage Check

| Milestone | FR Count | Has BDD Story? | Has User Story? |
|-----------|----------|----------------|-----------------|
| M0 | 9 (FR-000–FR-008) | Yes | Yes |
| M1 | 10 (FR-100–FR-109) | Yes | Yes |
| M2 | 11 (FR-200–FR-210) | Yes | Yes |
| M3 | 12 (FR-300–FR-311) | Yes | Yes |
| M4 | 11 (FR-400–FR-410) | Yes | Yes |
| M5 | 10 (FR-500–FR-509) | Yes | Yes |
| M6 | 13 (FR-600–FR-612) | Yes | Yes |

### Planning Artifacts

- **Roadmap:** see `brd.md` §1 + `README.md` §Roadmap (Q4 2026 → Q4 2027); phased delivery order is the spec.
- **Sprint allocation:** deferred to sprint-planning post-P2 — prerequisites are these FRs and the design brief.
- **Changelog:** `CHANGELOG.md` (Keep a Changelog) to be created at M0 tag per `product-planning` release-memory rule.

---

*This PRD is the contract for FSD decomposition. Any FR change requires a matching update in `fsd.md`, `user-stories.md`, and `bdd-scenarios.md` and a task comment on TASK-007.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
