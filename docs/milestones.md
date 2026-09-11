# RustaSea Milestone Status

> **Last updated:** 2026-09-11
> **Scope:** Authoritative done / partial / missing status for milestones **M0–M6**, plus the **P0 Foundation** gap-closure progress.

## Purpose

`README.md` defines the goal, scope, deliverables, and success criteria for each
milestone, but it does not track what is actually implemented. This document is
the single source of truth for **milestone status**, grounded in the current
source tree. Every status claim cites a real `path:line` or a task/commit id so
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

- Merged source tree: `origin/master` (ORM + queue) plus the PR #1 Dependabot branch (router) and docs.
- P0 gap-closure work: `GAP-001` (sqlx pool, async execution, transactions) and `GAP-002` (router → controller dispatch).
- Commit-reference policy: [`docs/documentation-conventions.md`](documentation-conventions.md) — milestones, task IDs, and `path:line` only; no raw commit SHAs.
- Task registry: `DOC-001`, `DOC-002`, `DOC-ROOT`, `GAP-ROOT`, `GAP-P0`…`GAP-P5`.
- Prior research: [`docs/laravel-13-research.md`](laravel-13-research.md).
- Workspace inventory: **21 crates under `crates/` + `xtask` (22 workspace packages total)**; see the [canonical crate inventory](../.agents/documents/application/modules/manifest.md#canonical-crate-inventory-source-of-truth), verified against `Cargo.toml:2` and package manifests.

---

## Summary Matrix

| Milestone | Goal | Status | Headline evidence |
|---|---|---|---|
| **M0** | Bootstrap & Core | **Partial** | Container + provider lifecycle real (`crates/rustasea-foundation/src/lib.rs:56-201`); provider/command registries empty (`bootstrap/providers.rs:11`, `bootstrap/commands.rs:11`) |
| **M1** | Routing & HTTP | **Partial** | Router DSL + controller dispatch real (`crates/rustasea-router/src/dispatch.rs:23`); `route:list` prints an empty table (`crates/rustasea-cli/src/commands/inspect.rs:33`) |
| **M2** | ORM & Database | **Done** | Real sqlx pool + async execution (`crates/rustasea-orm/src/db.rs:24`, `db/exec.rs:70`); model CRUD (`model_ops.rs:25`), transactions (`tx.rs:56`), migrations/seeders (`migration.rs:190`), eager loading (`eager.rs:59`), pgvector (`vector.rs:112`) |
| **M3** | Auth, Middleware & Validation | **Partial** | JWT/CSRF/throttle/validation real; session guard is a placeholder (`crates/rustasea-auth/src/session.rs:125`); attribute runtime enforcement pending (GAP-003) |
| **M4** | Queue, Cache, Scheduling & Events | **Partial** | MemoryStore, SyncDriver, inline events real; real database/Redis queue drivers + worker loop (`crates/rustasea-queue/src/driver/database.rs:36`, `driver/redis.rs:30`, `driver/worker.rs:78`); cache Redis still `StoreUnavailable` (`crates/rustasea-cache/src/redis.rs:37`) |
| **M5** | DX, CLI & Testing | **Partial** | CLI + 13 `make:*` generators real (`crates/rustasea-cli/src/commands/builtins.rs`); `xtask check-cycles` is a no-op (`xtask/src/main.rs:85`); `testcontainers` unused (`Cargo.toml:52`) |
| **M6** | Advanced (Broadcast, Search, FS, AI) | **Partial** | Broadcast WS/SSE, `object_store` storage, JSON:API real; AI providers are deterministic stubs (`crates/rustasea-ai/src/adapters.rs:50`) and vector search is in-memory only (`crates/rustasea-search/src/lib.rs:15`) |

---

## M0 — Bootstrap & Core

**Goal recap:** Bootable application skeleton with typed config, service
container, provider lifecycle, and graceful shutdown.

**Status: Partial** — the application boots, resolves container bindings, and
shuts down gracefully, but the provider/command registries are placeholders and
there is no provider DAG or project scaffolder.

**Done**
- Service container with `bind` / `singleton` / `instance` / `get` — `crates/rustasea-foundation/src/lib.rs:56`, `:63`, `:74`, `:85`.
- `Application::boot` runs register → boot — `crates/rustasea-foundation/src/lib.rs:188`.
- Graceful shutdown on `SIGTERM`/`SIGINT` — `crates/rustasea-foundation/src/lib.rs:217`, `:221`.
- Typed config loader with TOML + env overlay — `crates/rustasea-config/src/lib.rs:13`, `:16`, `:23`.
- Runnable app boots, registers routes, serves real handlers, and shuts down — `crates/rustasea-app/src/main.rs` (`configure()`, `app.shutdown()`).

**Partial (reason)**
- `bootstrap/app.rs` provider is a no-op scaffold — `bootstrap/app.rs:15-21`.
- Config `load()` reads only `config/app` — `crates/rustasea-config/src/lib.rs:16`; other `config/*.toml` are not auto-loaded.

**Missing**
- Provider registry returns an empty vector — `bootstrap/providers.rs:11`.
- Command registry returns an empty vector — `bootstrap/commands.rs:11`.
- Provider DAG / dependency ordering (no ordering or relationship resolution exists).
- `cargo artisan new <app>` project scaffolder (no generator found).

**Evidence:** `crates/rustasea-foundation/src/lib.rs:56-221`; `crates/rustasea-config/src/lib.rs:13-23`; `bootstrap/app.rs:15-29`; `bootstrap/providers.rs:11`; `bootstrap/commands.rs:11`; `crates/rustasea-app/src/main.rs`.

**Next actions**
- Populate `bootstrap/providers.rs` / `bootstrap/commands.rs` from generated app modules.
- Add provider dependency ordering (DAG) to the boot sequence.
- Extend the config loader to discover all `config/*.toml` and add `artisan new`.

---

## M1 — Routing & HTTP

**Goal recap:** Expressive HTTP layer with routing, middleware, request/response
ergonomics, and an HTTP client.

**Status: Partial** — the router DSL and real controller dispatch landed in
GAP-002, and the HTTP client `throw` semantics are implemented, but
CLI introspection (`route:list`, `show:model`) and idle-timeout enforcement are
not wired.

**Done**
- Router DSL: `get`/`post`/`put`/`delete`/`patch`/`options`/`any`, `group`, prefix/name/domain/resource — `crates/rustasea-router/src/router.rs:136`, `:288`.
- Controller dispatch to real handlers (`GAP-002`) — `crates/rustasea-router/src/dispatch.rs:23`, `:62`.
- `#[route]` metadata consumed at registration — `crates/rustasea-router/src/router.rs:218`; Laravel→Axum path params translated — `crates/rustasea-router/src/dispatch.rs`.
- HTTP client `throw` / `try_throw` callbacks and typed `HttpError` — `crates/rustasea-http/src/lib.rs:286`, `:298`, `:207`.
- Runnable app serves real handlers from `routes/web.rs`.

**Partial (reason)**
- `route:list` renders an **empty** table with a "until the router is wired" comment — `crates/rustasea-cli/src/commands/inspect.rs:33`.
- `show:model` emits placeholder output — `crates/rustasea-cli/src/commands/inspect.rs:67`.
- HTTP idle (inter-byte) timeout is declared but not enforced — `crates/rustasea-http/src/lib.rs:202`, `:364`.

**Missing**
- Real route-table introspection feeding `route:list` with middleware + binding fields.

**Evidence:** `crates/rustasea-router/src/router.rs:136-288`; `crates/rustasea-router/src/dispatch.rs:23-83`; `crates/rustasea-http/src/lib.rs:202-367`; `crates/rustasea-cli/src/commands/inspect.rs:33`, `:67`; task `GAP-002` (completed).

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
loading, and pgvector support all execute through the runtime `sqlx` API. This
landed on `origin/master` as the M2 ORM implementation: pool, async execution
and model ops, transactions, migrations/seeders/factories, and eager loading +
pgvector (tracked by `GAP-001`).

**Done**
- Fluent SQL builder — `crates/rustasea-orm/src/builder.rs`, `crates/rustasea-orm/src/clause.rs`; async execution `crates/rustasea-orm/src/builder/exec.rs:105` (`get`), `:112` (`first`), `:145` (`paginate`).
- Real sqlx pool + connection foundation — `crates/rustasea-orm/src/db.rs:24` (`DbPool`), `:43` (`connect`), `:70` (`ping`), `:91` (`close`); driver dispatch — `crates/rustasea-orm/src/db/exec.rs:70` (`fetch_json`), `:95` (`execute_bind`), `:116` (`execute_script`).
- Async model operations — `crates/rustasea-orm/src/model_ops.rs:25` (`create`, `save` upsert, `update`, `delete`, `soft_delete`, `force_delete`, `refresh`, `first_for_update`).
- Real transactions + executor dispatch — `crates/rustasea-orm/src/tx.rs:56`, `crates/rustasea-orm/src/execution.rs:324` (`transaction`).
- Real migrations, seeders, and factory state — `crates/rustasea-orm/src/migration.rs:190` (`run`), `:241` (`rollback`), `:280` (`fresh`), `:289` (`seed`); `crates/rustasea-orm/src/factory.rs`.
- Eager loading + relation serde round-trip — `crates/rustasea-orm/src/eager.rs:59`, `crates/rustasea-orm/src/relations.rs:22`.
- pgvector support — `crates/rustasea-orm/src/vector.rs:112` (`has_extension`), `:128` (`require_extension`), `:150` (`vector_param`).
- Populated `config/database.toml` (driver/url/pool settings).

**Partial (reason)**
- `raw` / `raw_sql` emit statement fragments for display only — `crates/rustasea-orm/src/execution.rs:239`, `:246`.

**Missing**
- Compile-time `query!` macros (intentionally excluded: CI has no `DATABASE_URL`).

**Evidence:** `crates/rustasea-orm/src/db.rs:24-130`; `crates/rustasea-orm/src/db/exec.rs:70-138`; `crates/rustasea-orm/src/model_ops.rs:25-143`; `crates/rustasea-orm/src/tx.rs:56-192`; `crates/rustasea-orm/src/migration.rs:190-296`; `crates/rustasea-orm/src/eager.rs:59`; `crates/rustasea-orm/src/vector.rs:112-152`; task `GAP-001`.

**Next actions**
- Wire `route:list` / `show:model` ORM introspection (`GAP-010`, `GAP-011`).
- Add `pgvector`-backed vector index integration (M6).

---

## M3 — Auth, Middleware & Validation

**Goal recap:** Complete auth, authorization, and validation with hardened
security defaults.

**Status: Partial** — JWT, CSRF, throttling, and validation are real; the
session guard is a placeholder and declarative attributes have no runtime
consumer yet.

**Done**
- JWT guard — `crates/rustasea-auth/src/jwt.rs`.
- Origin-aware CSRF protection (`Sec-Fetch-Site`) — `crates/rustasea-auth/src/csrf.rs`.
- Rate limiter / throttle — `crates/rustasea-auth/src/throttle/`.
- Validation crate (rules, ErrorBag, form requests) — `crates/rustasea-validation/`.
- Guard manager with typed `GuardMismatch` errors — `crates/rustasea-auth/src/guard.rs:222`, `:348`.

**Partial (reason)**
- Session guard is a placeholder ("M5 wiring target"); `store()` is `#[cfg(test)]` and `logout` is a no-op — `crates/rustasea-auth/src/session.rs:117`, `:125`, `:147`, `:207`.
- `tower-sessions` is declared but unused in any `.rs` — `Cargo.toml:28`.
- `#[authorize]` / `#[middleware]` emit metadata consts that nothing consumes at runtime — `crates/rustasea-macros/src/lib.rs:74`, `:111`.

**Missing**
- Runtime enforcement of `#[middleware]` / `#[authorize]` / `#[tries]` / `#[backoff]` / `#[timeout]` (GAP-003, backlog).
- Store-backed session guard and session persistence.

**Evidence:** `crates/rustasea-auth/src/{jwt,csrf,guard,session}.rs`; `crates/rustasea-macros/src/lib.rs:74-257`; `Cargo.toml:28`; task `GAP-003` (backlog).

**Next actions**
- Implement the `tower-sessions`-backed session guard (`GAP-007`).
- Add the attribute registration + runtime consumer (`GAP-003`).

---

## M4 — Queue, Cache, Scheduling & Events

**Goal recap:** Async workloads, caching, scheduling, and event dispatch with
observable queue metrics.

**Status: Partial** — in-memory cache, the sync queue driver, inline events, and
the scheduler exist, and `origin/master` added real `database`/`redis` queue
drivers with a worker loop, persistent failed jobs, and a `queue:work` command;
the cache Redis store, `queue:failed`/`queue:retry` CLI, and queue-backed async
listeners are still missing.

**Done**
- In-memory cache store — `crates/rustasea-cache/src/memory.rs:35`.
- Synchronous queue driver — `crates/rustasea-queue/src/driver.rs:115`.
- Real `database` queue driver — `crates/rustasea-queue/src/driver/database.rs:36` (`push`/`pop`/`ack`/`release`/`dead_letter` over a `jobs` table), with DB-backed failed jobs at `:71` (`failed_jobs`), `:95` (`retry_failed`).
- Real `redis` queue driver (feature-gated) — `crates/rustasea-queue/src/driver/redis.rs:30` (list + delayed/reserved sorted sets; disabled when no URL).
- Queue worker loop + handler registry — `crates/rustasea-queue/src/driver/worker.rs:78` (`run_worker`), `:36` (`register_job`).
- Queue migrations for `jobs`/`failed_jobs` — `crates/rustasea-queue/src/migrations.rs:16`, `:48`, `:90`; `queue:work` CLI command — `crates/rustasea-cli/src/commands/queue.rs:18`.
- Inline event dispatch + `dispatchAfterResponse` — `crates/rustasea-events/src/dispatcher.rs:102`, `:118`.
- Scheduler with pause/resume — `crates/rustasea-schedule/src/lib.rs:19`.

**Partial (reason)**
- Redis cache store returns `StoreUnavailable` for `get`/`put` — `crates/rustasea-cache/src/redis.rs:37`.
- The in-process `SyncDriver` still tracks failed jobs in a `OnceLock<Mutex<Vec<FailedJob>>>` — `crates/rustasea-queue/src/driver.rs:171`.
- Queue-backed listeners error with "no queue enqueue path is wired yet" — `crates/rustasea-events/src/dispatcher.rs:71`.

**Missing**
- Real Redis cache store.
- `queue:failed` / `queue:retry` CLI commands.
- Async event listeners via the queue.

**Evidence:** `crates/rustasea-cache/src/{memory,redis}.rs`; `crates/rustasea-queue/src/driver.rs:115-197`; `crates/rustasea-queue/src/driver/{database,redis,worker}.rs`; `crates/rustasea-queue/src/migrations.rs`; `crates/rustasea-events/src/dispatcher.rs:71-118`; `crates/rustasea-schedule/src/lib.rs:19`; backlog `GAP-005`.

**Next actions**
- Implement the Redis cache store (`GAP-005`).
- Add `queue:failed` / `queue:retry` CLI commands.
- Wire queue-backed async listeners.

---

## M5 — DX, CLI & Testing

**Goal recap:** First-class CLI, code generation, and a Laravel-like testing
story.

**Status: Partial** — the CLI and 13 `make:*` generators are real, but several
expected commands are missing, `check-cycles` is a no-op, and `testcontainers`
is declared but not used.

**Done**
- `cargo artisan` CLI with command registry — `crates/rustasea-cli/src/registry.rs`, `crates/rustasea-cli/src/lib.rs`.
- 13 `make:*` generators: controller, model, provider, command, job, event, listener, observer, test, seeder, migration, agent, tool — `crates/rustasea-cli/src/generators/mod.rs:19`, `crates/rustasea-cli/src/commands/builtins.rs`.
- Typed command args/flags and prompt/table helpers.

**Partial (reason)**
- `xtask check-cycles` runs `cargo metadata`, discards the output, and prints "member count OK" — no cycle detection — `xtask/src/main.rs:85`, `:92`.
- `testcontainers` is declared in the workspace but no crate depends on it; container helpers shell out to the `docker` binary — `Cargo.toml:52`, `crates/rustasea-testing/src/containers.rs`.

**Missing**
- `cargo artisan new` (project scaffold).
- `make:middleware`, `make:request`, and `xtask migrate` commands.
- Real cycle detection in `xtask`.

**Evidence:** `crates/rustasea-cli/src/generators/mod.rs:19`; `crates/rustasea-cli/src/commands/builtins.rs`; `xtask/src/main.rs:85`; `Cargo.toml:52`; backlog `GAP-016`, `GAP-017`, `GAP-018`.

**Next actions**
- Add missing CLI commands (`GAP-016`).
- Use real `testcontainers` + `sqlx::test` (`GAP-017`).
- Implement real `check-cycles` DAG validation (`GAP-018`).

---

## M6 — Advanced (Broadcasting, Search, Filesystem, AI SDK, Real-time)

**Goal recap:** AI-native capabilities and full Laravel parity on advanced
features.

**Status: Partial** — broadcasting (WS/SSE), `object_store`-backed storage, and
JSON:API are real; AI providers and vector search are deterministic in-process
stubs.

**Done**
- Broadcasting: WebSocket (feature-gated default) + SSE + `ShouldBroadcast` — `crates/rustasea-broadcast/Cargo.toml:20`, `crates/rustasea-broadcast/src/lib.rs:36`.
- Filesystem on real `object_store` — `crates/rustasea-storage/Cargo.toml:10`, `crates/rustasea-storage/src/manager.rs:15`, `:162` (`ObjectDisk`), `:118` (`get`), `:135` (`put`).
- JSON:API resources with correct content type — `crates/rustasea-jsonapi/src/wire.rs:7`.

**Partial (reason)**
- AI providers are deterministic in-process stubs — `crates/rustasea-ai/src/adapters.rs:50`, `:214`.
- Vector search ships `MemoryVectorStore` only; engine integration is M6-full — `crates/rustasea-search/src/lib.rs:15`.

**Missing**
- Real AI SDK adapters (e.g. `async-openai` + per-provider crates).
- Template rendering integration (`askama` / `minijinja`) — not declared.
- `pgvector`-backed vector index integration.

**Evidence:** `crates/rustasea-broadcast/{Cargo.toml,src/lib.rs}`; `crates/rustasea-storage/{Cargo.toml,src/manager.rs:15-239}`; `crates/rustasea-jsonapi/src/wire.rs:7`; `crates/rustasea-ai/src/adapters.rs:50`; `crates/rustasea-search/src/lib.rs:15`; backlog `GAP-014`.

**Next actions**
- Implement real provider adapters (`GAP-014`).
- Integrate a real vector index and templating.

---

## P0 Progress (Foundation Unblockers)

**Parent:** `GAP-P0` — *P0 — Foundation unblockers (DB backend + router dispatch)* — status **pending** (children below).

| Task | Title | Status | Evidence |
|---|---|---|---|
| **GAP-001** | Wire sqlx DB backend + connection pool into `rustasea-orm` | **Done** | `crates/rustasea-orm/src/db.rs:24`, `crates/rustasea-orm/src/db/exec.rs:70` |
| **GAP-002** | Router → controller dispatch (real handler binding) | **Done** | `crates/rustasea-router/src/dispatch.rs:23-83` |
| **GAP-003** | Runtime consumer for declarative attributes (`#[middleware]`, `#[authorize]`, `#[tries]`, `#[backoff]`, `#[timeout]`) | **Pending / backlog** | `crates/rustasea-macros/src/lib.rs:74-257` (emits metadata only) |

**Other gap phases (all backlog):** `GAP-004`–`GAP-009` (P1), `GAP-010`–`GAP-013`
(P2), `GAP-014`–`GAP-015` (P3), `GAP-016`–`GAP-018` (P4), `GAP-019`–`GAP-021`
(P5). See `GAP-ROOT` for the full program.

> **Note:** `GAP-P0` is **not** fully closed because `GAP-003` remains in
> backlog, even though `GAP-001` and `GAP-002` are completed and pushed.

---

## Related Documents

- [`README.md`](../README.md) — milestone goals, scope, deliverables, success criteria.
- [`docs/laravel-13-research.md`](laravel-13-research.md) — Laravel 13 feature research.
- [`docs/laravel-parity.md`](laravel-parity.md) — Laravel 13.x API adoption mapping (maintained separately).
- [`docs/documentation-conventions.md`](documentation-conventions.md) — commit-reference policy (milestone + date, no raw SHAs).
