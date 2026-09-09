# RustaSea — Test Case Design (per Crate)

> **Owner:** vheins/rustasea | **Phase:** Implementation P5 | **Task:** TASK-010 | **Date:** 2026-09-07
> **Parents:** `test-plan.md` (pyramid + 4-concern) · `brd.md` · `prd.md` (FR-000..612) · `fsd.md` (FS-M0-01..M6-07) · `user-stories.md` (US-M0-01..M6-07) · `bdd-scenarios.md` (33 features, Business language)
> **Techniques:** EP (Equivalence Partitioning), BVA (Boundary Value Analysis), Decision Tables, State Transition, Error Guessing — selected per feature per `test-planning/rules/test-design.md` and `test-planning/rules/boundary-taxonomy.md`.

**How to read:** Each table row is a test case (1 behavior). Columns: **Case** (`TC-{crate}-{seq}`), **FR** (PRD tag), **Concern** (DB/Service/State/UI), **Technique**, **Input / Precondition**, **Expected**, **Env**, **Priority** (P0..P4). Every row traces to an `fsd.md` FS and a `user-stories.md` Gherkin tag; tags in `bdd-scenarios.md` are shown where the Gherkin file covers the case more readably.

---

## M0 — Bootstrap & Core (`rustasea-foundation`, `rustasea-config`)

*Depends on nothing. FSD FS-M0-01..04. Stories US-M0-01/02.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin / Note |
|------|----|---------|-----------|----------------------|----------|-----|---|----------------|
| TC-M0-01 | FR-000, FS-M0-01 | State | State Transition | `bootstrap/app.rs` registers `A`, `B` where `B depends_on=[A]` | `A::register` before `B::register`; `A::boot` before `B::boot`; `AppState` available via `axum::extract::State` | Isolated | P0 | `@milestone-m0 @foundation` DAG-ordered boot |
| TC-M0-02 | FR-001, FS-M0-03 | Service | EP | `config/app.toml {port:3000}` + env `APP_PORT=4000` | `AppConfig::port == 4000` (env wins) | Unit | P0 | layered config precedence |
| TC-M0-03 | FR-001/007, FS-M0-03 | Service | BVA/Error Guessing | `config/database.toml` invalid TOML on line 7 | `ConfigError::Parse { file:"config/database.toml", line:7 }` diagnostic, no panic | Isolated | P0 | invalid config diagnostic |
| TC-M0-04 | FR-003, FS-M0-01 | State | State Transition | Provider `A→B`, `B→A` cycle registered | `BootError::Cycle { chain: ["A","B"] }`, app not `Running` | Unit | P0 | `@milestone-m0` Cycle rejected |
| TC-M0-05 | FR-004, FS-M0-04 | State | State Transition | `GET /slow` 3 s in-flight, `SIGTERM` with `shutdown_timeout 10s` | Request completes `200`, process exit `0`; timeout-expiry variant emits `ShutdownTimeout` | Integration | P0 | graceful drain |
| TC-M0-06 | FR-002, FS-M0-02 | Service | EP/BVA | `Singleton::<Counter>(0)` then `Make::<Counter>` twice | `Arc::ptr_eq(a,b)` is true; `Bind` variant yields `!ptr_eq` | Unit | P0 | Singleton identity |
| TC-M0-07 | FR-002, FS-M0-02 | Service | EP | No binding for `Mailer` | `Make::<Option<Mailer>> == None` (no auto-construct) | Unit | P0 | nullable-class `Option` |
| TC-M0-08 | FR-002 | Service | EP | No binding for `PaymentGateway` | `ContainerError::NotFound { type_name:"PaymentGateway" }` | Unit | P0 | NotFound |
| TC-M0-09 | FR-006, FS-M0-02 | Service | Error Guessing | Cache driver via `Manager::extend(|m,c| MyStore::new(m.prefix()))` | Closure sees manager prefix equal to registered prefix | Unit | P1 | `Manager::extend` bound closure |
| TC-M0-10 | FR-005, FS-M0-01 | UI | Decision Table | `cargo rustasea new demo` invoked | `demo/bootstrap/app.rs`, `config/`, `routes/web.rs`, `.env.example`, `Cargo.toml` exist; `cargo check` passes in scaffold | Isolated | P0 | scaffold |
| TC-M0-11 | FR-007/008 | Service | Error Guessing | Missing `config/app.toml` (defaults present), `AppState::app()` called from provider `boot` | Boots with defaults, same `Application` via `AppState::app()` | Unit | P2 | missing config fallback + App singleton |
| TC-M0-12 | FS-M0-04 | Service | BVA | `shutdown_timeout_secs = 1` and request 5 s in-flight | Exits with `1` after logging outstanding count (timeout path) | Integration | P1 | graceful drain timeout edge |

*4-concern gate M0:* DB 1 (migration-table existence via `TC-M0-10` + migration harness in M2), Service ≥5, State ≥3, UI 1 — all present above.

---

## M1 — Routing & HTTP (`rustasea-router`, `rustasea-http`, `rustasea-macros`)

