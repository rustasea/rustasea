# RustaSea — Test Plan (P5 Test Planning & Quality Strategy)

> **Owner:** vheins/rustasea | **Phase:** Implementation P5 | **Task:** TASK-010 | **Date:** 2026-09-07
> **Parents:** `brd.md` (BR-01..09) · `prd.md` (FR-000..612, NFR-*) · `fsd.md` (FS-M0-01..M6-07) · `bdd-scenarios.md` (33 features)
> **Stack:** Rust 1.88+, edition 2021, `tokio`, `axum`+`tower`, `sqlx`/`sea-orm`, `deadpool`, `validator`, `jsonwebtoken`+`argon2`, `serde`, `config`+`dotenvy`, `clap`+`xtask`, `syn`/`quote`, `moka`+`deadpool-redis`, `askama`, `reqwest`, `object_store`, `pgvector`+`async-openai`, `testcontainers`, `cargo test`
> **Reference:** `docs/laravel-13-research.md` (20 Laravel 13 features) · `README.md` §Milestones · `test-architecture.md` (4-concern rule)

---

## 1. Objectives and Scope

This plan is the single source for **what** is tested, **where** (concern + crate), **how much** (pyramid ratios + coverage gates), and **when** (per milestone + NFR + security triage). It is dependency-ordered M0→M6 matching `prd.md` and `fsd.md`. No milestone test suite may start without its prerequisites green per regression priority (see §9).

Non-objectives reaffirmed (from `brd.md` §4 Out-of-Scope): Filament/Nova admin, PaaS, full Blade runtime, PHP bridge. Any promotion requires ADR.

---

## 2. Principles

### 2.1 Four-Concern Rule (MANDATORY — from `test-architecture.md`)

Every feature is **incomplete** until these four layers are accounted for. Concerns are exclusive — a rule asserted in one concern MUST NOT be re-asserted in another (priority DB > Service > State > UI).

| Layer | MUST Test | MUST NOT Test | RustaSea Mapping |
|-------|-----------|---------------|------------------|
| **1. Database** | Integrity, FKs, unique indexes, cascades, defaults, `pgvector` column type, migration idempotence | Business rules, UI validation | `sqlx` migrations, `sqlx::migrate!`, `pgvector` `vector(1536)`, soft-delete `deleted_at`, snake_plural table convention |
| **2. Service** | Domain calculations, business rules, action outcomes, `Job<T>`/`Event<T>` typed payloads | DB constraints, UI validation | `rustasea-orm` query builder, `rustasea-queue`/`cache`/`events`/`schedule`, `rustasea-auth` guards, `rustasea-validation` strict rules, `rustasea-ai` provider trait |
| **3. State** | Status transitions, guards, workflow rules (`register→boot→running→draining`, `pending→reserved→processing→succeeded/failed→retrying`, `running↔paused` scheduler, auth token lifecycle) | DB integrity, UI rendering | Provider DAG, queue retry state machine, schedule pause/resume, JWT `login→parse→refresh→logout` lifecycle, dispatch-after-response deferral |
| **4. UI** | Form/validation feedback, middleware visibility, auth gates, route introspection output, CLI prompts/generators, JSON:API/Broadcast/SSE rendering | Business calcs, DB logic | `rustasea-router`/`http` middleware + extractors + `ErrorBag`, `route:list`/`show:model`, `cargo rustasea make:*` generators, `JsonApiResource`, WebSocket/SSE, CLI `list`/`ask`/`confirm` |

Execution order: Database → Service → State → UI. Success in a later layer depends on the earlier layer. Duplication across concerns is a defect.

### 2.2 Test Pyramid (per Architecture)

RustaSea is a **workspace of crates** (monolith-shaped app + microservice-shape libraries). Base ratios follow two templates; per-milestone blend is in §4.

| Architecture | Unit | Integration | Contract | E2E |
|--------------|------|-------------|----------|-----|
| Monolith (per crate) | 60–70% | 20–30% | — | 5–10% |
| Crate-as-service (cross-crate) | 50% | 30% | 15% | 5% |

