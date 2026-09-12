# RustaSea Milestone Status

> **Last updated:** 2026-09-12
> **Scope:** Authoritative done / partial / missing status for milestones **M0–M6**, plus the **gap-closure program** progress (P0–P5).

## Purpose

`README.md` defines the goal, scope, deliverables, and success criteria for each
milestone, but it does not track what is actually implemented. This document is
the single source of truth for **milestone status**, grounded in the current
source tree. Every status claim cites a real `path:line` and/or a task ID so
it can be re-verified.

## Status Legend

| Status | Meaning |
|---|---|
| **Done** | All deliverables for the milestone are implemented and reachable from real code paths. |
| **Partial** | A meaningful subset is implemented and usable; some deliverables are stubs, placeholders, or unwired. |
| **Missing** | No implementation beyond contracts/placeholders, or the deliverable does not exist. |
| **N/A** | Not applicable at this stage (e.g. a milestone explicitly deferred). |

A milestone is **Partial** when its core surface exists but at least one named
deliverable or success criterion is not yet met. The "Partial" list on each
section records the reason.

## Sources

- Current source tree at the repository root (workspace `Cargo.toml:2` — `members = ["crates/*", "xtask"]`), read directly to verify every claim below.
- P0 gap-closure work: `GAP-001` (sqlx pool, async execution, transactions) and `GAP-002` (router → controller dispatch).
- Gap-closure program completions: `GAP-004` (Redis cache store), `GAP-005`–`GAP-006` (queue drivers, worker, failed jobs), `GAP-007` (session guard), `GAP-008`–`GAP-009` (async listeners, metrics), `GAP-010`–`GAP-013` (migrations, CRUD, pgvector, eager loading), `GAP-014`–`GAP-015` (AI HTTP providers, AI queueing + MCP), `GAP-016`–`GAP-018` (CLI generators, testcontainers, xtask), `GAP-019`–`GAP-020` (config/DAG/bootstrap, templating + mail + storage facade), `GAP-021` (this documentation sync).
- Commit-reference policy: [`docs/documentation-conventions.md`](documentation-conventions.md) — milestones, task IDs, and `path:line` only; no raw commit SHAs.
- Task registry: `DOC-001`, `DOC-002`, `DOC-ROOT`, `GAP-ROOT`, `GAP-P0`…`GAP-P5`.
- Prior research: [`docs/laravel-13-research.md`](laravel-13-research.md).
- Workspace inventory: **29 crates under `crates/` + `xtask` (30 workspace packages total)**, per `Cargo.toml:2`. The [canonical crate inventory](../.agents/documents/application/modules/manifest.md#canonical-crate-inventory-source-of-truth) still records the older **21 + `xtask`** count; that drift is flagged for reconciliation under `GAP-021`, with the workspace `members` glob as the mechanical source of truth.

---

## Summary Matrix

| Milestone | Goal | Status | Headline evidence |
|---|---|---|---|
| **M0** | Bootstrap & Core | **Partial** | Container, provider DAG, and config auto-discovery real (`crates/rustasea-foundation/src/lib.rs:275`, `:296`; `crates/rustasea-config/src/lib.rs:89`); bootstrap registries populated (`bootstrap/providers.rs:35`, `bootstrap/commands.rs:12`) but `AppServiceProvider` stays a no-op (`bootstrap/providers.rs:17`) and the config loader is not mounted in app boot |
| **M1** | Routing & HTTP | **Partial** | Router DSL + controller dispatch real (`crates/rustasea-router/src/dispatch.rs:23`, `:62`); `route:list` prints an empty table (`crates/rustasea-cli/src/commands/inspect.rs:33`) and the idle timeout is declared but unenforced (`crates/rustasea-http/src/lib.rs:364`) |
| **M2** | ORM & Database | **Done** | Real sqlx pool + async execution (`crates/rustasea-orm/src/db.rs:24`, `db/exec.rs:72`); model CRUD (`model_ops.rs:28`), transactions (`tx.rs:67`), custom `Migrator` (`migration.rs:131`, `:189`), eager loading (`eager.rs:59`), pgvector (`vector.rs:100`); `raw`/`raw_sql` remain display-only fragments (`execution.rs:239`, `:247`) |
| **M3** | Auth, Middleware & Validation | **Partial** | JWT/CSRF/throttle/validation real; session guard now real via `tower-sessions` (`crates/rustasea-auth/src/session.rs:160`, login `:279`, parse `:326`, refresh `:347`, logout `:375`); declarative attributes still emit metadata only (`crates/rustasea-macros/src/lib.rs:74`, `:111`) with no runtime consumer in the tree (see P0 note) |
| **M4** | Queue, Cache, Scheduling & Events | **Partial** | Database/Redis queue drivers + worker + persistent failed jobs real (`crates/rustasea-queue/src/driver/database.rs:36`, `driver/worker.rs:75`); `queue:work`/`queue:failed`/`queue:retry` CLI real (`crates/rustasea-cli/src/commands/queue.rs:18`, `ops.rs:21`, `:72`); Redis cache store real behind the `redis` feature (`crates/rustasea-cache/src/redis.rs:155`); async listeners enqueue (`crates/rustasea-events/src/dispatcher.rs:82`); remaining: `SyncDriver` in-memory failed jobs (`crates/rustasea-queue/src/driver.rs:171`) and unconsumed declarative job metadata |
| **M5** | DX, CLI & Testing | **Partial** | CLI + 15 `make:*` generators real (`crates/rustasea-cli/src/commands/mod.rs:17-31`); `cargo rustasea new --variant` real (`crates/cargo-rustasea/src/main.rs:46`); real `xtask` cycle detection (`xtask/src/cycles.rs:32`) and `xtask migrate` (`xtask/src/main.rs:37`); testcontainers-backed `PostgresTestDb` (`crates/rustasea-testing/src/fixtures.rs:49`) with a Docker-gated suite (`crates/rustasea/tests/feature/`); remaining: integration tests are opt-in and generated controllers still need manual route registration (`crates/rustasea-cli/src/generators/kinds/controller.rs:32`) |
| **M6** | Advanced (Broadcast, Search, FS, AI) | **Partial** | Broadcast WS/SSE, `object_store` storage + `config/storage.toml` facade, JSON:API, `rustasea-mail`, `rustasea-view`/`rustasea-inertia`/`rustasea-livewire`/`rustasea-scaffold`, real AI HTTP providers (`crates/rustasea-ai/src/providers/mod.rs:27`), AI queueing (`crates/rustasea-ai/src/queue.rs:41`), MCP client (`crates/rustasea-ai/src/mcp/client.rs:40`), and feature-gated pgvector (`crates/rustasea-search/src/pgvector.rs:24`) all real; remaining: Gemini/Bedrock unsupported (`crates/rustasea-ai/src/providers/client.rs:223`), deterministic stub embeddings (`crates/rustasea-search/src/embeddings.rs:97`), and `InProcessProvider` kept for tests (`crates/rustasea-ai/src/adapters.rs:50`) |

---

## M0 — Bootstrap & Core

**Goal recap:** Bootable application skeleton with typed config, service
container, provider lifecycle, and graceful shutdown.

**Status: Partial** — the application boots, resolves container bindings, orders
providers by a real dependency DAG, loads all `config/*.toml`, and shuts down
gracefully. `AppServiceProvider` remains a no-op scaffold and the config loader
is not mounted in the application boot path.

**Done**
- Service container with `bind` / `singleton` / `instance` / `get` — `crates/rustasea-foundation/src/lib.rs:103`, `:114`, `:125`, `:136`.
- `Application::boot` runs register → boot with a real topological sort — `crates/rustasea-foundation/src/lib.rs:275`, `:296`.
- Provider dependency declaration + typed cycle detection — `crates/rustasea-foundation/src/lib.rs:27` (`ServiceProvider::dependencies`), `:187` (`BootError::DependencyCycle`), `:311`/`:338` (cycle extraction); tests `crates/rustasea-foundation/tests/boot_dag.rs`.
- Graceful shutdown on `SIGTERM`/`SIGINT` — `crates/rustasea-foundation/src/lib.rs:426`, `:437-452`.
- Config loader auto-discovers all `config/*.toml` with TOML + `.env` overlay (`__` separator, TOML only) — `crates/rustasea-config/src/lib.rs:32`, `:40`, `:57`, `:89`.
- Populated bootstrap registries — `bootstrap/providers.rs:35` (provider vec), `bootstrap/commands.rs:12` (`register_default()` loads the CLI command registry); mirror in `crates/rustasea-app/src/bootstrap/{providers.rs:33,commands.rs:13}`.
- Project scaffolder `cargo rustasea new <app> --variant {blade|react|vue|livewire}` — `crates/cargo-rustasea/src/main.rs:46`, `:64-74`; starter kits in `crates/rustasea-scaffold/src/variant.rs:13`.
- Runnable app boots, registers routes, serves real handlers, and shuts down — `crates/rustasea-app/src/main.rs` (`configure()`, `app.shutdown()`).

**Partial (reason)**
- `AppServiceProvider` is a no-op scaffold — `bootstrap/providers.rs:17`; it participates in the DAG but registers/boots nothing.
- The config loader is not mounted in the application boot path — the only consumers are the CLI and xtask (`crates/rustasea-cli/src/commands/ops/migration.rs:41`, `xtask/src/migrate.rs:21`).
- No app-level service bindings beyond the scaffold.

**Missing**
- Real `AppServiceProvider` bindings (database, cache, queue) wired at boot.

**Evidence:** `crates/rustasea-foundation/src/lib.rs:27-452`; `crates/rustasea-config/src/lib.rs:32-89`; `bootstrap/{app.rs,providers.rs,commands.rs}`; `crates/rustasea-app/src/main.rs`; tasks `GAP-001`, `GAP-019`.

**Next actions**
- Populate `AppServiceProvider` with real container bindings.
- Mount the config loader in the app boot sequence.

---

## M1 — Routing & HTTP

**Goal recap:** Expressive HTTP layer with routing, middleware, request/response
ergonomics, and an HTTP client.

**Status: Partial** — the router DSL and real controller dispatch landed in
GAP-002, and the HTTP client `throw` semantics are implemented, but
CLI introspection (`route:list`, `show:model`) and idle-timeout enforcement are
not wired.

**Done**
- Router DSL: `get`/`post`/`put`/`delete`/`patch`/`options`/`any`, `group`, prefix/name/domain/resource — `crates/rustasea-router/src/router.rs:46` (`prefix`), `:61` (`domain`), `:73` (`get`), `:288` (`group`), `:317` (`resource`).
- Controller dispatch to real handlers (`GAP-002`) — `crates/rustasea-router/src/dispatch.rs:23` (`into_axum_router`), `:62` (`resolve`).
- `#[route]` metadata consumed at registration — `crates/rustasea-router/src/router.rs:218` (`route_meta`); route table introspection surface — `:358` (`get_routes`).
- HTTP client `throw` / `try_throw` callbacks and typed `HttpError` — `crates/rustasea-http/src/lib.rs:286`, `:298`, `:207`.
- Runnable app serves real handlers from `routes/web.rs` (welcome page rendered through `rustasea::view::MinijinjaEngine`).

**Partial (reason)**
- `route:list` renders an **empty** table with a "until the router is wired" comment — `crates/rustasea-cli/src/commands/inspect.rs:33`.
- `show:model` emits placeholder output — `crates/rustasea-cli/src/commands/inspect.rs:67`.
- HTTP idle (inter-byte) timeout is declared but not enforced — `crates/rustasea-http/src/lib.rs:241`, `:364-366`.

**Missing**
- Real route-table introspection feeding `route:list` with middleware + binding fields.

**Evidence:** `crates/rustasea-router/src/router.rs:46-358`; `crates/rustasea-router/src/dispatch.rs:23-83`; `crates/rustasea-http/src/lib.rs:241-379`; `crates/rustasea-cli/src/commands/inspect.rs:33`, `:67`; task `GAP-002` (completed).

**Next actions**
- Wire `route:list` to the live router registry (`inspect.rs`).
- Implement the `show:model` inspector.
- Enforce `TimeoutKind::Idle` in the HTTP client.

---

## M2 — ORM & Database

**Goal recap:** Fluent, type-safe database layer with query builder,
migrations, seeders, factories, and vector support.

**Status: Done** — the fluent SQL builder, real sqlx pool, async query
execution, model CRUD, transactions, migrations/seeders/factories, eager
loading, and pgvector support all execute through the runtime `sqlx` API. The
ORM is sqlx-based end to end (no `sea-orm`, no `sqlx::migrate!` — migrations use
the crate's own `Migrator`); `raw`/`raw_sql` remain display-only fragments.
Tracked by `GAP-001`, `GAP-010`–`GAP-013`.

**Done**
- Fluent SQL builder — `crates/rustasea-orm/src/builder.rs`, `crates/rustasea-orm/src/clause.rs`; async execution `crates/rustasea-orm/src/builder/exec.rs:105` (`get`), `:115` (`first`), `:148` (`paginate`).
- Real sqlx pool + connection foundation — `crates/rustasea-orm/src/db.rs:24` (`DbPool`), `:43` (`connect`), `:70` (`ping`), `:91` (`close`); driver dispatch — `crates/rustasea-orm/src/db/exec.rs:72` (`fetch_json`), `:97` (`execute_bind`), `:118` (`execute_script`).
- Async model operations — `crates/rustasea-orm/src/model_ops.rs:28` (`create`), `:47` (`save` upsert), `:64` (`update`), `:78` (`delete`), `:87` (`force_delete`), `:100` (`soft_delete`), `:118` (`refresh`), `:133` (`first_for_update`).
- Real transactions + executor dispatch — `crates/rustasea-orm/src/tx.rs:67` (`begin`), `crates/rustasea-orm/src/execution.rs:325` (`transaction`).
- Real migrations, seeders, and factory state — custom `Migrator` at `crates/rustasea-orm/src/migration.rs:131`, `:189` (`run`), `:240` (`rollback`), `:279` (`fresh`), `:288` (`seed`); `crates/rustasea-orm/src/factory.rs`.
- Eager loading + relation serde round-trip — `crates/rustasea-orm/src/eager.rs:18` (`EagerPlan`), `:59` (`eager_load`), `crates/rustasea-orm/src/relations.rs:22`.
- pgvector support (native bind/encode behind the `vector` feature) — `crates/rustasea-orm/src/vector.rs:100` (`to_vector_literal`), `:141` (`has_extension_sql`), `:149` (`vector_param`); feature in `crates/rustasea-orm/Cargo.toml`.
- Populated `config/database.toml` (driver/url/pool settings).

**Partial (reason)**
- `raw` / `raw_sql` emit statement fragments for display only — `crates/rustasea-orm/src/execution.rs:239`, `:247`.

**Missing**
- Compile-time `query!` macros (intentionally excluded: CI has no `DATABASE_URL`).

**Evidence:** `crates/rustasea-orm/src/db.rs:24-130`; `crates/rustasea-orm/src/db/exec.rs:72-138`; `crates/rustasea-orm/src/model_ops.rs:28-143`; `crates/rustasea-orm/src/tx.rs:67-192`; `crates/rustasea-orm/src/migration.rs:131-296`; `crates/rustasea-orm/src/eager.rs:18-59`; `crates/rustasea-orm/src/vector.rs:100-152`; tasks `GAP-001`, `GAP-010`–`GAP-013`.

**Next actions**
- Wire `route:list` / `show:model` ORM introspection (M1).
- Extend pgvector index management surfaces (M6).

---

## M3 — Auth, Middleware & Validation

**Goal recap:** Complete auth, authorization, and validation with hardened
security defaults.

**Status: Partial** — JWT, CSRF, throttling, and validation are real, and the
session guard is now a working `tower-sessions`-backed guard (GAP-007);
declarative attributes still emit metadata consts with no runtime consumer in
the tree.

**Done**
- JWT guard — `crates/rustasea-auth/src/jwt.rs`.
- Origin-aware CSRF protection (`Sec-Fetch-Site`) — `crates/rustasea-auth/src/csrf.rs`.
- Rate limiter / throttle — `crates/rustasea-auth/src/throttle/`.
- Validation crate (rules, ErrorBag, form requests) — `crates/rustasea-validation/`.
- Guard manager with typed `GuardMismatch` errors — `crates/rustasea-auth/src/guard.rs:197`, `:302`.
- Real session guard over `tower-sessions` (`GAP-007`) — `crates/rustasea-auth/src/session.rs:160` (`SessionGuard<S: SessionStore = MemoryStore>`), `:274` (`impl Guard`), `:279` (`login`, fresh session id on login), `:307` (`login_using_id`, opt-in via `:211`), `:326` (`parse`), `:347` (`refresh`, id cycling), `:375` (`logout`, flushes the store), `:394` (`user`), `:409` (`id`); dependency `crates/rustasea-auth/Cargo.toml:20`.

**Partial (reason)**
- Default session store is in-memory (`MemoryStore`); a shared store (Redis/SQLx) must be injected via `SessionGuard::with_store` (`crates/rustasea-auth/src/session.rs:186`).
- Default user lookup is the fail-closed `StaticLookup` (`crates/rustasea-auth/src/session.rs:193`); replace via `with_lookup` (`:205`) where a real user source exists.
- `#[authorize]` / `#[middleware]` emit metadata consts that nothing consumes at runtime — `crates/rustasea-macros/src/lib.rs:74`, `:111`, `:247`–`:265`.

**Missing**
- Runtime enforcement of `#[middleware]` / `#[authorize]` / `#[tries]` / `#[backoff]` / `#[timeout]` in the tree (see the P0 note below on `GAP-003`).

**Evidence:** `crates/rustasea-auth/src/{jwt,csrf,guard,session}.rs`; `crates/rustasea-auth/Cargo.toml:20`; `crates/rustasea-macros/src/lib.rs:74-265`; tasks `GAP-003`, `GAP-007`.

**Next actions**
- Add the attribute registration + runtime consumer (`GAP-003`).
- Provide a store-backed default session store for multi-process deployments.

---

## M4 — Queue, Cache, Scheduling & Events

**Goal recap:** Async workloads, caching, scheduling, and event dispatch with
observable queue metrics.

**Status: Partial** — in-memory cache, the sync queue driver, inline events, and
the scheduler exist; the gap-closure program added real `database`/`redis` queue
drivers with a worker loop, persistent failed jobs, `queue:work`/
`queue:failed`/`queue:retry` CLI, a real feature-gated Redis cache store, and
queue-backed async listeners. Remaining: the in-process `SyncDriver` still
tracks failed jobs in memory, and declarative job metadata is not consumed.

**Done**
- In-memory cache store (lock-guarded map; no `moka` dependency) — `crates/rustasea-cache/src/memory.rs:35`.
- Real feature-gated Redis cache store (`GAP-004`) — `crates/rustasea-cache/src/redis.rs:62` (`RedisStore`), `:155` (full `Store` impl over `deadpool-redis`); feature `crates/rustasea-cache/Cargo.toml:12`; without the feature every op returns a typed `StoreUnavailable` (`:278`).
- Synchronous queue driver — `crates/rustasea-queue/src/driver.rs:115`.
- Real `database` queue driver (`GAP-005`) — `crates/rustasea-queue/src/driver/database.rs:36` (`push`/`pop`/`ack`/`release`/`dead_letter` over a `jobs` table), with DB-backed failed jobs at `:23` (`FAILED_JOBS_TABLE`), `:71` (`failed_jobs`), `:95` (`retry_failed`).
- Real `redis` queue driver (feature-gated) — `crates/rustasea-queue/src/driver/redis.rs:33` (`RedisDriver`), `:43` (`from_url`; disabled when no URL), `:329` (`dead_letter`).
- Queue worker loop + handler registry (`GAP-006`) — `crates/rustasea-queue/src/driver/worker.rs:75` (`run_worker`), `:36` (`register_job`).
- Queue migrations for `jobs`/`failed_jobs` — `crates/rustasea-queue/src/migrations.rs:16`, `:48`, `:82`, `:90`; `queue:work` CLI — `crates/rustasea-cli/src/commands/queue.rs:18`; `queue:failed`/`queue:retry` CLI — `crates/rustasea-cli/src/commands/ops.rs:21`, `:72`.
- Inline event dispatch + `dispatchAfterResponse` — `crates/rustasea-events/src/dispatcher.rs:182`; async listeners enqueue a `ListenerJob` through the queue facade (`GAP-008`) — `crates/rustasea-events/src/dispatcher.rs:82-100`, `crates/rustasea-events/src/job.rs:15`.
- Scheduler with pause/resume — `crates/rustasea-schedule/src/lib.rs:20` (`SchedulePaused`/`ScheduleResumed` re-exports).

**Partial (reason)**
- The in-process `SyncDriver` still tracks failed jobs in a `OnceLock<Mutex<Vec<FailedJob>>>` — `crates/rustasea-queue/src/driver.rs:171`.
- Declarative job attributes (`#[tries]`/`#[backoff]`/`#[timeout]`) emit metadata only; job-declared retry/timeout loops exist (`crates/rustasea-queue/src/job.rs:498`), but attribute metadata has no runtime consumer (see the P0 note below).

**Missing**
- Attribute-driven policy enforcement for queue jobs (GAP-003 consumer).

**Evidence:** `crates/rustasea-cache/src/{memory,redis}.rs`; `crates/rustasea-queue/src/driver.rs:115-197`; `crates/rustasea-queue/src/driver/{database,redis,worker}.rs`; `crates/rustasea-queue/src/migrations.rs`; `crates/rustasea-events/src/dispatcher.rs:82-182`; `crates/rustasea-schedule/src/lib.rs:20`; tasks `GAP-004`–`GAP-009`.

**Next actions**
- Move `SyncDriver` failed-job tracking to a persistent sink (or delegate to `DatabaseDriver`).
- Add the attribute-driven job policy consumer (`GAP-003`).

---

## M5 — DX, CLI & Testing

**Goal recap:** First-class CLI, code generation, and a Laravel-like testing
story.

**Status: Partial** — the CLI and 15 `make:*` generators are real (including
`make:middleware`/`make:request`), `cargo rustasea new --variant` scaffolds real
starter kits, `xtask` has real cycle detection plus `migrate`, and the testing
stack uses real `testcontainers` with a Docker-gated feature suite. Remaining:
the integration suite is opt-in, and generated controllers still require manual
route registration.

**Done**
- `cargo artisan` CLI with command registry — `crates/rustasea-cli/src/registry.rs`, `crates/rustasea-cli/src/lib.rs`.
- 15 `make:*` generators: controller, middleware, request, model, provider, command, job, event, listener, observer, test, seeder, migration, agent, tool — registrations `crates/rustasea-cli/src/commands/mod.rs:17-31`; kinds `crates/rustasea-cli/src/generators/kinds/{middleware,request}.rs:17` (`GAP-016`).
- Typed command args/flags and prompt/table helpers.
- Project scaffolder `cargo rustasea new <app> --variant {blade|react|vue|livewire}` — `crates/cargo-rustasea/src/main.rs:46`, `:64-74`; starter kits `crates/rustasea-scaffold/src/variant.rs:13`; `cargo artisan` remains a legacy alias via `normalize_args` (`:103`).
- Real `xtask` cycle detection (`GAP-018`) — `xtask/src/cycles.rs:32` (`run()`, DFS over `cargo metadata`); task dispatch `xtask/src/main.rs:23` (`ci`), `:24` (`fmt`), `:25` (`clippy`), `:36` (`check-cycles`), `:37` (`migrate`).
- `cargo xtask migrate` — `xtask/src/migrate.rs` via `xtask/src/main.rs:37`.
- Real testcontainers-backed fixtures (`GAP-017`) — `crates/rustasea-testing/src/fixtures.rs:49` (`PostgresTestDb`), `:63` (`start`); feature `crates/rustasea-testing/Cargo.toml:10`; integration suite `crates/rustasea/tests/feature/` gated behind the `integration` feature (`crates/rustasea/Cargo.toml:86`) and `#[ignore = "requires docker"]` (`crates/rustasea/tests/feature/route_to_db.rs:81`).

**Partial (reason)**
- Generated controllers leave route registration manual — `crates/rustasea-cli/src/generators/kinds/controller.rs:32` ("Register routes against these handlers in `routes/web.rs`").
- Generator smoke tests lock wiring only (output path, non-empty source, `--force`); compile/rustfmt cleanliness is exercised separately against an app fixture — `crates/rustasea-cli/tests/make_generators.rs:1-9`.
- Docker-backed integration tests are opt-in: `cargo test -p rustasea --features integration -- --ignored`.

**Missing**
- Generator-to-route-registration automation.

**Evidence:** `crates/rustasea-cli/src/commands/mod.rs:17-31`; `crates/rustasea-cli/src/commands/builtins.rs`; `crates/cargo-rustasea/src/main.rs:46-103`; `xtask/src/{main.rs:23-37,cycles.rs:32,migrate.rs}`; `crates/rustasea-testing/src/fixtures.rs:49`; `crates/rustasea/tests/feature/`; tasks `GAP-016`–`GAP-018`.

**Next actions**
- Automate route registration in generated controllers (or a route-table generator).
- Keep the integration suite green in CI with Docker enabled.

---

## M6 — Advanced (Broadcasting, Search, Filesystem, AI SDK, Real-time)

**Goal recap:** AI-native capabilities and full Laravel parity on advanced
features.

**Status: Partial** — broadcasting (WS/SSE), `object_store`-backed storage with a
TOML config facade, JSON:API, the mail crate, the presentation crates
(view/Inertia/Livewire/scaffold), real HTTP-backed AI providers, AI queueing, and
MCP are all real. Remaining: Gemini/Bedrock are unsupported, embeddings default
to a deterministic stub, and the in-process AI provider is kept for tests.

**Done**
- Broadcasting: WebSocket (`axum/ws`, feature-gated default) + SSE + `ShouldBroadcast` — `crates/rustasea-broadcast/Cargo.toml:20`, `:22`; `crates/rustasea-broadcast/src/lib.rs:35`.
- Filesystem on real `object_store` (0.14.1) with cloud features — `crates/rustasea-storage/Cargo.toml:10`, `:20-22`; `crates/rustasea-storage/src/manager.rs:56` (`StorageManager`), `:127` (`get` read-through), `:144` (`put`), `:171` (`ObjectDisk`).
- Storage config facade (`GAP-020`) — `config/storage.toml` (`[storage] default = "local"`), `crates/rustasea-storage/src/facade.rs:36` (`StorageFacadeConfig::from_toml`), `:168` (`StorageManager::from_toml`).
- JSON:API resources with correct content type — `crates/rustasea-jsonapi/src/wire.rs:7`.
- Mail & notifications (`GAP-020`) — `crates/rustasea-mail/`: `Mailable` (`mailable.rs:11`), `Mailer`/`ArrayMailer`/`LogMailer` (`mailer.rs:12`, `:22`, `:73`), `SmtpMailer` (feature `smtp`, `smtp.rs:14`), `QueuedNotification` dispatched through the queue (`notification.rs:41`).
- View layer — `rustasea-view` with askama (default) + minijinja (`runtime-templates`) — `crates/rustasea-view/Cargo.toml:14`, `:17`, `:22`; `crates/rustasea-view/src/askama_engine.rs:35`.
- Inertia protocol + WASM client + framework adapters — `crates/rustasea-inertia/src/page.rs:25` (`Page<T>`), `crates/rustasea-inertia-client/`, `crates/rustasea-inertia-adapters/`.
- Livewire analogue — `crates/rustasea-livewire/src/authorizer.rs:27` (`ActionAuthorizer`).
- Starter-kit scaffolder — `crates/rustasea-scaffold/src/variant.rs:13`.
- Real AI HTTP providers (`GAP-014`) — `crates/rustasea-ai/src/providers/mod.rs:27` (`provider_from_env`), `openai.rs:21` (`OpenAiProvider`), `anthropic.rs:18` (`AnthropicProvider`); config resolution for openai/anthropic/groq/xai/deepseek/mistral/openrouter/ollama/azure (`providers/client.rs:82`).
- AI queueing (`GAP-015`) — `crates/rustasea-ai/src/queue.rs:41` (`AgentRunJob`), routed on connection `ai` / queue `agents`.
- MCP client (`GAP-015`) — `crates/rustasea-ai/src/mcp/client.rs:40` (`McpClient`: connect/list/call), `crates/rustasea-ai/src/mcp/mod.rs:99` (`McpRegistry`).
- pgvector-backed vector store (`GAP-012`) — `crates/rustasea-search/src/pgvector.rs:24` (`PgVectorStore`, feature `pgvector`); in-memory store remains the default.

**Partial (reason)**
- Gemini/Bedrock are not wired — `provider_from_env` rejects unknown names with `AiError::UnknownProvider` (`crates/rustasea-ai/src/providers/client.rs:223`).
- Default embeddings are a deterministic hashing stub — `crates/rustasea-search/src/embeddings.rs:97` (`stub_embedding`, model `"stub"`).
- `InProcessProvider` remains for deterministic tests — `crates/rustasea-ai/src/adapters.rs:50`.
- `rustasea-mail` is not yet re-exported from the `rustasea` umbrella crate (`crates/rustasea/Cargo.toml`); use it as a direct workspace dependency for now.

**Missing**
- Gemini/Bedrock adapters.
- Real embedding-model integration (provider-backed `to_embeddings`).

**Evidence:** `crates/rustasea-broadcast/{Cargo.toml,src/lib.rs}`; `crates/rustasea-storage/{Cargo.toml,src/manager.rs:56-171,src/facade.rs}`; `config/storage.toml`; `crates/rustasea-jsonapi/src/wire.rs:7`; `crates/rustasea-mail/`; `crates/rustasea-view/`; `crates/rustasea-ai/src/{providers,queue.rs,mcp,adapters.rs}`; `crates/rustasea-search/{src/pgvector.rs,src/embeddings.rs}`; tasks `GAP-012`, `GAP-014`, `GAP-015`, `GAP-020`.

**Next actions**
- Add Gemini/Bedrock adapters behind the existing provider abstraction.
- Replace the stub embedding path with a real provider call.

---

## P0 Progress (Foundation Unblockers)

**Parent:** `GAP-P0` — *P0 — Foundation unblockers (DB backend + router dispatch)* — status **completed** (children below).

| Task | Title | Status | Evidence |
|---|---|---|---|
| **GAP-001** | Wire sqlx DB backend + connection pool into `rustasea-orm` | **Done** | `crates/rustasea-orm/src/db.rs:24`, `crates/rustasea-orm/src/db/exec.rs:72` |
| **GAP-002** | Router → controller dispatch (real handler binding) | **Done** | `crates/rustasea-router/src/dispatch.rs:23-83` |
| **GAP-003** | Runtime consumer for declarative attributes (`#[middleware]`, `#[authorize]`, `#[tries]`, `#[backoff]`, `#[timeout]`) | **Registry: completed — partial in tree** | Job-declared retry/timeout loops are real (`crates/rustasea-queue/src/job.rs:498`, `crates/rustasea-queue/src/driver/worker.rs:154`); attribute metadata consts still have no router/auth consumer in the tree (`crates/rustasea-macros/src/lib.rs:74`, `:111`, `:247`–`:265`) |

**Other gap phases (all completed):** `GAP-004`–`GAP-009` (P1), `GAP-010`–`GAP-013`
(P2), `GAP-014`–`GAP-015` (P3), `GAP-016`–`GAP-018` (P4), `GAP-019`–`GAP-020`
(P5); `GAP-021` (P5, this documentation sync) in progress. See `GAP-ROOT` for
the full program.

> **Note (2026-09-12):** `GAP-P0` closed with `GAP-001`/`GAP-002` verified in the
> tree. `GAP-003` is marked completed in the task registry, but this document
> records what the current tree actually contains: the queue retry/timeout loop
> is real, while `#[middleware]`/`#[authorize]` attribute metadata still has no
> runtime consumer. This discrepancy is flagged for reconciliation under
> `GAP-021`.

---

## Related Documents

- [`README.md`](../README.md) — milestone goals, scope, deliverables, success criteria.
- [`docs/laravel-13-research.md`](laravel-13-research.md) — Laravel 13 feature research.
- [`docs/laravel-parity.md`](laravel-parity.md) — Laravel 13.x API adoption mapping (maintained separately).
- [`docs/documentation-conventions.md`](documentation-conventions.md) — commit-reference policy (milestone + date, no raw SHAs).