*Depends on M0. FSD FS-M1-01..06. Stories US-M1-01..06.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin |
|------|----|---------|-----------|----------------------|----------|-----|---|---------|
| TC-M1-01 | FR-100, FS-M1-01 | Service | EP | `Route::resource("users", UserController)` | 7 CRUD routes (`index..destroy`) with paths `/users`/`/users/{id}` and conventional names | Unit | P0 | `@routing` resource helper |
| TC-M1-02 | FR-101, FS-M1-01 | UI | EP | Group `prefix("/api/v1")` with `get("/users")` | Canonical path `/api/v1/users` | Unit | P0 | group prefix |
| TC-M1-03 | FR-100, FS-M1-01 | State | State Transition | Two routes `name("users.index")` duplicate | `RouteError::Conflict { name:"users.index" }` at build | Unit | P0 | duplicate name |
| TC-M1-04 | FR-102, FS-M1-02 | State | Decision Table | Domain `{tenant}.example.com/*` + non-domain `/docs`; `GET docs.example.com/docs` | Tenant catch-all runs, `tenant="docs"` extracted | Integration | P0 | `@routing-validation` domain wins |
| TC-M1-05 | FR-102, FS-M1-02 | State | EP | Only `domain("{tenant}.example.com/dashboard")` + non-dom `/dashboard`; `GET example.com/dashboard` | Non-domain handler runs | Integration | P1 | non-domain fallback |
| TC-M1-06 | FR-103, FS-M1-03 | UI | Contract (snapshot) | Route `get("/users/{user:slug}")` | `route:list --json` entry contains `binding_fields:["slug"]` | Unit | P1 | `@observability-tooling` binding field |
| TC-M1-07 | FR-103, FS-M1-03 | UI | Contract | `route:list --json` | Valid JSON array; each entry has `middleware: []` array | Unit | P1 | machine-readable |
| TC-M1-08 | FR-104/306, FS-M1-04 | State | BVA | `throttle(per_minute:60).by_ip()` then 61 req from same IP in 60 s | 61st → `429` + `Retry-After` seconds to window reset | Integration | P0 | throttle |
| TC-M1-09 | FR-104, FS-M1-04 | UI | EP | `Cors::allow_origins(["https://app.example.com"])`, `Origin: https://app.example.com` | Response has `Access-Control-Allow-Origin: https://app.example.com` | Integration | P1 | CORS allow |
| TC-M1-10 | FR-104, FS-M1-04 | UI | EP | Same CORS, `Origin: https://evil.com` | No `Access-Control-Allow-Origin` header (or `403` in strict) | Integration | P1 | CORS deny |
| TC-M1-11 | FR-105/106, FS-M1-05 | UI | EP | `POST {name:"Ada ",email:"ada@example.com"}` to `Json<CreateUser>` | `201` with serialized user (name trimmed+validated) | Integration | P0 | typed extractor happy |
| TC-M1-12 | FR-105/308, FS-M1-05 | UI | EP | `POST {name:"Ada",email:"not-an-email"}` | `422` with `ErrorBag { email: ["must be valid email"] }` | Integration | P0 | `ErrorBag` 422 |
| TC-M1-13 | FR-105, FS-M1-05 | UI | Error Guessing | `POST` with unknown field `age:30` in strict deserialize mode | `422` with `age` listed as unexpected | Integration | P2 | strict unknown field |
| TC-M1-14 | FR-107, FS-M1-06 | Service | Decision Table | `Http::get(url).throw(|r| r.status().is_server_error())` against upstream `500` | `HttpError::Status { code:500 }` | Integration (wiremock) | P0 | `@http-client-process` throw on 5xx |
| TC-M1-15 | FR-107/108, FS-M1-06 | Service | BVA | Idle silence 6 s with `idle_timeout 5s` | `HttpError::Timeout { kind: Idle }` | Integration (wiremock) | P0 | idle timeout distinct |
| TC-M1-16 | FR-107, FS-M1-06 | Service | EP | Throw predicate `server_error` vs `always` with `422` upstream | `server_error+422→success`, `always+422→HttpError` | Unit | P1 | throw configurability |

---

## M2 — ORM & Database (`rustasea-orm`, `rustasea-macros`, `pgvector`)