Anti-patterns forbidden: ice-cream cone (many E2E, few unit), testing implementation details, over-mocking `tokio`/`axum` internals, slow unit tests (>100 ms). E2E remains business language from `bdd-scenarios.md`.

### 2.3 Incremental Adoption

Each crate MUST pass `cargo check` and its own crate test suite **standalone** (see NFR-Sca-02). Cross-crate suites are additive. AI/broadcast/storage are behind feature flags — their test matrix is gated on `features = ["ai"]` etc.

---

## 3. Tooling and Harness

| Concern | Tool |
|---------|------|
| Unit | `cargo test --lib`, `rstest`/`proptest` for property tests, `mockall` only where trait boundary demands it — prefer fakes over mocks |
| Integration (DB) | `sqlx::test`, `testcontainers` (Postgres + Redis images), per-test random ports, isolated DB per test binary, `migrate` once per binary |
| HTTP | `axum::test` / `tower::ServiceExt::oneshot`, `reqwest` mock via `wiremock`/`httpmock` for throw/timeout cases |
| Contract | `schemars` JSON-Schema validation for JSON:API/queue payloads/HTTP responses; `cargo-insta` snapshots for CLI `route:list --json` / `list --json` |
| Migration | `sqlx::migrate!` round-trip (up→down→up) harness + bulk migration idempotence |
| E2E / BDD | `cucumber-rs` (Gherkin from `bdd-scenarios.md`), `tokio::test` async, `testcontainers` for queue/cache/schedule shared fixtures |
| Factory | `Factory::create` + `Str` sequences with per-test reset hook in `TestCase` (see FS-M5-04) |
| Observability | `tracing` subscriber capture + `metrics`/`tracing-test` assertions |
| Perf / NFR | `criterion` benches, `k6`/`oha` for HTTP p95, `cargo test -- --test-threads` contention probes |
| Security | `cargo audit` + `cargo deny`, `Sec-Fetch-Site` fuzz fixtures, path-traversal corpus, allow-list negative suite |

### Harness Conventions

- `TestCase` trait sets up `AppState` with `.env.testing` overlay, isolated Postgres/Redis via `testcontainers`, runs `migrate` once, resets `Str` factory sequences between tests. Teardown kills containers; timeout 30 s → `TestError::ContainerTimeout`.
- `Factory::create(n)` and `Str` resets are mandatory for determinism under `--test-threads=N`; port collision handled by random allocation.
- Unit tests are `#[deny(clippy::unwrap_used)]`-clean except in test code where `expect` with message is allowed.

---

## 4. Pyramid per Milestone

> Legend: U=Unit · I=Integration (crate + cross-crate) · C=Contract (JSON-Schema/snapshot) · E=E2E (Gherkin `cucumber-rs`). Percentages are targets for new code per milestone; whole-workspace aggregate target is 62% U / 25% I / 8% C / 5% E. No milestone is cone-shaped.

| Milestone | Crates Primary | U | I | C | E | Rationale |
|-----------|---------------|---|---|---|---|-----------|
| **M0 Bootstrap & Core** | `rustasea-foundation`, `rustasea-config`, `rustasea` umbrella | 65% | 25% | 5% | 5% | Provider DAG and config layering are service-heavy; DB is minimal (migration table only); UI is `cargo rustasea new` scaffold output |
| **M1 Routing & HTTP** | `rustasea-router`, `rustasea-http`, `rustasea-macros` (`#[route]`) | 60% | 25% | 10% | 5% | `axum` wiring needs `oneshot` integration; JSON extractors/middleware are contract-shaped; domain routing is branching logic |
| **M2 ORM & Database** | `rustasea-orm`, `rustasea-macros` (`Model`), `pgvector` | 55% | 30% | 5% | 10% | Heaviest integration: `sqlx` Postgres/MySQL/SQLite + `pgvector` + migrations + `serde` round-trip; E2E elevada for builder Gherkin |
| **M3 Auth, Middleware & Validation** | `rustasea-auth`, `rustasea-validation`, `rustasea-macros` (`#[middleware]`/`#[authorize]`/`#[validate]`) | 60% | 20% | 10% | 10% | JWT/session/CSRF are security E2E; strict `ErrorBag` and allow-list are contract-heavy |
| **M4 Queue, Cache, Scheduling & Events** | `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule` | 50% | 30% | 10% | 10% | Cross-store matrix (memory+redis, sync+database+redis) drives integration; Cloud metrics and schedule pause are contract/E2E |
| **M5 DX, CLI & Testing** | `rustasea-cli`, `rustasea-macros` (generators+attrs), `rustasea-testing` | 55% | 20% | 15% | 10% | `make:*` output is snapshot-contract; CLI `list --json` + `Artisan::call` are contract; `TestCase` isolation is integration-heavy |
| **M6 Advanced** | `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai` (12 providers) | 50% | 25% | 15% | 10% | JSON:API and provider-trait shape are contract-heavy; AI streaming/broadcast/queue are E2E; storage read-through is integration |

