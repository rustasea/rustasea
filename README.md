# RustaSea

> A Rust framework with Laravel ergonomics — expressive syntax, convention over configuration, and Rust-grade safety and performance.

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](#license)
[![Status](https://img.shields.io/badge/status-alpha-yellow.svg)](#roadmap)

> **Last updated:** 2026-09-12

---

## Vision

RustaSea brings the developer experience that made Laravel the most loved PHP framework to Rust — without sacrificing what makes Rust great. Route definitions that read like prose, Eloquent-inspired query builders with compile-time safety, Artisan-like code generation via proc-macros, and a service container that leverages Rust's type system instead of fighting it.

**Core thesis:** Laravel proves ergonomics and velocity win hearts; Rust proves safety and performance win production. RustaSea proves you can have both.

**Design principles:**

- **Convention over configuration** — sensible defaults, explicit opt-out.
- **Type safety as a feature** — generics, lifetimes, and traits replace runtime `any` blobs.
- **Zero-cost ergonomics** — expressive APIs that compile away where possible.
- **Incremental adoption** — use one crate or the full stack.

---

## Why Rust × Laravel Ergonomics

| What Laravel does best | What Rust does best | RustaSea synthesis |
|---|---|---|
| Expressive routing, middleware, validation | Ownership, lifetimes, fearless concurrency | `axum` routing with typed extractors + proc-macro attributes (`#[middleware]`, `#[validate]`) |
| Eloquent ORM — fluent, chainable | Compile-time type safety | `sqlx`-backed query builder with derive macros; `whereVectorSimilarTo` from day one |
| Artisan code generation | `cargo` + proc-macros + `clap` | `cargo artisan make:*` with `clap`-powered CLI and `xtask` |
| Queue / Schedule / Events | `tokio` async runtime | Typed jobs/events (no `any`), backpressure-aware queues |
| Blade / JSON:API resources | `serde` / `askama` / `minijinja` | `JsonApiResource` via `serde` with sparse fieldsets + relationship inclusion; `rustasea-view` (askama default, minijinja opt-in) |
| Batteries-included DX | Minimal runtime, no GC | Pay only for crates you include; workspace-gated features |

**Who is RustaSea for?** Teams that outgrew dynamic-language frameworks on performance, correctness, or concurrency — but do not want to outgrow the productivity that made them ship fast.

---

## Laravel 13 Feature Map

Laravel 13.0.0 shipped 2026-03-17 (PHP 8.3+, 20 headline features). Every feature is mapped to the RustaSea milestone that delivers its equivalent.

| # | Laravel 13 Feature | Type | RustaSea Milestone | Notes |
|---|---|---|---|---|
| 1 | AI SDK (`laravel/ai`) — 12 providers, agents, tools, streams | NEW | **M6** | Provider-agnostic trait + agentic workflow |
| 2 | AI Agents (tools, structured output, streaming, MCP, queueing) | NEW | **M6** | `make:agent` / `make:tool`, sub-agents, middleware |
| 3 | JSON:API Resources (`JsonApiResource`) | NEW | **M6** | `serde`-based with sparse fieldsets, links, headers |
| 4 | Queue Routing by Class (`Queue::route()`) | NEW | **M4** | Central registry `Queue::route::<Job>(queue:)` |
| 5 | Cache `touch()` — extend TTL without get/set | NEW | **M4** | Added to `Store` + `Repository` traits |
| 6 | Semantic / Vector Search (`whereVectorSimilarTo`, `toEmbeddings`) | NEW | **M2** + **M6** | `pgvector` + embedding trait from day one |
| 7 | Expanded PHP Attributes (declarative) | NEW | **M5** | Rust proc-macros: `#[middleware]`, `#[tries]`, `#[authorize]`, etc. |
| 8 | Laravel Cloud Facade & Cloud Queue metrics | NEW | **M4** | Cloud driver + queue depth/age metrics |
| 9 | Read-through Filesystem (primary + fallback disk) | NEW | **M6** | `Storage` with fallback + path confinement |
| 10 | Schedule Pause / Resume (`schedule:pause`) | NEW | **M4** | `schedule:pause` / `schedule:resume` + events |
| 11 | Request Forgery Protection (origin-aware `Sec-Fetch-Site`) | IMPROVED | **M3** | `PreventRequestForgery` with origin verification |
| 12 | Cache & Session Hardening (JSON serialization, allow-list) | IMPROVED | **M3** / **M4** | `serde` allow-listing; JSON session store by default |
| 13 | Eloquent Collection Serialization (relations survive serialize) | IMPROVED | **M2** | `serde` round-trip preserves eager-loaded relations |
| 14 | Database Upsert & Delete Improvements (strict `uniqueBy`) | IMPROVED | **M2** | `upsert` validates `uniqueBy`; MySQL `DELETE JOIN` support |
| 15 | Eloquent / Query Builder Additions (`insertOrIgnoreReturning`, etc.) | IMPROVED | **M2** | `chunkBy`, `whereBinary`, `orWhereKey`, `StraightJoin` |
| 16 | Event / Queue Contract Expansion (`dispatchAfterResponse`, etc.) | IMPROVED | **M4** | Expanded `Dispatcher` + `Queue` traits |
| 17 | Mail / Notification Defaults | IMPROVED | **M6** | Queued notification `DeleteWhenMissingModels` |
| 18 | HTTP Client & Process (`throw` callbacks, `CarbonInterval` timeouts) | IMPROVED | **M1** | Typed HTTP client with `reqwest` |
| 19 | Routing & Validation (domain priority, strict `in_array`, `ErrorBag`) | IMPROVED | **M1** / **M3** | Domain-route precedence; strict validation |
| 20 | Observability & Tooling (`show:model`, `route:list` binding fields) | IMPROVED | **M1** / **M5** | Model inspector, route introspection |

> Source: [`docs/laravel-13-research.md`](docs/laravel-13-research.md) — cross-checked against `laravel.com/docs/releases`, `laravel.com/docs/13.x/upgrade`, and `laravel/framework` v13.0.0 changelog.

---

## Goravel Inspiration

[Goravel](https://goravel.dev) (Go port of Laravel, v1.18) is the closest prior art and the primary reference for porting Laravel idioms to a compiled, statically-typed language.

**What Goravel proves works:**

- **Service providers** with explicit `Register` → `Boot` lifecycle and DAG ordering (`Relationship()`) + `Runner` lifecycle (HTTP, Queue, Schedule) translate cleanly to Rust — RustaSea implements this: `ServiceProvider::dependencies()` + a real topological sort with typed cycle detection (`crates/rustasea-foundation/src/lib.rs:27`, `:296`, `BootError::DependencyCycle` at `:187`).
- **Container** (`Bind` / `Singleton` / `Instance` + `Make`) — in Rust this becomes trait objects + `Any` / `typemap` with compile-time registration.
- **ORM** — `facades.Orm().Query()` builder over GORM shows a fluent Eloquent-style API is viable outside PHP; Rust equivalent uses `sqlx` with derive macros.
- **Artisan CLI** (`make:controller`, `make:job`, signatures + typed Args/Flags) maps to `clap` + `cargo xtask` with `#[command]` proc-macros.
- **Queue / Event / Schedule / Cache / Auth** — typed `queue.Arg` / `event.Arg` patterns, `Cache::Remember`, `Auth(ctx).Login` all have direct Rust analogues.

**What RustaSea does differently:**

| Goravel (Go) | RustaSea (Rust) |
|---|---|
| Global facades `facades.Cache().Get()` | Explicit `AppState` via `axum::extract::State` (`OnceLock` / `Arc`, no global `static mut`) |
| `any` / `interface{}` job/event args | Strongly-typed generic jobs/events — `Job<T>`, `Event<T>` |
| Runtime reflection for container | Proc-macros + trait bounds; no reflection |
| `gin` / `fiber` router | `axum` (tower-native, `tokio`-aligned) |
| `testify/suite` + Docker helpers | `cargo test` + `testcontainers` + `PostgresTestDb` fixture |
| Stringly-typed config `GetString("app.name")` | Typed config via `config` + `serde` with env overlay |

> See `TASK-002` findings for the full Goravel → Rust mapping table and crate implications.

---

## Milestones

Milestones are **dependency-ordered**: each builds only on predecessors. No circular dependencies.

> **Status:** See [`docs/milestones.md`](docs/milestones.md) for the authoritative, evidence-backed done/partial/missing recap of M0–M6, and [`docs/laravel-parity.md`](docs/laravel-parity.md) for the Laravel 13.x API adoption mapping.

### M0 — Bootstrap & Core

| Field | Detail |
|---|---|
| **Goal** | Bootable application skeleton with config, container, and service providers. |
| **Scope** | `foundation::Application`, typed config loader (TOML + env overlay), service container (`Bind`/`Singleton`/`Instance`), provider lifecycle (`register` → `boot`) with dependency DAG ordering, graceful shutdown, `.env` support. |
| **Deliverables** | `rustasea` umbrella crate, `rustasea-foundation` crate, `cargo rustasea new <app> --variant {blade\|react\|vue\|livewire}` scaffold, `config/` directory, `bootstrap/app.rs` entry point, example `AppServiceProvider`. |
| **Success Criteria** | `cargo run` boots, loads config from `config/*.toml` + `.env`, resolves a bound singleton from the container, and shuts down gracefully on `SIGTERM`. |
| **Laravel 13 features** | Container `call` semantics, `Manager::extend` closure binding. |

> **Status (2026-09-12):** Partial. The config loader auto-discovers every `config/*.toml` (`crates/rustasea-config/src/lib.rs:89`, `load_from_dir` at `:40`; TOML only — no YAML), and provider boot runs a real topological sort with cycle detection (`crates/rustasea-foundation/src/lib.rs:275`, `:296`). `bootstrap/providers.rs:35` and `bootstrap/commands.rs:12` are populated; the scaffolded `AppServiceProvider` is still a no-op (`bootstrap/providers.rs:17`), and the config loader is not yet mounted during app boot. Tracked in `GAP-019` (completed) and `GAP-P5`.

### M1 — Routing & HTTP

| Field | Detail |
|---|---|
| **Goal** | Expressive HTTP layer with routing, middleware, and request/response ergonomics. |
| **Scope** | `axum`-backed router (`get`/`post`/`put`/`delete`/`patch`/`options`/`any`), route groups + prefix + naming, `resource` helper, domain-aware routing (domain routes prioritized), `route:list` introspection, middleware stack (including `throttle` / `cors`), typed request extractors, `Json`/`View` responses, HTTP client (`reqwest` wrapper with `throw` callbacks). |
| **Deliverables** | `rustasea-router` + `rustasea-http` crates, `routes/web.rs`, `#[route]` proc-macro, `cargo artisan route:list`. |
| **Success Criteria** | Define `Route::get("/users", [UserController, "index"])` equivalent in Rust, hit it with `cargo test` HTTP assertions, see it in `route:list` with middleware and binding fields. Domain catch-all routes do not shadow non-domain routes. |
| **Laravel 13 features** | #18 HTTP Client & Process, #19 domain-route priority, #20 `route:list` binding fields. |

> **Status (2026-09-12):** Partial. The router DSL and controller dispatch are real, but `route:list` still prints an empty table (`crates/rustasea-cli/src/commands/inspect.rs:33`) and the HTTP idle timeout is declared but unenforced (`crates/rustasea-http/src/lib.rs:241`, `:364`).

### M2 — ORM & Database

| Field | Detail |
|---|---|
| **Goal** | Fluent, type-safe database layer with migrations, seeders, and factories. |
| **Scope** | Query builder over `sqlx` (drivers: Postgres, MySQL, SQLite), `where`/`orWhere`/`whereJson*`, `find`/`first`/`firstOrFail`, `create`/`save`/`update`/`delete`/`forceDelete`, `paginate`/`cursor`, scopes, transactions, `toSql`/`toRawSql`, pessimistic locks, raw queries, `insertOrIgnoreReturning`/`saveOrIgnore`/`refreshForUpdate`/`whereBinary`/`chunkBy`/`orWhereKey`, `#[derive(Model)]` with `id`/`created_at`/`updated_at`/`deleted_at` (soft deletes), snake_plural table convention, vector extension (`whereVectorSimilarTo`, `vector` column type). Migrations via the custom `Migrator` (`cargo artisan make:migration` + `migrate`/`migrate:fresh`/`migrate:rollback`), seeders, factories. |
| **Deliverables** | `rustasea-orm` crate, `database/migrations/`, `database/seeders/`, `#[derive(Model)]` macro, `cargo artisan make:model` generator, `pgvector` support behind feature flag. |
| **Success Criteria** | Create a `User` model, run `cargo artisan migrate`, `Factory::create(&user)` in tests, demonstrate `whereVectorSimilarTo` with `pgvector`, and round-trip a collection with eager-loaded relations via `serde`. |
| **Laravel 13 features** | #6 vector search, #13 collection serialization, #14 upsert/delete, #15 query builder additions. |

### M3 — Auth, Middleware & Validation

| Field | Detail |
|---|---|
| **Goal** | Complete auth, authorization, and validation with hardened security defaults. |
| **Scope** | Auth guards (JWT via `jsonwebtoken` + session), `login`/`loginUsingId`/`parse`/`refresh`/`logout`/`user`/`id`, `Auth::extend` for custom guards, `#[authorize]` attribute, CSRF origin-aware protection (`PreventRequestForgery` with `Sec-Fetch-Site` check), `#[middleware]` attribute, rate limiter (`limit.perMinute().by(ip)` → `Throttle`), CORS, validation rules (strict `in_array`/`contains`/`doesnt_contain`, `ErrorBag` for form requests), `#[validate]` proc-macro, session store (JSON serialization by default), security allow-list for deserialization. |
| **Deliverables** | `rustasea-auth` + `rustasea-validation` crates, `app/http/middleware/`, `cargo artisan make:middleware` / `make:request` (both implemented — `crates/rustasea-cli/src/generators/kinds/{middleware,request}.rs:17`), JWT + session guard implementations. |
| **Success Criteria** | Guard mismatch returns typed `Error::GuardMismatch`; CSRF rejects cross-site `POST` without valid `Sec-Fetch-Site`; `#[validate]` rejects strict-mismatch payloads; session cookie uses JSON serialization and hyphenated cache prefix. |
| **Laravel 13 features** | #11 origin-aware CSRF, #12 cache/session hardening, #19 strict validation + `ErrorBag`. |

> **Status (2026-09-12):** Partial. JWT, CSRF, throttle, validation, and the `tower-sessions`-backed `SessionGuard` are real (`crates/rustasea-auth/src/session.rs:160`, login/parse/refresh/logout at `:279`–`:375`), but the declarative attributes (`#[middleware]`, `#[authorize]`, `#[tries]`, `#[backoff]`, `#[timeout]`) still emit metadata consts that nothing consumes at runtime (`crates/rustasea-macros/src/lib.rs:74`, `:111`, `:247`–`:265`). See [`docs/milestones.md`](docs/milestones.md) for the `GAP-003` reconciliation note.

### M4 — Queue, Cache, Scheduling & Events

| Field | Detail |
|---|---|
| **Goal** | Async workloads, caching, scheduling, and event dispatch with observable queue metrics. |
| **Scope** | Queue: `Queue::route::<Job>(connection:, queue:)` central routing, `Job` trait with `handle`, `ShouldRetry` / `#[tries]` / `#[backoff]` / `#[timeout]`, drivers `sync` + `database` + `redis` (via `deadpool-redis`), `dispatch`/`dispatchSync`/`chain`/`delay`/`onQueue`/`onConnection`, batch dispatch, `queue:failed` / `queue:retry`, `failed_jobs` table. Cache: `get`/`put`/`add`/`remember`/`forever`/`forget`/`flush`/`increment`/`decrement`/`pull`/`has` + `touch()` (extend TTL), `Lock` (atomic `get`/`block`/`release`), stores `memory` + `redis`, `withContext`/`store("redis")`. Events: `Event` trait, `Listener` with `Queue { enable: true }` for async, `dispatch` + `dispatchAfterResponse`, `JobAttempted { exception }`, `QueueBusy { connectionName }`. Schedule: `schedule:list`, `schedule:run`, `schedule:pause`/`schedule:resume` + `SchedulePaused`/`ScheduleResumed` events, frequencies (`daily`/`cron`/`everyMinute`/`skipIfStillRunning`/`onOneServer`). Cloud queue metrics (`pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob`). |
| **Deliverables** | `rustasea-queue` + `rustasea-cache` + `rustasea-events` + `rustasea-schedule` crates, `app/jobs/`, `app/events/`, `app/listeners/`, `cargo artisan make:job` / `make:event` / `make:listener`. |
| **Success Criteria** | Dispatch a typed job to a routed queue and assert it executes; `Cache::touch` extends TTL without re-reading; `schedule:pause` halts the scheduler and emits `SchedulePaused`; event listener runs async when `Queue { enable: true }`. |
| **Laravel 13 features** | #4 queue routing, #5 `Cache::touch`, #8 Cloud queue metrics, #10 schedule pause/resume, #16 event/queue contracts. |

> **Status (2026-09-12):** Partial. Real `database` + `redis` queue drivers, the worker loop, DB-backed `failed_jobs`, `queue:work`/`queue:failed`/`queue:retry`, the feature-gated Redis **cache** store, async queue-backed event listeners, and `schedule:list`/`run`/`pause`/`resume` are all implemented (`crates/rustasea-queue/src/driver/{database,redis,worker}.rs`; `crates/rustasea-cli/src/commands/ops.rs:21`, `:72`; `crates/rustasea-cache/src/redis.rs:155`; `crates/rustasea-events/src/dispatcher.rs:82`). Remaining: the in-process `SyncDriver` still keeps failed jobs in memory (`crates/rustasea-queue/src/driver.rs:171`), and declarative job metadata (`#[tries]`/`#[backoff]`/`#[timeout]`) is not consumed at runtime. Tracked in `GAP-004`–`GAP-009` (completed).

### M5 — DX, CLI & Testing

| Field | Detail |
|---|---|
| **Goal** | First-class developer experience: CLI, code generation, and a testing story that feels like Laravel. |
| **Scope** | `cargo artisan` CLI (via `clap` + `cargo xtask`): `list`, `make:*` (controller, middleware, request, model, provider, command, job, event, listener, observer, test, seeder, migration, agent, tool — 15 kinds), typed command args/flags, `ask`/`secret`/`confirm`/`choice`/`multiSelect` prompts, `table`/`progressBar`/`spinner`, graceful shutdown (`Shutdownable`), programmatic `Artisan::call()`. Declarative attributes: `#[middleware]`, `#[authorize]`, `#[tries]`, `#[backoff]`, `#[timeout]`, `#[usage]`/`#[help]`/`#[hidden]` for commands (metadata only — see M3 status). Testing: `cargo test` integration, `TestCase` harness, `testcontainers`-backed isolated Postgres fixture (`PostgresTestDb`, `crates/rustasea-testing/src/fixtures.rs:49`), `Factory::create`, `Str` factory resets between tests, paginator views. |
| **Deliverables** | `rustasea-cli` + `rustasea-macros` + `rustasea-testing` crates, `bootstrap/commands.rs` (populated — `:12`), `crates/rustasea/tests/feature/` integration suite, `cargo artisan make:test` generator, `#[test]` helpers. |
| **Success Criteria** | `cargo artisan make:controller UserController` scaffolds a controller (route registration stays manual — see status note); all generated `make:*` commands produce `rustfmt`-clean code that compiles; `cargo test` spins up an isolated Postgres via `testcontainers` and tears it down. |
| **Laravel 13 features** | #7 expanded attributes (all `#[Tries]`/`#[Backoff]`/`#[Timeout]`/`#[WithoutBroadcasting]` etc.), #20 `ModelInspector`/`route:list`/`Str` factory resets. |

> **Status (2026-09-12):** Partial. The 15 `make:*` generators, `cargo rustasea new --variant {blade|react|vue|livewire}`, real `xtask check-cycles` DAG validation, `xtask migrate`, and the testcontainers `PostgresTestDb` fixture are implemented (`crates/rustasea-cli/src/commands/mod.rs:17-31`; `crates/cargo-rustasea/src/main.rs:46`; `xtask/src/cycles.rs:32`; `xtask/src/main.rs:37`; `crates/rustasea-testing/src/fixtures.rs:49`). Remaining: Docker-gated integration tests are opt-in (`cargo test -p rustasea --features integration -- --ignored`), and generated controllers leave route registration manual (`crates/rustasea-cli/src/generators/kinds/controller.rs:32`). Tracked in `GAP-016`–`GAP-018` (completed).

### M6 — Advanced (Broadcasting, Search, Filesystem, AI SDK, Real-time)

| Field | Detail |
|---|---|
| **Goal** | Differentiate RustaSea with AI-native capabilities and complete Laravel parity on advanced features. |
| **Scope** | Broadcasting & real-time: WebSocket via `axum` (`ws` feature), channel auth, `ShouldBroadcast` trait, SSE via `Response::eventStream`. Search: `whereVectorSimilarTo` integration with embedding providers, `Str::toEmbeddings`, `dropVectorIndex`. Filesystem: read-through disk (primary + fallback with optional copy), `Storage::path()` confinement, `Storage` facade over `object_store` / local, `config/storage.toml` disk configuration. JSON:API resources: `JsonApiResource` with sparse fieldsets, relationship inclusion, links, headers. Notifications & mail: `rustasea-mail` (`Mailable`, `Mailer`, `ArrayMailer`/`LogMailer`/`SmtpMailer`, queued with `#[deleteWhenMissingModels]`). Server-side presentation: `rustasea-view` (askama default, minijinja behind `runtime-templates`), `rustasea-inertia` + `rustasea-inertia-client` + `rustasea-inertia-adapters`, `rustasea-livewire`, `rustasea-scaffold` starter kits. AI SDK (`rustasea-ai`): provider-agnostic trait with real HTTP adapters (OpenAI, Anthropic, Azure, Groq, xAI, DeepSeek, Mistral, Ollama, OpenRouter, OpenAI-compatible), `Agent` contracts, `make:agent`/`make:tool`, `SimilaritySearch`/`FileStorage`/`ToolSearch` deferred loading, sub-agents, middleware, anonymous agents, streaming + broadcasting + queueing (`AgentRunJob`), MCP client/registry. |
| **Deliverables** | `rustasea-broadcast` + `rustasea-storage` + `rustasea-search` + `rustasea-ai` + `rustasea-view` + `rustasea-inertia` + `rustasea-livewire` + `rustasea-scaffold` + `rustasea-mail` crates, `app/ai/agents/` + `app/ai/tools/`, `resources/views/`, `cargo artisan make:agent` / `make:tool`. |
| **Success Criteria** | Define an `Agent` with a `Tool`, stream its response over WebSocket, and assert the stream includes structured output; `Storage` read falls through to fallback disk and `path()` never escapes root; `JsonApiResource` renders correct `Content-Type: application/vnd.api+json` with sparse fieldsets. |
| **Laravel 13 features** | #1 AI SDK, #2 AI Agents, #3 JSON:API Resources, #6 semantic/vector search (full), #9 read-through filesystem, #17 mail/notification defaults, #18 SSE `eventStream`. |

> **Status (2026-09-12):** Partial. Broadcast WS (default `ws` feature → `axum/ws`) + SSE, `object_store`-backed storage with the `config/storage.toml` facade, JSON:API, `rustasea-mail`, the view/Inertia/Livewire/scaffold presentation crates, real HTTP AI providers (`provider_from_env()`, `crates/rustasea-ai/src/providers/mod.rs:27`), AI queueing (`crates/rustasea-ai/src/queue.rs:41`), MCP client/registry (`crates/rustasea-ai/src/mcp/client.rs:40`), and feature-gated `pgvector` (`crates/rustasea-search/src/pgvector.rs:24`) are implemented. Remaining: Gemini/Bedrock have no adapter (`AiError::UnknownProvider`, `crates/rustasea-ai/src/providers/client.rs:223`), embeddings fall back to the deterministic stub (`crates/rustasea-search/src/embeddings.rs:97`), and `InProcessProvider` remains for deterministic tests (`crates/rustasea-ai/src/adapters.rs:50`). Tracked in `GAP-012`, `GAP-014`, `GAP-015`, `GAP-020` (completed).

---

## Tech Stack

| Layer | Crate | Rationale |
|---|---|---|
| Async runtime | `tokio` | De-facto async runtime; powers `axum`, `sqlx`, `deadpool`, and queue workers. Work-stealing scheduler, `tokio::select!` for graceful shutdown. |
| HTTP | `axum` + `tower` + `tower-http` | Ergonomic, extractor-based routing; `tower` middleware composes cleanly; `tower-http` ships CORS, rate-limit, tracing, compression. Preferred over `actix-web` for `tokio` alignment and simpler ownership. |
| ORM / DB | `sqlx` | Runtime-checked queries via the `sqlx` API (no compile-time `query!` macros — CI has no `DATABASE_URL`); `pgvector` integration behind the `vector` feature. Migrations run through the custom `Migrator` (`crates/rustasea-orm/src/migration.rs:131`). |
| Connection pooling | `sqlx` built-in pool + `deadpool-redis` (Redis) | `DbPool` wraps the driver-native sqlx pool (`crates/rustasea-orm/src/db.rs:24`); `deadpool-redis` 0.21 backs the feature-gated Redis cache/queue drivers. No `bb8`. |
| Migrations | Custom `Migrator` | Versioned, reversible migrations (`run`/`rollback`/`fresh`/`seed` — `crates/rustasea-orm/src/migration.rs:189`–`:288`); `cargo artisan migrate` (plus `migrate:fresh`/`migrate:rollback`) or `cargo xtask migrate` wraps them. |
| Validation | `validator` + custom `rustasea-validation` | `validator` derive macros for struct-level rules; custom crate for `ErrorBag` + FormRequest semantics. |
| Auth | `jsonwebtoken` + `argon2` + `tower-sessions` | `jsonwebtoken` for JWT guards; `argon2` for password hashing; `tower-sessions` for session store (JSON by default). |
| Serialization | `serde` + `serde_json` | Universal; powers config, JSON:API, queue payloads, session store. |
| Config | `config` + `dotenvy` | Layered `config/*.toml` (auto-discovered: `app.toml` base first, remaining files sorted — `crates/rustasea-config/src/lib.rs:89`) + env overlay (`__` separator) + `.env` via `dotenvy`. Typed via `serde`. TOML only — no YAML. |
| CLI | `clap` (derive) + `cargo xtask` | `clap` for `cargo artisan` subcommands; `xtask` pattern avoids extra binary install. `dialoguer` / `indicatif` for prompts/progress. `cargo rustasea new` scaffolds apps (`crates/cargo-rustasea/src/main.rs:46`). |
| Proc-macros | `syn` + `quote` + `proc-macro2` | Powers `#[route]`, `#[middleware]`, `#[validate]`, `#[derive(Model)]`, `#[tries]`, etc. |
| Queue | `tokio` + `deadpool-redis` (feature-gated) + `serde_json` | `database` and `redis` drivers plus a worker loop are live (`crates/rustasea-queue/src/driver/{database,redis,worker}.rs`); the `sync` driver remains for tests. `tokio::spawn` for workers; retries use an exponential backoff loop (`crates/rustasea-queue/src/job.rs:498`) — no external `backoff` crate. |
| Cache | lock-guarded in-memory store + `deadpool-redis` (feature-gated) | `MemoryStore` is a `Mutex<HashMap>` with TTL support (`crates/rustasea-cache/src/memory.rs:35`) — moka-like but not the `moka` crate. `RedisStore` is a real `deadpool-redis` implementation behind the opt-in `redis` feature (`GAP-004`; `crates/rustasea-cache/src/redis.rs:155`; `crates/rustasea-cache/Cargo.toml:12`), wired via `RedisStore::from_url`/`from_pool` (`:91`, `:105`): real `get`/`put`/`touch`/`increment`, atomic `SET NX` insert-if-absent (`:178`) and Lua compare-and-delete lock release (`:192`); without the feature — or when Redis is unreachable — operations return a typed `StoreUnavailable` (`:278`; pool/command failures `:149`, `:349`). Both behind the `Store` trait. |
| Scheduling | Hand-rolled cron + `tokio` interval | 5-field cron parsing (`crates/rustasea-schedule/src/command.rs:129`) driven by a 60-second `tokio` ticker (`crates/rustasea-schedule/src/scheduler.rs:102`, `:168`); `schedule:list`/`run`/`pause`/`resume` commands. No `tokio-cron-scheduler`/`cron` crates. |
| Templating | `askama` (default) + `minijinja` (opt-in) | `rustasea-view`: `AskamaEngine` compiles templates at build time (`crates/rustasea-view/src/askama_engine.rs:35`); `MinijinjaEngine` behind the `runtime-templates` feature (`crates/rustasea-view/Cargo.toml:22`). |
| WebSocket / SSE | `axum` (`ws` feature) + SSE | `rustasea-broadcast` enables `axum/ws` by default (`crates/rustasea-broadcast/Cargo.toml:20`); SSE via `Response::eventStream` (`crates/rustasea-broadcast/src/lib.rs:25`). No `tokio-tungstenite`. |
| HTTP client | `reqwest` | Async HTTP client with middleware; Laravel-style `throw`/`try_throw` callbacks and typed `HttpError` implemented (`crates/rustasea-http/src/lib.rs:286-342`). |
| Filesystem | `object_store` + `tokio::fs` | `object_store` 0.14 for S3/GCS/Azure abstraction (`aws`/`gcp`/`azure` features), wrapped by `ObjectDisk` (`crates/rustasea-storage/src/manager.rs:171`); local disk via `tokio::fs`. Read-through across primary + fallback disks is implemented, and `config/storage.toml` drives `StorageManager::from_toml` (`crates/rustasea-storage/src/facade.rs:168`). |
| Vector / AI | `pgvector` (feature-gated) + real HTTP providers | `MemoryVectorStore` is the default; `PgVectorStore` executes real similarity queries behind the `pgvector` feature (`crates/rustasea-search/src/pgvector.rs:24`). AI providers make real HTTP calls via `reqwest` — `provider_from_env()` supports OpenAI, Anthropic, Azure, Groq, xAI, DeepSeek, Mistral, Ollama, OpenRouter, and OpenAI-compatible endpoints (`crates/rustasea-ai/src/providers/mod.rs:27`); Gemini/Bedrock are not yet wired. |
| Testing | `testcontainers` 0.23 + `cargo test` | Isolated Postgres via the `PostgresTestDb` fixture (`crates/rustasea-testing/src/fixtures.rs:49`); Docker-gated integration suite in `crates/rustasea/tests/feature/` runs with `cargo test -p rustasea --features integration -- --ignored`. |
| Lint / Format | `rustfmt` + `clippy` | Enforced in CI; generated code is `rustfmt`-clean. |

---

## Proposed Directory Structure

Workspace with one crate per milestone domain. Application code lives in `app/` (mirrors Laravel/Goravel conventions).

```text
rustasea/                          # workspace root
├── Cargo.toml                     # [workspace] — members = ["crates/*"]
├── rustasea.toml                  # framework config (optional)
├── .env.example
├── bootstrap/
│   ├── app.rs                     # Application::configure() — providers, routing, schedule, events
│   ├── providers.rs               # provider registry
│   └── commands.rs                # CLI command registry
├── config/
│   ├── app.toml
│   ├── database.toml
│   ├── cache.toml
│   ├── queue.toml
│   ├── storage.toml
│   └── auth.toml
├── routes/
│   └── web.rs                     # route definitions
├── database/
│   ├── migrations/
│   └── seeders/
├── resources/
│   └── views/                     # askama / minijinja templates
├── storage/
│   ├── app/
│   └── logs/
├── tests/
│   └── feature/                   # placeholder (real suite: crates/rustasea/tests/feature/)
├── docs/
│   ├── laravel-13-research.md
│   ├── laravel-parity.md
│   └── milestones.md
├── crates/
│   ├── rustasea/                  # umbrella re-export crate (like `laravel/framework`)
│   ├── rustasea-foundation/       # M0 — Application, Container, ServiceProvider DAG
│   ├── rustasea-config/           # M0 — layered config loader (auto-discovers config/*.toml)
│   ├── cargo-rustasea/            # M0 — `cargo rustasea new` app scaffolder
│   ├── rustasea-router/           # M1 — routing + route:list
│   ├── rustasea-http/             # M1 — request/response, middleware, HTTP client
│   ├── rustasea-orm/              # M2 — query builder, Model derive, migrations
│   ├── rustasea-macros/           # M2/M5 — proc-macros (Model, route, middleware, validate, tries, …)
│   ├── rustasea-auth/             # M3 — guards, JWT, session, authorize
│   ├── rustasea-validation/       # M3 — rules, ErrorBag, FormRequest
│   ├── rustasea-queue/            # M4 — jobs, routing, workers, failed_jobs
│   ├── rustasea-cache/            # M4 — Store trait, memory + redis, Lock, touch
│   ├── rustasea-events/           # M4 — Event/Listener, dispatchAfterResponse
│   ├── rustasea-schedule/         # M4 — scheduler, pause/resume, frequencies
│   ├── rustasea-cli/              # M5 — clap CLI, make:* generators (cargo-artisan binary)
│   ├── rustasea-testing/          # M5 — TestCase, factories, testcontainers PostgresTestDb
│   ├── rustasea-broadcast/        # M6 — WebSocket (axum `ws`), SSE, channel auth
│   ├── rustasea-storage/          # M6 — Storage facade, read-through disks, config/storage.toml
│   ├── rustasea-search/           # M6 — vector search, embeddings, pgvector store
│   ├── rustasea-ai/               # M6 — provider trait + HTTP adapters, Agent, Tool, MCP, queueing
│   ├── rustasea-jsonapi/          # M6 — JSON:API resources, sparse fieldsets
│   ├── rustasea-mail/             # M6 — Mailable, Mailer (array/log/smtp), queued notifications
│   ├── rustasea-view/             # M6 — askama (default) + minijinja (runtime-templates)
│   ├── rustasea-inertia/          # M6 — Inertia server protocol
│   ├── rustasea-inertia-client/   # M6 — Inertia WASM client protocol
│   ├── rustasea-inertia-adapters/ # M6 — React→Dioxus / Vue→Leptos WASM adapters
│   ├── rustasea-livewire/         # M6 — Livewire analogue (askama components, HTMX swaps)
│   ├── rustasea-scaffold/         # M6 — starter-kit variants (blade/react/vue/livewire)
│   └── rustasea-app/              # runnable example app (`cargo run -p rustasea-app`)
├── xtask/                         # workspace dev tasks (ci, fmt, clippy, check-cycles, migrate)
└── app/                           # application layer (generated by `cargo rustasea new`)
    ├── http/
    │   ├── controllers/
    │   └── middleware/
    ├── models/
    ├── providers/
    ├── console/
    │   └── commands/
    ├── jobs/
    ├── events/
    ├── listeners/
    ├── ai/
    │   ├── agents/
    │   └── tools/
    └── grpc/                      # optional — mirrors Goravel grpc support
```

---

The [canonical crate inventory](.agents/documents/application/modules/manifest.md#canonical-crate-inventory-source-of-truth)
records **21 crates under `crates/` + `xtask` (22 workspace packages total)**; the
current workspace tree contains **29 crates under `crates/` + `xtask`
(30 packages)** — the inventory count is stale and is flagged for reconciliation
under `GAP-021`. The directory tree above mirrors the current tree; `Cargo.toml`
(`members = ["crates/*", "xtask"]`) is the mechanical source of truth.

## Roadmap

| Milestone | Focus | Target | Depends On |
|---|---|---|---|
| **M0** | Bootstrap & Core | Q4 2026 | — |
| **M1** | Routing & HTTP | Q4 2026 – Q1 2027 | M0 |
| **M2** | ORM & Database | Q1 2027 | M0, M1 |
| **M3** | Auth, Middleware & Validation | Q1 – Q2 2027 | M1, M2 |
| **M4** | Queue, Cache, Scheduling & Events | Q2 2027 | M0, M2, M3 |
| **M5** | DX, CLI & Testing | Q2 – Q3 2027 | M0 – M4 |
| **M6** | Advanced (Broadcast, Search, AI SDK, Real-time) | Q3 – Q4 2027 | M1 – M5 |

> Timeline is aspirational and will be refined after M0 ships. Each milestone ships as a tagged release with migration notes.

```mermaid
gantt
    title RustaSea Roadmap
    dateFormat YYYY-MM-DD
    section Core
    M0 Bootstrap & Core          :m0, 2026-10-01, 2026-12-31
    M1 Routing & HTTP            :m1, 2026-11-15, 2027-02-15
    section Data
    M2 ORM & Database            :m2, 2027-01-01, 2027-03-31
    M3 Auth & Validation         :m3, 2027-02-15, 2027-05-15
    section Async
    M4 Queue/Cache/Schedule/Event :m4, 2027-04-01, 2027-06-30
    section DX
    M5 CLI & Testing             :m5, 2027-05-15, 2027-08-31
    M6 Advanced (AI/Broadcast)   :m6, 2027-07-01, 2027-12-31
```

---

## Contributing

> Early stage — M2 (ORM & Database) is complete; M0, M1, and M3–M6 are partial. See [`docs/milestones.md`](docs/milestones.md) for the authoritative, evidence-backed status. Contributions to research, RFCs, and prototype crates are welcome.

1. **Read the research** — [`docs/laravel-13-research.md`](docs/laravel-13-research.md) and `TASK-002` Goravel study.
2. **Pick a milestone** — check the [Issues](https://github.com/vheins/rustasea/issues) for `milestone:M0` … `milestone:M6` labels.
3. **Open an RFC** — for any cross-crate design decision, open a discussion/issue before coding.
4. **Conventions** — `rustfmt` + `clippy -- -D warnings` must pass; generated code must be `rustfmt`-clean; workspace `Cargo.toml` is the source of truth for versions.

```bash
# local setup
cargo xtask ci       # fmt + clippy + check-cycles
cargo xtask migrate  # run migrations
cargo test --workspace
```

The full xtask surface is `ci`, `fmt`, `clippy`, `check-cycles` (real workspace
DAG cycle detection — `xtask/src/cycles.rs:32`), and `migrate`
(`xtask/src/main.rs:22-43`).

Questions? Open a [Discussion](https://github.com/vheins/rustasea/discussions) or reach out via Issues.

---

## License

Licensed under the [MIT License](LICENSE-MIT).

---

*Inspired by [Laravel](https://laravel.com) and [Goravel](https://goravel.dev). Not affiliated with either project.*