*Depends on M0+M1. FSD FS-M2-01..06. Stories US-M2-01..06.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin |
|------|----|---------|-----------|----------------------|----------|-----|---|---------|
| TC-M2-01 | FR-200, FS-M2-01 | DB | EP | Config `driver: postgres` vs `sqlite`, same builder `where("id",1).first()` | Driver-specific SQL generated; driver health check passes | Integration | P0 | driver abstraction |
| TC-M2-02 | FR-201, FS-M2-02 | DB | BVA/EP | `#[derive(Model)]` with `#[soft_delete]`, snake_plural table `users` | Table `users` has `id, created_at, updated_at, deleted_at` with soft-delete semantics | Integration | P0 | Model derive |
| TC-M2-03 | FR-202, FS-M2-03 | Service | EP | Table with `{id:1,status:"active"}`, `where("status","active").first()` | `Some(User{id:1})` | Integration | P0 | `@orm` fluent filter |
| TC-M2-04 | FR-205, FS-M2-03 | State | State Transition | Two concurrent `transaction(|tx| select_for_update(1))` | Second blocks until first commits, no dirty read | Integration | P0 | transactional lock |
| TC-M2-05 | FR-202, FS-M2-03 | Service | EP | No row matching id | `firstOrFail() → QueryError::NotFound` | Unit | P0 | firstOrFail |
| TC-M2-06 | FR-203, FS-M2-03 | Service | BVA | 10k rows, `chunkBy("id",500, |chunk| ... )` | 20 callbacks, each `chunk.len()==500`, no OOM (cursor by index) | Integration | P0 | `@query-builder-additions` chunkBy |
| TC-M2-07 | FR-203, FS-M2-03 | Service | EP | `insertOrIgnoreReturning` with half conflicting rows | Only non-conflicting inserted; generated IDs returned | Integration | P0 | insertOrIgnoreReturning |
| TC-M2-08 | FR-203, FS-M2-03 | Service | EP | `whereBinary("blob_col", &bytes)` on non-binary column (Postgres) | `QueryError::IncompatibleColumn` surfaced | Integration | P2 | whereBinary |
| TC-M2-09 | FR-204, FS-M2-04 | Service | EP | `upsert(rows, unique_by:[])` | `UpsertError::EmptyUniqueBy` before any DB round-trip; 0 rows written | Unit | P0 | `@upsert-delete` empty uniqueBy |
| TC-M2-10 | FR-204, FS-M2-04 | Service | EP | MySQL `join("orders").where("orders.status","expired").delete()` | SQL contains `DELETE users FROM users JOIN orders` | Unit (sql snapshot) | P0 | MySQL delete JOIN |
| TC-M2-11 | FR-205, FS-M2-03 | Service | EP | `toSql` / `toRawSql` for known builder chain | Stable SQL string snapshot (insta) | Unit | P1 | sql snapshot |
| TC-M2-12 | FR-206, FS-M2-02 | UI | EP | Users with eager `posts:2`, `serde_json::to_string` then `from_str` | `posts.len()==2` and `relation_loaded("posts")==true` | Integration | P0 | `@collection-serialization` round-trip |
| TC-M2-13 | FR-206, FS-M2-02 | UI | BVA | User with 0 posts eagerly loaded | `posts == []` (empty vec, not None) | Integration | P1 | empty relation |
| TC-M2-14 | FR-208, FS-M2-05 | DB | Decision Table | `make:migration create_users_table` then `migrate` | `users` collection exists; `migrations` table records entry | Integration | P0 | migration create |
| TC-M2-15 | FR-208, FS-M2-05 | DB | State Transition | `migrate` then `migrate:fresh --seed` then `migrate` | Same version after re-run; idempotent | Integration | P0 | migrate idempotence |
| TC-M2-16 | FR-210, FS-M2-03 | Service | EP | `fetch_mode: FetchMode::Assoc` | Rows returned as associative maps | Unit | P3 | fetch modes |
| TC-M2-17 | FR-209, FS-M2-06 | Service | State Transition | `UserFactory::create(5)` in test A then `UserFactory::create(1)` in test B | Sequence resets to `user1@example.com` | Integration | P0 | `@orm` factory reset |
| TC-M2-18 | FR-209, FS-M2-06 | DB | EP | Missing migration for `posts`, then `Post::query().get()` | `QueryError::MissingTable { name:"posts" }` | Integration | P0 | missing table |
| TC-M2-19 | FR-207/602, FS-M2-06 | DB | EP | `vector(1536)` column on `products` with 100 embeddings | `whereVectorSimilarTo("embedding",&q,limit:10)` returns 10 rows ordered by cosine (`<=>`) | Integration (pgvector) | P0 | `@vector-search` nearest neighbor |
| TC-M2-20 | FR-207, FS-M2-06 | DB | Error Guessing | `CREATE EXTENSION vector` not installed on target PG | `PgVectorError::ExtensionMissing` with remediation hint | Integration | P0 | vector extension missing |
| TC-M2-21 | FR-207, FS-M2-06 | Service | Decision Table | Column dim 1536, query dim 768 (and reverse) | `VectorDimensionMismatch{expected:1536,actual:768}` | Unit | P0 | dimension mismatch |

*Migration test matrix:* each `FS-M2-05` migration file must pass **round-trip** (`up→down→up`) and **large-migration** (bulk via `migrate`); irreversible migrations must be marked and tested as error on `down`.

---

## M3 — Auth, Middleware & Validation (`rustasea-auth`, `rustasea-validation`, `rustasea-macros`)