Per-milestone **minimum test counts** (directional — actual count governed by 4-concern coverage in §6, not raw count):

| Milestone | Floor tests (new) | Floor per concern (min) |
|-----------|-------------------|-------------------------|
| M0 | 30 | DB 4 / Service 10 / State 8 / UI 8 |
| M1 | 35 | DB 2 / Service 12 / State 6 / UI 15 |
| M2 | 55 | DB 15 / Service 18 / State 6 / UI 16 |
| M3 | 45 | DB 4 / Service 14 / State 10 / UI 17 |
| M4 | 55 | DB 8 / Service 18 / State 12 / UI 17 |
| M5 | 40 | DB 2 / Service 10 / State 6 / UI 22 |
| M6 | 60 | DB 6 / Service 18 / State 8 / UI 28 |

---

## 5. Four-Concern Coverage per Milestone (Normative Matrix)

Each cell MUST have at least one suite; `†` marks feature-flag gating.

| Milestone | Database | Service | State | UI |
|-----------|----------|---------|-------|-----|
| **M0** | `migrations` table idempotence; `config/*.toml` parse diagnostics | Container `Bind`/`Singleton`/`Instance` + `Manager::extend` closure binding | Provider `register→boot` DAG; cycle detection; `Idle→Registering→Booting→Running→Draining→Stopped`; graceful shutdown drain | `cargo rustasea new <app>` scaffold + `bootstrap/app.rs` existence + `cargo check` clean; `.env.example` |
| **M1** | — (no persistence) | Route registration, group prefix, `resource` expansion, `Http` client `throw`/timeout classification | Domain-route precedence state (domain before non-domain), throttle bucket window | `route:list --json` binding fields + middleware list; typed extractor `422 ErrorBag`; `Json`/`View` Content-Type; CORS headers |
| **M2** | `#[derive(Model)]` table/columns/indexes/FK, `deleted_at` soft delete, `vector` column type, `dropVectorIndex`, migration up→down→up, seeder idempotence | Query builder (`where`/`orWhere`/`whereJson*`, `find`/`firstOrFail`, `create`/`save`/`update`/`delete`/`forceDelete`, `paginate`/`cursor`, `chunkBy`/`orWhereKey`/`whereBinary`/`StraightJoin`/`insertOrIgnoreReturning`, `upsert` strict `uniqueBy`, MySQL `DELETE JOIN`, `toSql`/`toRawSql`, locks, scopes, raw queries) | Transaction isolation (`forUpdate` inside `transaction`), `Factory` sequence isolation between tests, `pgvector` extension-missing fast-fail | Collection `serde` round-trip preserving eager relations; factory output + scaffolded model/migration files lint-clean |
| **M3** | Session/allow-list storage keys (hyphenated `-session-`/`-cache-`), password hash storage | `validator` strict `in_array`/`contains`/`doesnt_contain`, `ErrorBag` aggregation, `#[validate]` wiring, `Auth::extend` custom guard, `argon2` verify | JWT `login→parse→refresh→logout` token lifecycle + `GuardMismatch`/`BadCredentials`/`ExpiredToken`/`InvalidToken`; CSRF `Sec-Fetch-Site` origin state (`cross-site→403` vs `missing→token-only`) | `#[middleware("auth:jwt")]` gate (401), `#[authorize]` gate (403), CORS preflight headers, session cookie JSON + prefix, rate-limit `429 Retry-After`, `X-Forwarded-For` behind proxy policy |
| **M4** | `jobs` table + `failed_jobs` table, `schedule_paused` flag store | `Job<T>` handle + `ShouldRetry`/`#[tries]`/`#[backoff]`/`#[timeout]`, `touch()` TTL semantics, `Store`/`Repository` + `withContext`, `Lock` `get`/`block`/`release`, `Dispatcher::dispatch`/`dispatchAfterResponse` | Queue `Pending→Reserved→Processing→Succeeded|Failed→Retrying→DeadLetter`; `Queue::route` registry `OnceLock` + `DuplicateRoute`; chain stop-on-failure; `Running↔Paused` scheduler; `skipIfStillRunning`/`onOneServer`; buffer-flush after response | `schedule:list`/`schedule:run`/`pause`/`resume` CLI; `queue:failed`/`queue:retry` CLI; Cloud metrics output (`pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob`) |
| **M5** | `TestCase` container ports (distinct per test binary) | `Shutdownable` trait on long-running commands; `Artisan::call` in-process invocation; declarative attrs `#[tries]`/`#[backoff]`/`#[timeout]`/`#[usage]`/`#[help]`/`#[hidden]`/`#[withoutBroadcasting]` | `make:*` generator idempotence (`AlreadyExists` without `--force`); prompt cancellation (`confirm` → abort, exit 1) | `cargo rustasea list --json` + per-command help `usage`; `make:controller/model/.../agent/tool` files `rustfmt`+`clippy` clean; `table`/`progressBar`/`spinner` rendering; paginator `bootstrap-3` view |
| **M6†** | `storage` disk roots, `pgvector` index lifecycle | `Storage` primary→fallback read-through + `copy_back`, `JsonApiResource` sparse fieldsets/inclusion/links/headers, `AiProvider` 12-provider adapter trait, `SimilaritySearch`/`FileStorage`/`ToolSearch` deferred loaders, `Str::toEmbeddings` | `BroadcastService` channel auth lifecycle; `Agent` streaming state (`event: token` ordering) + sub-agent delegation + MCP feature flag state (`McUnavailable` when off); queued-tool `→job` emission | WebSocket `private-chat.1` channel subscription + `4403 Unauthorized`; SSE `text/event-stream` chunks; JSON:API `application/vnd.api+json` media type + `RelationNotLoaded`; AI streaming over WebSocket; `make:agent`/`make:tool` scaffolds; `DeleteWhenMissingModels` visible outcome (skip, not retry) |