*Depends on M1+M2. FSD FS-M3-01..06. Stories US-M3-01..05.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin |
|------|----|---------|-----------|----------------------|----------|-----|---|---------|
| TC-M3-01 | FR-300, FS-M3-01 | Service | EP | Correct `{email,password}` via JWT guard (`argon2` hash) then `parse(token)` | Token issued, `parse` yields `AuthUser{id}` | Integration | P0 | `@auth` login+parse |
| TC-M3-02 | FR-301, FS-M3-01 | State | EP | Only `jwt` guard registered, call `Auth::guard("api").user()` | `Error::GuardMismatch{expected:"jwt",actual:"api"}` | Unit | P0 | guard mismatch |
| TC-M3-03 | FR-300, FS-M3-01 | State | Decision Table | Wrong password / expired JWT / malformed JWT | `BadCredentials` / `ExpiredToken` / `InvalidToken` respectively | Unit | P0 | auth failures classified |
| TC-M3-04 | FR-302, FS-M3-02 | State | Decision Table | `POST /form` coverage — `Sec-Fetch-Site` × `Origin` × token validity (see matrix below) | Pass/reject per row (see matrix) | Integration | P0 | `@csrf-origin` (matrix) |
| TC-M3-05 | FR-303, FS-M3-03 | UI | EP | Default config, stored session entry | Key contains `-session-`, payload is JSON | Integration | P0 | session JSON+prefix |
| TC-M3-06 | FR-304, FS-M3-03 | Service | EP | `serializable_classes:["App::UserDto"]`, cached `AdminDto` then `get` | `SerializationError::NotAllowed{type_name:"AdminDto"}` before `deserialize` | Unit | P0 | allow-list deserialization |
| TC-M3-07 | FR-303, FS-M3-03 | Service | EP | Cache prefix for `redis` inspected | Prefix contains `-cache-` (not `_cache_`) | Unit | P1 | hyphenated prefix |
| TC-M3-08 | FR-305, FS-M3-06 | UI | EP | `#[middleware("auth:jwt")]` handler, guest request | `401 Unauthorized` | Integration | P0 | declarative middleware |
| TC-M3-09 | FR-307/309, FS-M3-05 | Service | EP/BVA | `role="Admin"` vs `contains_strict="admin"` | Validation fails on `role` (strict type/case) | Unit | P0 | strict containment |
| TC-M3-10 | FR-308/309, FS-M3-05 | UI | EP | Invalid `email` + too-short `password` | `422` with `ErrorBag {email:[..], password:[..]}` (both keys) | Integration | P0 | ErrorBag per field |
| TC-M3-11 | FR-305, FS-M3-06 | UI | State Transition | `#[authorize("update",User)]`, non-owner caller | `403` typed `AuthorizationError` before handler body | Integration | P0 | authorize gate |
| TC-M3-12 | FR-306, FS-M3-04 | State | BVA | `limit.per_minute(3).by_ip()`, 4th `POST /login` from `1.2.3.4` in 60 s | `429` + `Retry-After` | Integration | P0 | rate limit |
| TC-M3-13 | FR-306, FS-M3-04 | Service | Error Guessing | `trusted_proxies` unset, request has `X-Forwarded-For:9.9.9.9` from peer `1.2.3.4` | Rate-limit key is `1.2.3.4` (spoofed header ignored) | Unit | P0 | proxy-aware |
| TC-M3-14 | FR-311 | Service | EP | `markEmailAsUnverified` on verified user | `email_verified_at cleared` | Integration | P2 | MustVerifyEmail |

**CSRF Decision Table for TC-M3-04:**

| Row | `Sec-Fetch-Site` | `Origin` (for cross-site) | Token | Method | Expected |
|-----|------------------|---------------------------|-------|--------|----------|
| 1 | `same-origin` | — | valid | POST | `200` pass |
| 2 | `cross-site` | `https://evil.com` (not allowed) | valid | POST | `403 UntrustedOrigin` |
| 3 | `cross-site` | `https://app.example.com` (allowed) | valid | POST | `200` pass |
| 4 | absent | — | valid | POST | `200` (degrade to token) |
| 5 | `none` | — | valid | POST | `200` (treated as same-origin) |
| 6 | `cross-site` | evil | valid | GET/HEAD/OPTIONS | `200` (exempt) |

---

## M4 — Queue, Cache, Scheduling & Events (`rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule`)

*Depends on M0+M2+M3. FSD FS-M4-01..05. Stories US-M4-01..06.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin |
|------|----|---------|-----------|----------------------|----------|-----|---|---------|
| TC-M4-01 | FR-400, FS-M4-01 | Service | EP | `#[tries(3)] #[backoff(1s)] Flaky` fails on attempts 1–2 | Retried twice ~1 s apart, 3rd succeeds | Integration (sync driver) | P0 | retry policy |
| TC-M4-02 | FR-401, FS-M4-02 | State | EP | `Queue::route::<ProcessPodcast>(queue:"podcasts")`, dispatch without override | Placed on `podcasts` connection/queue | Integration | P0 | `@queue-routing` central |
| TC-M4-03 | FR-401, FS-M4-02 | State | EP | Same route, dispatch `onQueue("urgent")` | Placed on `urgent` (override wins) | Integration | P0 | override |
| TC-M4-04 | FR-401, FS-M4-02 | State | EP | Duplicate `Queue::route::<ProcessPodcast>(queue:"b")` during boot | `QueueError::DuplicateRoute{type_name}` | Unit | P0 | duplicate route |
| TC-M4-05 | FR-402, FS-M4-02 | State | State Transition | `chain([JobA,JobB,JobC])` where `JobB` fails | `JobC` not executed; `failed_jobs` has `JobB` exception | Integration | P0 | `@queue` chain |
| TC-M4-06 | FR-402, FS-M4-02 | DB | EP | `batch([Job{1},Job{2},Job{3}])` dispatched | `BatchId` returned; all enqueued | Integration | P1 | batch |
| TC-M4-07 | FR-402, FS-M4-02 | DB | State Transition | Failed job `abc` then `queue:retry abc` | Re-queued; `failed_jobs` cleared on success | Integration | P1 | failed retry |
| TC-M4-08 | FR-403, FS-M4-04 | Service | BVA | `put("k","v",60s)` then 30 s elapsed, `touch("k",120s)` then wait 90 s | `get("k")==Some("v")` (TTL extended from touch point) | Integration | P0 | `@cache-touch` TTL |
| TC-M4-09 | FR-403, FS-M4-04 | Service | EP | `touch("k",60s)` with `k` missing | Returns `false` (not error) | Unit | P0 | touch missing |
| TC-M4-10 | FR-405, FS-M4-04 | State | BVA | `Lock("billing",10s)` held by A, B calls `block(2s)` | B waits ≤2 s then `LockError::AlreadyHeld` | Integration (redis+memory) | P0 | distributed lock |
| TC-M4-11 | FR-404, FS-M4-04 | Service | Decision Table | Stores `redis` vs `memory` isolation | `put` in `redis` not visible in `memory` (and reverse) | Integration | P0 | cache store isolation |
| TC-M4-12 | FR-406, FS-M4-03 | State | EP | `Listener SendMail { queue:true }` + `dispatch(UserCreated)` | Listener enqueued as job (not inline) | Integration | P0 | `@contracts-expansion` async |
| TC-M4-13 | FR-406, FS-M4-03 | State | State Transition | Handler defers `AnalyticsFlushed` via `dispatchAfterResponse` | Dispatched after `200` response sent (spy observed post-response) | Integration | P0 | dispatchAfterResponse |
| TC-M4-14 | FR-406, FS-M4-03 | UI | Contract | Inspect `JobAttempted` and `QueueBusy` event types | Fields `exception` vs `exceptionOccurred`, `connectionName` vs `connection` | Unit | P1 | contract renames |
| TC-M4-15 | FR-407/410, FS-M4-05 | State | State Transition | `everyMinute` + `skipIfStillRunning` 80 s job, tick at 60 s | Tick skipped | Integration | P1 | skipIfStillRunning |
| TC-M4-16 | FR-407, FS-M4-05 | State | EP | `onOneServer` with two scheduler nodes ticking `daily` | Exactly one node dispatches (distributed `Lock` acquisition) | Integration | P1 | onOneServer |
| TC-M4-17 | FR-408, FS-M4-05 | State | State Transition | `schedule:pause` while `email:send everyMinute` | `SchedulePaused` emitted, next tick not dispatched; `resume→ScheduleResumed` | Integration | P0 | `@schedule-pauseresume` |
| TC-M4-18 | FR-408, FS-M4-05 | State | Error Guessing | Pause while job mid-execution | Running job completes; next tick suppressed | Integration | P1 | pause during exec |
| TC-M4-19 | FR-409, FS-M4-03 | Service | BVA | Queue `podcasts` with 42 pending jobs | `pendingSize("redis","podcasts")==42` | Integration | P0 | `@queue-metrics` |
| TC-M4-20 | FR-409, FS-M4-03 | Service | EP | Oldest job at `2026-09-07T10:00:00Z`, then request `creationTimeOfOldestPendingJob` | RFC3339 timestamp returned; empty queue → `None` | Integration | P0 | oldest timestamp |
| TC-M4-21 | FR-409, FS-M4-03 | Service | Decision Table | Queue has 10 pending, 2 delayed, 1 reserved | Each metric returns corresponding count | Integration | P1 | queue states |

---

## M5 — DX, CLI & Testing (`rustasea-cli`, `rustasea-macros`, `rustasea-testing`)