Gates: no story is done without its row's DB+Service+State+UI suites passing (or explicitly N/A with justification per §6).

---

## 6. Test Design Strategy

### Techniques (selected per feature)

| Technique | When |
|-----------|------|
| Equivalence Partitioning (EP) | Input partitions — e.g., `upsert unique_by: empty vs non-empty`, `Cache::touch key exists vs missing` |
| Boundary Value Analysis (BVA) | Numeric boundaries — `Throttle 60→61`, `chunkBy 500→501`, `ttl 60→120` extension, `tries 3` attempts |
| Decision Tables | CSRF `Sec-Fetch-Site × origin allow-list × token`, cache store matrix, queue driver matrix, vector dimension matrix |
| State Transition | Provider lifecycle, queue job, scheduler pause/resume, JWT token lifecycle, broadcast subscription |
| Error Guessing | `unwrap_used` bans, traversal payloads `../../`, allow-list deserialization of unknown types, `Manager::extend` prefix races |

Selection rationale and per-crate catalog are detailed in `test-cases.md`. Cross-cutting boundaries are in `qa-design.md`.

### Coverage Gates

- **Line/branch (per crate):** ≥85% lines, ≥80% branches (workspace aggregate ≥80% lines). Measured via `cargo llvm-cov` / `tarpaulin`. Exceptions require ADR that maps excluded lines to generated-code shim.
- **Mutation (periodic):** `cargo mutants` on `rustasea-orm`/`validation`/`auth` — threshold ≥65% killed mutants per PR touching those crates.
- **Contract conformance:** 100% of JSON-API documents validate the JSON:API 1.1 schema; 100% of `route:list --json` snapshots `cargo insta` reviewed.