*Depends on M0..M4. FSD FS-M5-01..04. Stories US-M5-01..04.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin |
|------|----|---------|-----------|----------------------|----------|-----|---|---------|
| TC-M5-01 | FR-500/502, FS-M5-01 | UI | Contract | `cargo rustasea list --json` | JSON with `make:controller` + `migrate` + `usage` strings | Isolated | P0 | `@generators @attributes` |
| TC-M5-02 | FR-502, FS-M5-01 | UI | EP | `AppSend` with `#[usage("app:send {user}")]` then `list --help` | Help line contains `app:send {user}` | Unit | P1 | usage |
| TC-M5-03 | FR-503, FS-M5-01 | UI | State Transition | Command asks `confirm("Proceed?")`, user answers `n` | Command aborts, exit `1` | Isolated | P1 | prompt abort |
| TC-M5-04 | FR-505, FS-M5-01 | Service | EP | `Artisan::call("migrate", vec![])` in-process | Migration runs without subprocess (spy verifies) | Integration | P1 | Artisan::call |
| TC-M5-05 | FR-502, FS-M5-01 | UI | EP | Command marked `#[hidden]` then `list` without `--all` | Command absent from output | Unit | P2 | hidden |
| TC-M5-06 | FR-500, FS-M5-01 | UI | Error Guessing | Unknown command `make:controll` / `migrat` | Suggestion `did you mean make:controller / migrate` (strsim) | Unit | P2 | unknown suggestion |
| TC-M5-07 | FR-501, FS-M5-02 | UI | Contract + Boundary | `make:controller UserController` in scaffold | `app/http/controllers/user_controller.rs` exists; `rustfmt --check` + `clippy -- -D warnings` pass | Isolated | P0 | gen controller |
| TC-M5-08 | FR-501, FS-M5-02 | UI | Contract | `make:model Post -m` | `app/models/post.rs` with `#[derive(Model)]` + migration `*_create_posts_table.rs` | Isolated | P0 | gen model+migration |
| TC-M5-09 | FR-501, FS-M5-02 | State | EP | `make:model Post` without `--force` when `app/models/post.rs` exists | `GeneratorError::AlreadyExists{path}` | Isolated | P0 | duplicate gen |
| TC-M5-10 | FR-501, FS-M5-02 | UI | BVA/Contract | `make:{job|event|listener} ...` (matrix of kinds) | File `<path>` per row; formatted+lint-clean | Isolated | P1 | other generators |
| TC-M5-11 | FR-506, FS-M5-03 | Service | State Transition | `#[tries(3)] MyJob` always fails, sync driver | 3 attempts then `failed_jobs` entry | Integration | P0 | tries attr |
| TC-M5-12 | FR-506, FS-M5-03 | Service | Decision Table | `#[tries(3)]` + `ShouldRetry→false` on same job | Attribute wins, warning `tries attribute shadows ShouldRetry` | Unit | P1 | attr shadowing |
| TC-M5-13 | FR-507, FS-M5-04 | DB | State Transition | Two `TestCase` with `testcontainers` PG, run `--test-threads=2` | Distinct random ports per test binary, no collision | Integration | P0 | isolated store |
| TC-M5-14 | FR-508, FS-M5-04 | Service | BVA | `Factory::sequence` at 10 in test A, `UserFactory::create(1)` in test B | Email `user1@example.com` (reset) | Integration | P0 | Str reset |
| TC-M5-15 | FR-509, FS-M5-04 | UI | EP | `paginate(15)` view `bootstrap-3` rendered | Paginator HTML with `bootstrap-3` classes | Unit | P3 | paginator view |
| TC-M5-16 | FS-M5-04 | Service | Error Guessing | `testcontainers` startup timeout `30s` expired | `TestError::ContainerTimeout` | Integration | P1 | container timeout |
| TC-M5-17 | FS-M5-04 | State | EP | After `cargo test` completes | No container `rustasea-test-*` left running | Integration | P0 | teardown |

*Generator matrix for TC-M5-10:*

| kind | name | path |
|------|------|------|
| `job` | `SendEmail` | `app/jobs/send_email.rs` |
| `event` | `UserCreated` | `app/events/user_created.rs` |
| `listener` | `SendWelcome` | `app/listeners/send_welcome.rs` |

*Also matrix: `agent`/`tool` generators covered in M6 — same verification shape.*

---

## M6 — Advanced (`rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai`)†

*Depends on M1..M5. FSD FS-M6-01..07. Stories US-M6-01..07. Feature-flagged `ai`/`broadcast`/`storage`.*

| Case | FR | Concern | Technique | Input / Precondition | Expected | Env | P | Gherkin |
|------|----|---------|-----------|----------------------|----------|-----|---|---------|
| TC-M6-01 | FR-600, FS-M6-01 | UI | State Transition | `UserCreated` on `private-chat.1`, user1 authorized (`Authorize` gate allow) + WS subscribe | Subscriber receives broadcast | Integration (WS) | P0 | `@broadcast` authorized |
| TC-M6-02 | FR-600, FS-M6-01 | State | EP | Same channel, user2 not authorized | `BroadcastError::Unauthorized{channel:"private-chat.1"}` + WS close `4403` | Integration | P0 | unauthorized |
| TC-M6-03 | FR-601, FS-M6-01 | UI | Contract | `Response::eventStream(stream_of(["hello","world"]))` | `Content-Type: text/event-stream`, two `data:` frames | Integration | P0 | SSE |
| TC-M6-04 | FR-603, FS-M6-02 | Service | State Transition | Read-through `primary:s3, fallback:local`, file `a/b.txt` only on `local` | Bytes from `local` returned | Integration (object_store fake) | P0 | `@storage-readthrough` fallback |
| TC-M6-05 | FR-611, FS-M6-02 | State | Boundary/Error Guessing | `Storage::path("../../etc/passwd")` plus corpus `..%2f`, `..\\`, long `../×N` | Every probe → `StorageError::PathTraversal` without FS access | Unit | P0 | traversal corpus |
| TC-M6-06 | FR-603, FS-M6-02 | State | State Transition | `copy_back:true` then `get("a/b.txt")` after fallback read | File now also on `primary:s3` | Integration | P1 | copy-back |
| TC-M6-07 | FR-603, FS-M6-02 | Service | EP | No store contains `<path>` (`a/b.txt`, `missing.bin`) | `NotFound{path}` uniformly | Unit | P1 | missing file |
| TC-M6-08 | FR-604, FS-M6-03 | UI | Contract | `UserResource::new(user).include("posts").fields(["name"])` with eager `posts` | `Content-Type: application/vnd.api+json`; `data.attributes=={name}` only; `included` has posts | Integration | P0 | `@jsonapi` fieldset+include |
| TC-M6-09 | FR-604, FS-M6-03 | State | EP | Same `include("posts")` but `posts` not eager-loaded | `JsonApiError::RelationNotLoaded{relation:"posts"}` | Unit | P0 | not loaded |
| TC-M6-10 | FR-604, FS-M6-03 | UI | EP/Contract | Fields `name` vs `name,email` | Visible attributes exactly match requested fieldset | Unit | P1 | sparse fieldset matrix |
| TC-M6-11 | FR-605, FS-M6-04 | State | State Transition | Queued `WelcomeNotification(user 9)` with `#[deleteWhenMissingModels]`, user 9 deleted before process | Skipped, `NotificationSkipped{MissingModel}`, no retry | Integration | P1 | missing-model suppression |
| TC-M6-12 | FR-605, FS-M6-04 | State | EP | Same notification where user exists | Notification delivered | Integration | P2 | delivered |
| TC-M6-13 | FR-606, FS-M6-05 | Service | Contract | `Ai::provider("openai").text("hello")` vs same call `provider("anthropic")` | Same `AiResponse{text,usage,tool_calls}` shape | Unit (fake adapter) | P0 | `@ai-sdk` provider switch |
| TC-M6-14 | FR-606, FS-M6-05 | Service | Decision Table | Provider×capability matrix (ollama×`reranking`, groq×`files`) | `AiError::UnsupportedCapability{provider,capability}` | Unit | P1 | unsupported capability |
| TC-M6-15 | FR-606/612, FS-M6-05 | Service | EP | Feature flag `ai` disabled, workspace `rustasea-router` only then `cargo check` | `async-openai`/provider crates absent from dep graph | Isolated | P0 | feature-gated dep |
| TC-M6-16 | FR-606, FS-M6-05 | Service | EP | Each of 12 provider names requested | Adapter available (per-row existence) | Unit | P0 | 12 providers |
| TC-M6-17 | FR-607/610, FS-M6-06 | Service | State Transition | `SupportAgent` with `SearchDocs` tool, prompt `"summarize ticket 42"`, streaming enabled + WS subscriber | `SearchDocs::call` invoked; chunks stream `event:token` in order to subscriber | Integration (WS+AI fake) | P0 | `@ai-agents` stream |
| TC-M6-18 | FR-608, FS-M6-06 | UI | Contract | `make:agent SupportAgent` | `app/ai/agents/support_agent.rs` with `Agent` marker + formatted+clean | Isolated | P0 | make:agent |
| TC-M6-19 | FR-607, FS-M6-06 | UI | State Transition | `ParentAgent` with `KnowledgeAgent` sub-agent + `Logging` middleware triggered as tool | Middleware observes both parent and sub-agent calls | Integration | P1 | sub-agent+middleware |
| TC-M6-20 | FR-607, FS-M6-06 | Service | State Transition | Agent with deferred `SimilaritySearch` loader, then prompt | Documents fetched via `whereVectorSimilarTo` before tool invoked | Integration | P1 | deferred loader |
| TC-M6-21 | FR-609, FS-M6-06 | Service | EP | `mcp` feature off, agent requests MCP discovery | `AgentError::McpUnavailable` with hint | Unit | P1 | MCP gate |
| TC-M6-22 | FR-610, FS-M6-06 | State | EP | Agent streaming 1000 tokens, WS subscriber | Chunks in order with `event: token` frames | Integration | P0 | broadcast streaming |
| TC-M6-23 | FR-607, FS-M6-06 | State | EP | Agent whose tool calls configured `queue:true`, invoke tool | Tool call enqueued as Job | Integration | P1 | queued tools |
| TC-M6-24 | FR-602/207, FS-M6-07 | Service | EP | `Str::toEmbeddings("hello", provider:"openai")` with embedding model `text-embedding-3-small` | `Vec<f32>` dim 1536 | Integration (AI fake) | P0 | toEmbeddings |
| TC-M6-25 | FR-602, FS-M6-07 | DB | State Transition | Vector index `products_embedding_index` exists, then `dropVectorIndex("embedding")` | `DROP INDEX` succeeds; subsequent `whereVectorSimilarTo` still returns (seq scan) | Integration | P0 | dropVectorIndex |
| TC-M6-26 | FR-602, FS-M6-07 | Service | EP | Embedding provider `openai` vs `gemini` requested | Embedding adapter available | Unit | P1 | embedding providers |