---

## 7. NFR Strategy

### 7.1 Performance & Load

| Target (from `prd.md` NFR) | How tested | Fixture |
|----------------------------|------------|---------|
| NFR-Per-01 cold boot M0 `<2s` on CI (2 vCPU, 5 providers) | Bench `Application::configure().boot()` in `criterion`; CI gate fails on p50 >2 s | `cargo bench --bench boot` in `xtask check` |
| NFR-Per-02 HTTP p95 `<50 ms` (no DB, `GET /users`, 1k RPS localhost) | `oha -n 10000 -c 100 http://localhost:3000/users` + `ghz` for gRPC stub; fail on p95 >50 ms | Dedicated `perf` feature build, no `debug-assertions` |
| NFR-Per-03 `Cache::get` p95 `<5 ms` memory / `<20 ms` redis (localhost) | `criterion` microbench + `k6` scenario concurrent readers | Redis `testcontainers` singleton for bench |
| NFR-Per-04 `cargo check` after `make:*` `<10s` incremental | `cargo check --timings` in CI after generator fixtures | Generated app in `target/.generated-bench` |

Load strategy: latency profile captured per milestone; no PR merging M1/M2 changes that regress p95 >15% without ADR.

### 7.2 Reliability & Resilience

| NFR | Test |
|-----|------|
| NFR-Rel-01 graceful drain (M0) + FS-M4-05 pause-during-execution | `SIGTERM` during `GET /slow (3s)` with `shutdown_timeout 10s` — assert `200` + exit `0`; timeout-expiry hook asserts `ShutdownTimeout` diagnostic |
| NFR-Rel-02 `migrate` idempotence | Bulk migration harness: `migrate` → re-run no-op; `migrate:fresh` → re-up; per-migration up→down→up |
| NFR-Rel-03 `Lock::block` liveness | Contending workers on `Lock("billing")` — holder 10 s, waiter `block(2s)` — assert `AlreadyHeld` after lease expiry applies |

Chaos subset (see `qa-design.md` §Chaos): Redis unavailable mid-queue, Postgres connection lost mid-transaction, `schedule:pause` during tick, streamed AI consumer lag/backpressure (`Lagged` / bounded `mpsc`).

### 7.3 Observability

Signals MUST emit typed traits, not ad-hoc logs. Tests assert trait methods, not string matching alone.