*Provider matrix for TC-M6-16 (one test per row via `TestMatrix` parameterized `rstest`):* `openai, anthropic, gemini, azure, bedrock, groq, xai, deepseek, mistral, ollama, openrouter, openai_compatible`.

---

## Cross-Cutting Cases (NFR + Contract + Migration)

| Case | FR/NFR | Concern | Technique | Input | Expected | Env | P |
|------|--------|---------|-----------|-------|----------|-----|---|
| TC-MIG-01 | FS-M2-05 / FR-208 | DB | State Transition | Each migration up→down→up | Reversible; bulk `migrate` idempotent | Integration | P0 |
| TC-MIG-02 | FS-M2-05 | DB | EP | Irreversible migration attempts `down` | `MigrationError::Irreversible{name}` | Unit | P1 |
| TC-CTR-01 | FR-103 / FS-M1-03 | UI | Contract | `route:list --json` snapshot vs JSON-Schema | Schema valid; snapshot approved | Unit | P1 |
| TC-CTR-02 | FR-604 / FS-M6-03 | UI | Contract | JSON:API document vs JSON:API 1.1 schema | Valid `data/included/links/meta`, correct media type/headers | Unit | P1 |
| TC-CTR-03 | FR-406 / FS-M4-03 | State | Contract | Queue payload `serde_json` round-trip | Same `Job<T>` payload after serialize/deserialize | Unit | P1 |
| TC-NFR-01 | NFR-Per-01 | Service | BVA | Cold boot M0 with 5 providers, `cargo bench --bench boot` | p50 <2 s on CI 2 vCPU | Bench | P1 |
| TC-NFR-02 | NFR-Per-02 | Service | BVA | `GET /users` (axum hello) 1k RPS `oha -n 10000 -c 100` | p95 <50 ms (no DB) | Bench | P1 |
| TC-PROP-01 | FR-206 / FS-M2-02 | Service | Property (round-trip) | Arbitrary relation size 0..20 | `serde` round-trip preserves loaded relations | Property (`proptest`) | P1 |

---

## Traceability Summary

Every Laravel 13 feature #1–#20 has ≥1 case above (mirrors `prd.md` §8 · `fsd.md` §6 · `bdd-scenarios.md` §3):

| # | Feature | Cases | Tags |
|---|---------|-------|------|
| 1 | AI SDK | TC-M6-13..16 | `@ai-sdk` |
| 2 | AI Agents | TC-M6-17..23, TC-M5-10 | `@ai-agents` |
| 3 | JSON:API | TC-M6-08..10, TC-CTR-02 | `@jsonapi` |
| 4 | Queue Routing | TC-M4-02..04 | `@queue-routing` |
| 5 | Cache `touch()` | TC-M4-08/09 | `@cache-touch` |
| 6 | Vector Search | TC-M2-19..21, TC-M6-24..26 | `@vector-search` |
| 7 | Expanded Attributes | TC-M3-08..11, TC-M4-01, TC-M5-11/12, TC-M5-01/02 | `@attributes` |
| 8 | Cloud Queue metrics | TC-M4-19..21 | `@queue-metrics` |
| 9 | Read-through FS | TC-M6-04..07 | `@storage-readthrough` |
| 10 | Schedule Pause/Resume | TC-M4-17/18, TC-M4-15/16 | `@schedule-pauseresume` |
| 11 | Origin-aware CSRF | TC-M3-04 | `@csrf-origin` |
| 12 | Cache/Session hardening | TC-M3-05..07, TC-M4-11 | `@cache-session-hardening` |
| 13 | Collection serialization | TC-M2-12/13, TC-PROP-01 | `@collection-serialization` |
| 14 | Upsert/Delete | TC-M2-09/10 | `@upsert-delete` |
| 15 | Builder additions | TC-M2-06..08, TC-M2-11 | `@query-builder-additions` |
| 16 | Event/Queue contracts | TC-M4-12..14, TC-M6-03 | `@contracts-expansion` |
| 17 | Mail/Notification | TC-M6-11/12 | `@mail-notifications` |
| 18 | HTTP Client & Process | TC-M1-14..16 | `@http-client-process` |
| 19 | Routing & Validation | TC-M1-04/05, TC-M3-09/10, TC-M1-12 | `@routing-validation` |
| 20 | Observability & Tooling | TC-M1-06/07, TC-M3-13, TC-M5-01/13..15, TC-CTR-01 | `@observability-tooling` |

*Counts:* M0 12 · M1 16 · M2 21 · M3 14 · M4 21 · M5 17 · M6 26 · cross 7 = **134 test cases** plus decision-table row expansions.

*Sources:* `test-plan.md` §4 pyramid and §5 4-concern matrix are the normative gate; rows above implement that gate per crate. Parametrized matrices use `rstest`/`proptest` — each logical row is a distinct `cargo test` case.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