| Signal (from `fsd.md` §4.3 / `prd.md` NFR-Mai-01) | Assertion |
|---|---|
| `route:list --json` binding fields + middleware | Snapshot (`cargo insta`) + `serde` schema validation |
| `show:model` / `ModelInspector` | Structured metadata object with attributes/relations/casts — schema snapshot |
| Queue metrics (#8) `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` | Values against known queue state (42 pending, RFC3339 oldest), empty-queue `None` |
| `SchedulePaused`/`ScheduleResumed` events | Event spy collected after `schedule:pause`/`resume` |
| Throttle `429` + `Retry-After` | Header present + integer value; throttle-hit metric emitted exactly once per throttled request |

---

## 8. Security Triage (from `security-audit` skill, sourced to `prd.md` NFR-Sec-*)

Security tests are NOT a separate pyramid — they are distributed across the 4 concerns with a dedicated triage table here. Every triage item maps to a verification test in §5 and `test-cases.md`.

| # | Threat (Laravel 13 trace) | Control | Verification Test | Severity | Priority |
|---|---|---|---|---|---|
| Sec-01 | CSRF bypass via token theft (#11 origin-aware) | `PreventRequestForgery` checks `Sec-Fetch-Site` allow-list before token-only fallback | `POST /form` with `Sec-Fetch-Site: cross-site` + `Origin: evil` → `403 UntrustedOrigin` even with valid token; missing `Sec-Fetch-Site` degrades to token-only (pass) | High | P0 |
| Sec-02 | Object-injection via session/cache deserialization (#12) | JSON default + `serializable_classes` allow-list before `deserialize` | `Cache::get` for `AdminDto` when allow-list has only `UserDto` → `NotAllowed`; `AdminDto` never instantiated | High | P0 |
| Sec-03 | Path traversal via `Storage::path()` (#9) | Canonicalize + confinement under disk root | Corpus `../../etc/passwd`, `%2e%2e/`, symlink escaping — every probe → `StorageError::PathTraversal`, no filesystem access | High | P0 |
| Sec-04 | Cache prefix collision (brand confusion) (#12) | Hyphenated prefixes `-cache-`/`-session-` | `Cache::store("redis").prefix()` contains `-cache-` (not `_cache_`); session key contains `-session-` | Medium | P1 |
| Sec-05 | Weak password hashing / timing side-channel | `argon2` with per-password salt + constant-time `verify` | `argon2` params asserted (`m=19456, t=2, p=1` or configured); different hashes for same password (salt); verify on wrong password in constant-time (no early exit branch) | High | P0 |
| Sec-06 | Rate-limit bypass via forged `X-Forwarded-For` | Trusted-proxy gate before IP extraction | `X-Forwarded-For` ignored when no trusted proxies; counted only when `trusted_proxies` configured | Medium | P1 |
| Sec-07 | CORS allow-list bypass | `Cors::allow_origins` strict origin comparison | `Origin: https://evil.com` → no `Access-Control-Allow-Origin`; `evil.com.example.com` suffix trick still rejected | Medium | P1 |
| Sec-08 | `Storage` read-through fallback leakage (copy-back copies attacker payload to primary) | `copy_back` only on trusted fallback read with integrity check | Fallback-only file promotion requires explicit `copy_back: true`; primary not polluted when `copy_back: false` (asserted) | Medium | P2 |
| Sec-09 | AI streaming / broadcast auth bypass | Channel auth gate evaluated before stream subscription | `private-chat.1` unauthenticated subscriber → `4403 Unauthorized` (WS close); JSON:API `include` of non-loaded relation → `RelationNotLoaded` (not silent omit) | Medium | P1 |

Fuzz and corpus notes: path traversal probes use the canonical traversal fixture set (`..`, `..%2f`, `..\\`, long `../` chains). Deserialization corpus includes PO-style gadget payloads encoded as JSON (should never deserialize to a non-allow-listed type). CSRF corpus covers `same-origin` / `same-site` / `cross-site` / `none` plus absent header.

---

## 9. Execution Plan, Regression Priorities, and Environments

### 9.1 Ordering

1. Linter + typecheck gate (`rustfmt --check`, `clippy -- -D warnings`, `cargo check --workspace`) — blocks before any test execution (per global authority).
2. Scope tests to changed crates + dependents (never `cargo test --workspace` unfiltered in CI — scoped by crate graph).
3. Database → Service → State → UI inside each scoped crate (per 4-concern rule).
4. NFR smoke (see `qa-design.md` §Smoke) after functional suites; chaos slice runs nightly, not per-PR.
5. Contract snapshots reviewed via `cargo insta review` before merge.

### 9.2 Regression Priorities (from `test-planning/rules/regression-priority.md`)

| Priority | Meaning | Examples | Run When |
|----------|---------|----------|----------|
| P0 | Revenue/operability/auth — blocks release | M0 boot, M1 domain routing, M2 `upsert` strict, M3 CSRF/allow-list/`argon2`, M4 `Queue::route`/`Lock`, M6 `Storage::path()` confinement | Every PR |
| P1 | Major feature / cross-crate contract | `route:list` snapshots, JSON:API media type + sparse fieldset, vector dimension checks, queue metrics, schedule pause/resume | Every PR touching that crate |
| P2 | Edge / minor / integration matrix rows | `insertOrIgnoreReturning`, `whereBinary`, `onOneServer`, `copy_back`, CORS edge origins | Nightly + PR if matrix touched |
| P3 | Cosmetic / low-impact | Paginator `bootstrap-3` view, `table` rendering, `Str` factory demo names | Weekly |
| P4 | Nice-to-have | AI provider reranking/files/vector-stores beyond top 3 providers | Weekly + pre-tag only |

Parallel execution: unit shards by module; integration shards by driver (`postgres` | `mysql` | `sqlite` + `pgvector` flag gate); max parallelism bounded by available `testcontainers` ports (randomized). Flaky policy: quarantine on 3rd `FLAKE` occurrence with `DEBT-` task; never re-runs to "green".

### 9.3 Environments

| Env | DB | Cache/Queue | Use |
|-----|----|-------------|-----|
| Unit/Isolated | none / `SqlitePool::connect(":memory:")` | `moka` in-process | CI per-PR, <30 s suite |
| Integration | `testcontainers` Postgres (+ `pgvector` extension image) + MySQL + SQLite file | `moka` + `deadpool-redis` via `testcontainers` Redis | CI per-PR, scoped by crate |
| E2E / BDD | Same as integration but with `axum` test server (ephemeral port) + `wiremock` upstreams | Same | CI per-PR for happy paths, nightly for full Gherkin |
| Perf | Postgres+Redis local (not container) for stable timings | As above | Nightly / tagged |
| Chaos | Killable containers (`testcontainers` with SIGKILL + network partition via `tc`) | — | Nightly |

---

## 10. Entry / Exit Criteria, Defect Handling, and Gates

### 10.1 Entry

- `task-write` for the crate is `in_progress`, its `standard-read` gate passed.
- Lint/typecheck/format gate green (no bypass via `--no-verify` without ADR).

### 10.2 Exit (per crate)

- All P0–P1 cases for that crate green on scoped run (or explicitly quarantined as `FLAKE`/`PRE_EXISTING` with `DEBT-` task).
- Coverage line ≥85%, branch ≥80% (workspace aggregate ≥80%); mutation ≥65% when applicable.
- Contract snapshots approved (`cargo insta` no pending).
- Security triage Sec-01..09 verified (or N/A justified with ADR — e.g., no `Storage` in M2 crate).
- `task-write` comment "Completed: ..." with files changed + verdict persisted.

### 10.3 Defects

Every failure is classified `REAL_BUG | PRE_EXISTING | FLAKE | TEST_BUG | ENV_ERROR` per tester contract. Only `REAL_BUG`/`TEST_BUG` create `FIX-` tasks (priority 4–5, `phase: Testing`, routed to owning crate skill). `FLAKE` with >2 occurrences → quarantine + `DEBT-` task (priority 3–4). Environment issues reported to orchestrator, not filed as tasks.

---

## 11. Traceability and Deliverable Map

Deliverables from this task:

| Deliverable | Purpose | Trace |
|-------------|---------|-------|
| This file (`test-plan.md`) | Pyramid + 4-concern per milestone + NFR + security triage | All BR/PRD FRs + PRD §8 Laravel 13 matrix |
| `test-cases.md` | Per-crate test case catalog (EP/BVA/Decision Table/State Transition) | Each FSD `FS-M*-*` + story `US-M*-*` + Gherkin tags `@milestone-m*` |
| `qa-design.md` | QA scenario buckets, smoke design, boundary taxonomy, non-functional suites (load, observability, chaos) | NFR-*, Sec-*, PRD §8 BDD tags |
| `application/testing/*` stubs per module | Executable skeletons tracing to the catalogs above | Per-crate `FS-M*-*` + FR tag |

Every Laravel 13 feature #1–#20 appears in §5 and has ≥1 row in `test-cases.md` and ≥1 scenario bucket in `qa-design.md` (mirroring `prd.md` §8 and `bdd-scenarios.md` §3).

---

*Next: `test-cases.md` (case catalog) → `qa-design.md` (QA scenario + smoke) → `application/testing/*` (stubs per module). Updates to this plan require `TASK-010` comment.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
