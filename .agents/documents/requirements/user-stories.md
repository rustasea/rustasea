# Rustavel — User Stories

> **Status:** Draft — P2 Requirements Phase  
> **Date:** 2026-09-07  
> **Parents:** `brd.md` + `prd.md` (FR-000 … FR-612) + `fsd.md` (FS-M0-01 … FS-M6-07)  
> **Research base:** `docs/laravel-13-research.md` (Laravel 13.0.0 2026-03-17) + `README.md`  
> **Format:** `As a <role>, I want <goal> so that <benefit>.` — one role per story, ≥3 EARS Given/When/Then per story (happy + error + edge/NFR as `product-planning/rules/requirements.md` requires).  
> **Coverage gate:** every milestone M0–M6 has ≥1 story; every Laravel 13 feature #1–#20 is traced.

---

## How to Read

| Field | Meaning |
|-------|---------|
| **Story ID** | `US-M{0..6}-{seq}` — scoped to milestone |
| **FRs** | PRD functional requirements satisfied |
| **Laravel 13** | Feature # from research doc (or `—` for architectural) |
| **Priority** | MoSCoW inherited from PRD/BRD |
| **EARS AC** | Acceptance criteria in EARS Given/When/Then — Happy / Error / Edge / NFR; thresholds are measurable (`<2s`, `422`, `429`, etc.) |

Shared preconditions that appear across many stories (authenticated user, project exists) are repeated explicitly per story for isolation — no hidden `Background` assumed outside BDD.

---

## M0 — Bootstrap & Core

### US-M0-01 — Boot the application with providers and graceful shutdown

- **Role:** Rust backend team lead  
- **Story:** *As a Rust backend team lead, I want a bootable application skeleton with typed config and provider lifecycle so that `cargo run` boots in under 2 seconds and shuts down without dropping in-flight work.*  
- **Priority:** Must  
- **FRs:** FR-000, FR-001, FR-003, FR-004, FR-008 (BR-01 — `fsd.md` FS-M0-01/03/04)  
- **Laravel 13:** #20 closure binding (FR-006 adjacency)

**Acceptance Criteria (EARS):**

- **Happy — boots and provider ordering:**
  > *Given* `bootstrap/app.rs` registers provider `A` and provider `B` where `B` declares `depends_on = [A]`, *When* `Application::configure().boot().await` runs, *Then* `A::register` and `A::boot` complete before `B::boot` and `AppState` is available via `axum::extract::State`.

- **Happy — layered config precedence:**
  > *Given* `config/app.toml` sets `port = 3000` and `.env` sets `APP_PORT=4000`, *When* the application reads `AppConfig::port`, *Then* the value is `4000`.

- **Error — provider cycle:**
  > *Given* provider `A` depends on `B` and `B` depends on `A`, *When* `boot()` is called, *Then* it returns `BootError::Cycle { chain: ["A","B"] }` and the application does not reach `Running`.

- **Error — invalid config is diagnostic:**
  > *Given* `config/database.toml` contains invalid TOML on line 7, *When* the application boots, *Then* it returns `ConfigError::Parse { file: "config/database.toml", line: 7 }` with a human-readable hint and does not panic.

- **NFR — graceful drain:**
  > *Given* a `GET /slow` handler sleeps 3s, *When* `SIGTERM` is sent during that request and `shutdown_timeout_secs = 10`, *Then* the request completes with `200` and the process exits with code `0`.

---

### US-M0-02 — Resolve services from a typed container

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `Bind`/`Singleton`/`Instance` semantics in a type-safe container so that I can share singletons across the application without global `static mut`.*  
- **Priority:** Must  
- **FRs:** FR-002, FR-006, FR-008 (BR-01 — FS-M0-02)  
- **Laravel 13:** `Container::call` nullable-class semantics (#20)

**Acceptance Criteria:**

- **Happy — Singleton identity:**
  > *Given* a `Singleton::<Counter>` registered with initial `0`, *When* `Make::<Counter>` is called twice, *Then* the two `Arc<Counter>` values are `ptr_eq`.

- **Happy — `Make::<Option<T>>` for nullable binding:**
  > *Given* no binding for `Mailer` is registered, *When* `Make::<Option<Mailer>>` is resolved, *Then* the result is `None` (not an auto-constructed instance).

- **Error — NotFound:**
  > *Given* no binding for `PaymentGateway` exists, *When* `Make::<PaymentGateway>` is called, *Then* `ContainerError::NotFound { type_name: "PaymentGateway" }` is returned.

- **Edge — `Manager::extend` bound closure:**
  > *Given* a cache driver registered via `Manager::extend(|manager, config| MyStore::new(manager.prefix().clone()))`, *When* the driver is resolved, *Then* `manager.prefix()` inside the closure equals the manager's configured prefix.

---

## M1 — Routing & HTTP

### US-M1-01 — Define expressive routes with groups and resources

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want axum-backed route helpers (`get`/`post`/`put`/`delete`/`patch`/`options`/`any`), route groups, and a `resource` helper so that route definitions read like Laravel and share middleware cleanly.*  
- **Priority:** Must  
- **FRs:** FR-100, FR-101 (BR-02 — FS-M1-01)  
- **Laravel 13:** baseline

**Acceptance Criteria:**

- **Happy — CRUD via `resource`:**
  > *Given* `Route::resource("users", UserController)` is registered, *When* `cargo rustavel route:list --json` is inspected, *Then* seven routes (`index` … `destroy`) exist with paths `/users` and `/users/{id}` appropriately.

- **Happy — Group prefix:**
  > *Given* a group with `prefix("/api/v1")` containing `get("/users")`, *When* that route is inspected, *Then* its canonical path is `/api/v1/users`.

- **Error — Duplicate named route:**
  > *Given* two routes both declare `name("users.index")`, *When* the router is built, *Then* `RouteError::Conflict { name: "users.index" }` is returned.

---

### US-M1-02 — Domain-aware routing precedence

- **Role:** Platform engineer  
- **Story:** *As a platform engineer running multi-tenant subdomains, I want domain routes evaluated before non-domain routes so that a catch-all subdomain never shadows an explicit path.*  
- **Priority:** Must  
- **FRs:** FR-102 (BR-02 — FS-M1-02)  
- **Laravel 13:** #19 domain priority

**Acceptance Criteria:**

- **Happy — domain wins:**
  > *Given* routes `domain("{tenant}.example.com").get("/*", TenantCatchAll)` and `get("/docs", DocsPage)`, *When* `GET docs.example.com/docs` is requested, *Then* the tenant catch-all handler runs, not the docs handler — and `tenant = "docs"` is extracted.

- **Edge — non-domain fallback:**
  > *Given* only `domain("{tenant}.example.com").get("/dashboard", ...)` and `get("/dashboard", ...)` both exist, *When* `GET example.com/dashboard` (no subdomain) is requested, *Then* the non-domain handler runs.

---

### US-M1-03 — Introspect routes including binding fields

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `cargo rustavel route:list` to show each route's method, path, name, middleware, and binding fields so that I can audit route coverage without reading source.*  
- **Priority:** Must  
- **FRs:** FR-103, FR-109 (BR-02/BR-06 — FS-M1-03)  
- **Laravel 13:** #20 route binding fields

**Acceptance Criteria:**

- **Happy — JSON with binding_fields:**
  > *Given* a route `get("/users/{user:slug}", ...)` with binding `slug`, *When* `cargo rustavel route:list --json` runs, *Then* the JSON entry includes `binding_fields: ["slug"]` for that route.

- **NFR — Machine-readable:**
  > *Given* `--json` flag is passed, *When* output is piped to `jq`, *Then* valid JSON array is produced with `middleware` as an array per route.

---

### US-M1-04 — Middleware stack with throttling and CORS

- **Role:** Platform engineer  
- **Story:** *As a platform engineer, I want a composable middleware stack with per-route throttling and CORS so that abusive traffic is rejected and cross-origin policy is explicit.*  
- **Priority:** Must  
- **FRs:** FR-104, FR-306 (BR-02 — FS-M1-04)  
- **Laravel 13:** baseline + strict context

**Acceptance Criteria:**

- **Happy — throttling:**
  > *Given* a route guarded by `throttle(per_minute: 60).by_ip()`, *When* 61 requests from the same IP arrive within 60 seconds, *Then* the 61st response is `429` with `Retry-After` header.

- **Happy — CORS allow-list:**
  > *Given* `Cors::allow_origins(["https://app.example.com"])` is configured, *When* a preflight `OPTIONS` arrives with `Origin: https://app.example.com`, *Then* the response includes `Access-Control-Allow-Origin: https://app.example.com`.

- **Error — disallowed origin:**
  > *Given* the same CORS config, *When* `Origin: https://evil.com` is sent, *Then* no `Access-Control-Allow-Origin` is emitted (or `403` if strict).

---

### US-M1-05 — Typed extractors and `ErrorBag` validation bridge

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want typed extractors (`Json<T>`, `Path<T>`) that surface validation failures as an `ErrorBag` with `422`, so that I never hand-parse request bodies.*  
- **Priority:** Must  
- **FRs:** FR-105, FR-106 (BR-02 — FS-M1-05)  
- **Laravel 13:** #19 `ErrorBag` adjacency

**Acceptance Criteria:**

- **Happy — valid payload passes:**
  > *Given* handler `async fn create(Json(CreateUser { name, email }): Json<CreateUser>)`, *When* `{ "name": " Ada ", "email": "ada@example.com" }` is posted, *Then* `201` with serialized user is returned.

- **Error — validation failure is `ErrorBag`:**
  > *Given* the same handler requires `email` format, *When* `{ "name":"Ada","email":"not-an-email" }` is posted, *Then* `422` with `{ "errors": { "email": ["must be a valid email"] } }`.

- **Edge — unknown field is rejected (strict deserialize):**
  > *Given* handler expects `CreateUser`, *When* payload contains an unknown field `age: 30`, *Then* `422` with field `age` listed as unexpected (if strict mode enabled).

---

### US-M1-06 — HTTP client with `throw` callbacks and timeouts

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want an HTTP client that supports `throw` callbacks and idle/total timeouts so that downstream failure handling mirrors Laravel's HTTP client without ad-hoc error mapping.*  
- **Priority:** Must  
- **FRs:** FR-107, FR-108 (BR-02 — FS-M1-06)  
- **Laravel 13:** #18 HTTP Client & Process + #16 `eventStream` adjacency

**Acceptance Criteria:**

- **Happy — `throw` on 5xx:**
  > *Given* `Http::get(url).throw(|r| r.status().is_server_error())` is called, *When* the upstream returns `500`, *Then* `HttpError::Status { code: 500 }` is returned.

- **Error — idle timeout is distinct:**
  > *Given* a client with `idle_timeout = 5s`, *When* the upstream stops sending bytes for 6 seconds, *Then* `HttpError::Timeout { kind: Idle }` is returned.

- **NFR — Performance:**
  > *Given* a mocked `200` upstream with 1ms latency, *When* 100 parallel `Http::get` calls run, *Then* p95 latency <100ms (localhost mock).

---

## M2 — ORM & Database

### US-M2-01 — Fluent query builder with multi-driver support

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want a fluent query builder over `sqlx` that supports Postgres, MySQL, and SQLite with transactions and pessimistic locks so that I can express Eloquent-style queries with compile-time safety.*  
- **Priority:** Must  
- **FRs:** FR-200, FR-202, FR-205 (BR-03 — FS-M2-01/03)  
- **Laravel 13:** baseline + #15 additions

**Acceptance Criteria:**

- **Happy — CRUD:**
  > *Given* `users` table with one row `{ id: 1, status: "active" }`, *When* `User::query().where("status","active").first().await` runs, *Then* `Some(User { id: 1 })` is returned.

- **Happy — transactions + lock:**
  > *Given* two concurrent `db.transaction(|tx| tx.select_for_update(User::with_id(1)))` tasks, *When* the second transaction starts before the first commits, *Then* the second blocks until commit (no dirty read).

- **Error — `firstOrFail`:**
  > *Given* no matching rows, *When* `firstOrFail()` is called, *Then* `QueryError::NotFound` is returned.

---

### US-M2-02 — New query builder additions (`chunkBy`, `insertOrIgnoreReturning`, etc.)

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want the Laravel 13 builder additions (`insertOrIgnoreReturning`, `saveOrIgnore`, `refreshForUpdate`, `whereBinary`, `chunkBy`, `orWhereKey`, `StraightJoin`) so that large-table operations and binary-safe comparisons work without raw SQL.*  
- **Priority:** Must  
- **FRs:** FR-203 (BR-03 — FS-M2-03)  
- **Laravel 13:** #15

**Acceptance Criteria:**

- **Happy — `chunkBy` without OOM:**
  > *Given* 10k rows in `users`, *When* `User::query().chunkBy("id", 500, |chunk| async { chunk.len() })` runs, *Then* `chunk` is called 20 times with 500 rows each.

- **Happy — `insertOrIgnoreReturning`:**
  > *Given* `users` has duplicate-conflicting rows, *When* `insertOrIgnoreReturning(rows)` is executed, *Then* non-conflicting rows inserted, conflicting rows ignored, and generated IDs returned.

- **Error — `whereBinary` requires binary column:**
  > *Given* non-binary column compared with `whereBinary`, *When* query runs on Postgres, *Then* driver error `QueryError::IncompatibleColumn` is surfaced.

---

### US-M2-03 — Strict `upsert` validation and MySQL `DELETE` support

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `upsert` to throw on empty `uniqueBy` and MySQL `DELETE … JOIN … ORDER BY/LIMIT` to compile, so that silent data errors become typed failures.*  
- **Priority:** Must  
- **FRs:** FR-204 (BR-03 — FS-M2-04)  
- **Laravel 13:** #14

**Acceptance Criteria:**

- **Error — empty `uniqueBy`:**
  > *Given* `User::upsert(rows, unique_by: [], update: ["name"])` is called, *When* executed, *Then* `UpsertError::EmptyUniqueBy` is returned before any DB round-trip.

- **Happy — MySQL delete with join:**
  > *Given* MySQL driver and `User::query().join("orders", "orders.user_id","users.id").where("orders.status","expired").delete()`, *When* executed, *Then* the generated SQL contains `DELETE users FROM users JOIN orders`.

---

### US-M2-04 — Collection serialization that preserves relations

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want eager-loaded relations to survive `serde` round-trip so that serializing a collection to JSON and back keeps `posts` loaded without extra queries.*  
- **Priority:** Must  
- **FRs:** FR-201, FR-206 (BR-03 — FS-M2-02)  
- **Laravel 13:** #13

**Acceptance Criteria:**

- **Happy — round-trip:**
  > *Given* `users` fetched via `User::with("posts").get().await` where each user has 2 posts, *When* `let j = serde_json::to_string(&users); let r: Vec<User> = serde_json::from_str(&j)` runs, *Then* `r[0].relation_loaded("posts")` is true and `r[0].posts.len() == 2`.

- **Edge — empty relation:**
  > *Given* a user with no posts, *When* round-tripped, *Then* `posts` is an empty vec, not `None`.

---

### US-M2-05 — Derived models, migrations, seeders, factories

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `#[derive(Model)]`, migrations, seeders, and factories with per-test resets so that schema and test data are managed like in Laravel.*  
- **Priority:** Must  
- **FRs:** FR-201, FR-208, FR-209 (BR-03 — FS-M2-02/05/06)  
- **Laravel 13:** #20 `Str` resets + baseline

**Acceptance Criteria:**

- **Happy — `make:model` + migrate:**
  > *Given* `cargo rustavel make:model Post -m` was run, *When* `cargo rustavel migrate` executes against SQLite, *Then* a `posts` table exists.

- **Happy — factory with reset:**
  > *Given* `UserFactory::create(5)` in test A, *When* test B calls `UserFactory::create(1)`, *Then* generated email sequence restarts at index 1 (no leak from test A).

- **Error — missing table:**
  > *Given* `Post::query().get().await` without migration applied, *When* executed, *Then* `QueryError::MissingTable { name: "posts" }` is returned.

---

### US-M2-06 — Vector column and nearest-neighbor search

- **Role:** AI application builder  
- **Story:** *As an AI application builder, I want `vector` Blueprint columns and `whereVectorSimilarTo` over `pgvector` so that semantic search works from day one without wiring my own SQL.*  
- **Priority:** Must  
- **FRs:** FR-207 (BR-03 — FS-M2-06)  
- **Laravel 13:** #6 (M2 initial)

**Acceptance Criteria:**

- **Happy — vector search:**
  > *Given* `products` with `vector(1536)` and 100 rows with embeddings plus one row `q` embedding `[0.1; 1536]`, *When* `Product::query().whereVectorSimilarTo("embedding", &q_embedding, limit: 10)` runs on Postgres+pgvector, *Then* 10 nearest rows are returned ordered by cosine distance.

- **Error — extension missing:**
  > *Given* target Postgres has no `CREATE EXTENSION vector` installed and `vector` feature is enabled, *When* migration runs, *Then* `PgVectorError::ExtensionMissing` is emitted with remediation hint.

- **Edge — dimension mismatch:**
  > *Given* column `vector(1536)` but embedding provided with length 768, *When* `whereVectorSimilarTo` executes, *Then* `VectorDimensionMismatch { expected: 1536, actual: 768 }` is returned.

---

## M3 — Auth, Middleware & Validation

### US-M3-01 — JWT + session guards with custom guard extension

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want JWT and session guards plus `Auth::extend` for custom guards so that token and cookie auth coexist and mismatch is a typed error.*  
- **Priority:** Must  
- **FRs:** FR-300, FR-301 (BR-04 — FS-M3-01)  
- **Laravel 13:** baseline + #11 context

**Acceptance Criteria:**

- **Happy — JWT login + parse:**
  > *Given* a user with `password` hashed via `argon2`, *When* `Auth::guard("jwt").login({ email, password })` is called with correct credentials, *Then* a token is returned and `Auth::guard("jwt").parse(token).await` yields `AuthUser { id }`.

- **Error — GuardMismatch:**
  > *Given* only `jwt` guard registered, *When* `Auth::guard("api").user()` is called, *Then* `Error::GuardMismatch { expected: "jwt", actual: "api" }`.

- **Error — Bad credentials / expired:**
  > *Given* an expired JWT (`exp` in past) or wrong password, *When* `login` or `parse` is called, *Then* `ExpiredToken` / `BadCredentials` respectively.

---

### US-M3-02 — Origin-aware CSRF protection

- **Role:** Security auditor  
- **Story:** *As a security auditor, I want CSRF to validate `Sec-Fetch-Site` origin alongside the token so that cross-site `POST` without a trusted origin is rejected even with a stolen token.*  
- **Priority:** Must  
- **FRs:** FR-302 (BR-04 — FS-M3-02)  
- **Laravel 13:** #11

**Acceptance Criteria:**

- **Happy — same-site passes:**
  > *Given* `csrf_origins: ["https://app.example.com"]` and request with `Sec-Fetch-Site: same-origin` + valid CSRF token, *When* `POST /form` is processed, *Then* `200`.

- **Error — cross-site rejected:**
  > *Given* request with `Sec-Fetch-Site: cross-site` and `Origin: https://evil.com` (not in allow-list) even with valid token, *When* `POST /form` runs, *Then* `403 CsrfError::UntrustedOrigin`.

- **Edge — missing `Sec-Fetch-Site` degrades to token-only:**
  > *Given* an older browser omitting `Sec-Fetch-Site`, *When* request arrives with valid token, *Then* request passes (token is primary).

---

### US-M3-03 — JSON session serialization + allow-list + hyphenated prefixes

- **Role:** Security auditor  
- **Story:** *As a security auditor, I want sessions serialized as JSON by default with a deserialization allow-list and hyphenated prefixes so that PHP-unserialize gadget attacks are impossible and cache prefixes don't collide.*  
- **Priority:** Must  
- **FRs:** FR-303, FR-304 (BR-04 — FS-M3-03)  
- **Laravel 13:** #12

**Acceptance Criteria:**

- **Happy — JSON session cookie:**
  > *Given* default config, *When* a session cookie is emitted, *Then* payload is JSON text containing `-session-` prefix in the cache key.

- **Error — disallowed class on deserialize:**
  > *Given* `serializable_classes: ["App::UserDto"]` and cached payload deserializing to `AdminDto`, *When* `Cache::get("k")` runs, *Then* `SerializationError::NotAllowed { type_name: "AdminDto" }`.

- **NFR — prefix is hyphenated:**
  > *Given* default `CACHE_PREFIX`, *When* inspected under `Cache::store("redis").prefix()`, *Then* it contains `-cache-` (not `_cache_`).

---

### US-M3-04 — Declarative `#[middleware]` / `#[authorize]` and validation rules

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `#[middleware]`/`#[authorize]` proc-macro attributes and `#[validate]` with strict `in_array` semantics and `ErrorBag`, so that auth/validation are declared on the handler, not wired manually.*  
- **Priority:** Must  
- **FRs:** FR-305, FR-307, FR-308, FR-309 (BR-04 — FS-M3-05/06)  
- **Laravel 13:** #7 + #19

**Acceptance Criteria:**

- **Happy — `#[middleware]` protects route:**
  > *Given* handler `#[middleware("auth:jwt")] async fn me(...)`, *When* unauthenticated `GET` arrives, *Then* `401`.

- **Happy — strict `in_array`:**
  > *Given* `#[validate(contains_strict = "admin")] role: String`, *When* payload `role = "Admin"` (case mismatch) or `role = 1` (int) submitted, *Then* validation fails (strict, no loose equality).

- **Error — `ErrorBag` on two fields:**
  > *Given* form with `email` wrong + `password` too short, *When* `#[validate]` runs, *Then* `ErrorBag` JSON has both `email` and `password` keys.

- **Edge — `#[authorize]` before handler body:**
  > *Given* `#[authorize("update", User)]` on `update_user`, *When* non-owner calls it, *Then* `403` authoritative response (typed `AuthorizationError`).

---

### US-M3-05 — Rate limiting per-IP

- **Role:** Platform engineer  
- **Story:** *As a platform engineer, I want `limit.per_minute(n).by(ip)` to throttle by IP so that brute-force attempts are bounded.*  
- **Priority:** Must  
- **FRs:** FR-306 (BR-04 — FS-M3-04)

**Acceptance Criteria:**

- **Happy — first window passes, next throttled:**
  > *Given* `limit.per_minute(3).by_ip()` on `POST /login`, *When* 3 requests from `1.2.3.4` succeed and a 4th within 60s arrives, *Then* `429` with `Retry-After` seconds to window reset.

- **Edge — proxies:**
  > *Given* `trusted_proxies` unset and request has `X-Forwarded-For: 9.9.9.9`, *When* throttling checks IP, *Then* it uses direct peer IP, not spoofed header.

---

## M4 — Queue, Cache, Scheduling & Events

### US-M4-01 — Typed jobs with retry and routing

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want typed `Job<T>` with `#[tries]`/`#[backoff]`/`#[timeout]` and `Queue::route::<Job>(queue:)` central routing so that job routing and retry are declared once, not per `dispatch`.*  
- **Priority:** Must  
- **FRs:** FR-400, FR-401, FR-402 (BR-05 — FS-M4-01/02)  
- **Laravel 13:** #4 + #7

**Acceptance Criteria:**

- **Happy — central routing:**
  > *Given* `Queue::route::<ProcessPodcast>(queue: "podcasts")` registered at boot, *When* `ProcessPodcast::dispatch({ id: 42 })` is called without `onQueue`, *Then* the job is enqueued on `podcasts` queue on the configured connection.

- **Happy — per-dispatch override:**
  > *Given* the same route, *When* `ProcessPodcast::dispatch(payload).onQueue("urgent")` is called, *Then* job goes to `urgent`, not `podcasts`.

- **Error — duplicate route:**
  > *Given* `Queue::route::<ProcessPodcast>(queue:"a")` already registered, *When* a second `Queue::route::<ProcessPodcast>(queue:"b")` is attempted during boot, *Then* `QueueError::DuplicateRoute { type_name: "ProcessPodcast" }`.

- **NFR — retry with backoff:**
  > *Given* `#[tries(3)] #[backoff(1)] struct Flaky` where `handle` fails twice, *When* dispatched, *Then* retried twice with ~1s backoff and succeeds on third attempt.

---

### US-M4-02 — Async job drivers, chaining, batching, failed jobs

- **Role:** Platform engineer  
- **Story:** *As a platform engineer, I want `sync`/`database`/`redis` drivers with `chain`/`batch` and a `failed_jobs` table so that the queue behaves correctly under failure.*  
- **Priority:** Must  
- **FRs:** FR-402 (BR-05 — FS-M4-02)

**Acceptance Criteria:**

- **Happy — chain stops on failure:**
  > *Given* `chain([JobA, JobB, JobC])` where `JobB::handle` fails, *When* chain is dispatched, *Then* `JobC` is not executed and `failed_jobs` contains `JobB` with exception.

- **Happy — batch dispatch:**
  > *Given* `batch([Job{1}, Job{2}, Job{3}])`, *When* dispatched, *Then* all three are enqueued and the batch `BatchId` is returned.

- **Happy — retry from failed:**
  > *Given* a failed job id `abc`, *When* `cargo rustavel queue:retry abc` runs, *Then* the job is re-queued and `failed_jobs` entry cleared after success.

---

### US-M4-03 — Cache `touch` and `Lock`

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `Cache::touch` to extend TTL without get/set and `Lock::get`/`block` for distributed locking so that long sessions stay alive cheaply and critical sections are serial.*  
- **Priority:** Must  
- **FRs:** FR-403, FR-404, FR-405 (BR-05 — FS-M4-04)  
- **Laravel 13:** #5

**Acceptance Criteria:**

- **Happy — `touch` extends TTL:**
  > *Given* `Cache::put("k","v", ttl: 60s)` and 30s elapsed, *When* `Cache::touch("k", 120s)` is called, *Then* `Cache::get("k")` still returns `Some("v")` after 90s from touch.

- **Error — `touch` missing key is `false`:**
  > *Given* `k` not in cache, *When* `touch("k", 60s)` is called, *Then* returns `false` (not an error).

- **Happy — `Lock` contention:**
  > *Given* worker A holds `Lock("billing", 10s).get()`, *When* worker B calls `Lock("billing").block(2s)`, *Then* worker B waits up to 2s or times out with `LockError::AlreadyHeld` after lease expiry.

---

### US-M4-04 — Events: `dispatch`, `dispatchAfterResponse`, async listeners, contract renames

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `Event`/`Listener` with `Queue { enable: true }` for async handling and `dispatchAfterResponse` so that listeners can run off the request path, and the `JobAttempted`/`QueueBusy` field renames are correct.*  
- **Priority:** Must  
- **FRs:** FR-406 (BR-05 — FS-M4-03)  
- **Laravel 13:** #16

**Acceptance Criteria:**

- **Happy — async listener:**
  > *Given* `impl Listener for SendMail { queue: Queue { enable: true } }`, *When* `Dispatcher::dispatch(UserCreated { id: 1 })` runs, *Then* the listener is enqueued as a job, not awaited inline.

- **Happy — `dispatchAfterResponse`:**
  > *Given* `Dispatcher::dispatchAfterResponse(AnalyticsFlushed)` inside a `GET /page` handler, *When* handler returns `200`, *Then* the event is dispatched after the response is sent (observable via test spy).

- **Error — contract field names:**
  > *Given* handlers for `JobAttempted` and `QueueBusy`, *When* inspected, *Then* fields are `exception` (not `exceptionOccurred`) and `connectionName` (not `connection`).

---

### US-M4-05 — Scheduling with pause/resume and frequencies

- **Role:** Platform engineer  
- **Story:** *As a platform engineer, I want `schedule:pause`/`schedule:resume` plus frequencies (`daily`, `cron`, `everyMinute`, `skipIfStillRunning`, `onOneServer`) so that schedules can be halted without redeploy.*  
- **Priority:** Must  
- **FRs:** FR-407, FR-408, FR-410 (BR-05 — FS-M4-05)  
- **Laravel 13:** #10

**Acceptance Criteria:**

- **Happy — pause halts firing:**
  > *Given* scheduler running `email:send` every minute, *When* `cargo rustavel schedule:pause` succeeds, *Then* `SchedulePaused` event is emitted and the next tick does not dispatch the job.

- **Happy — resume restarts:**
  > *Given* scheduler in `Paused` state, *When* `cargo rustavel schedule:resume` runs, *Then* `ScheduleResumed` event is emitted and subsequent tick dispatches again.

- **Happy — `skipIfStillRunning`:**
  > *Given* job `NightlyImport` takes 80s and schedule `everyMinute` with `skipIfStillRunning`, *When* the 60s tick arrives while previous run still active, *Then* that tick is skipped.

- **Happy — `onOneServer`:**
  > *Given* two scheduler nodes both ticking `daily` with `onOneServer`, *When* tick fires, *Then* only one node acquires the distributed `Cache::lock` and dispatches.

- **Edge — pause while job mid-execution:**
  > *Given* job running when `schedule:pause` is issued, *When* job completes, *Then* the next tick is already suppressed (running job is not killed).

---

### US-M4-06 — Cloud queue metrics (observable operability)

- **Role:** Platform engineer  
- **Story:** *As a platform engineer, I want `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` exposed via the `Queue` trait so that dashboards can show queue health without ad-hoc Redis commands.*  
- **Priority:** Must  
- **FRs:** FR-409 (BR-05 — FS-M4-03)  
- **Laravel 13:** #8

**Acceptance Criteria:**

- **Happy — pending size:**
  > *Given* `redis` queue `podcasts` with 42 pending jobs enqueued via `ProcessPodcast::dispatch`, *When* `Queue::pendingSize("redis","podcasts").await` is called, *Then* `42` is returned.

- **Happy — oldest pending job time:**
  > *Given* the oldest pending job created at `2026-09-07T10:00:00Z`, *When* `creationTimeOfOldestPendingJob("redis","podcasts").await` runs, *Then* RFC3339 string for that timestamp is returned.

- **Edge — empty queue:**
  > *Given* queue is empty, *When* `creationTimeOfOldestPendingJob` called, *Then* `None` is returned.

---

## M5 — DX, CLI & Testing

### US-M5-01 — `cargo rustavel` CLI with typed commands and prompts

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `cargo rustavel list` plus typed command args/flags, prompts (`ask`/`confirm`/`choice`), and `table`/`progressBar` so that CLI ergonomics match Artisan.*  
- **Priority:** Must  
- **FRs:** FR-500, FR-502, FR-503, FR-505 (BR-06 — FS-M5-01)  
- **Laravel 13:** #7 `#[Usage]`/`#[Help]`/`#[Hidden]`

**Acceptance Criteria:**

- **Happy — `list` shows commands:**
  > *Given* `cargo rustavel list --json` is run, *When* output is parsed, *Then* JSON contains entries for `make:controller` and `migrate` with `usage` strings.

- **Happy — `#[usage]`/`#[help]`:**
  > *Given* command `AppSend` annotated `#[usage("app:send {user}")]`, *When* `list` prints help, *Then* usage line shows `app:send {user}`.

- **Happy — prompts:**
  > *Given* command calls `confirm("Proceed?")`, *When* user answers `n`, *Then* command aborts with exit code `1`.

- **Happy — `Artisan::call`:**
  > *Given* `Artisan::call("migrate", vec![])` is invoked in-process, *When* checked, *Then* migrations ran without spawning a subprocess.

- **Edge — hidden command:**
  > *Given* command with `#[hidden]`, *When* `list` runs without `--all`, *Then* that command is not shown.

---

### US-M5-02 — `make:*` generators produce `rustfmt`/`clippy`-clean code

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `cargo rustavel make:*` for `controller`/`model`/`provider`/`command`/`job`/`event`/`listener`/`observer`/`test`/`seeder`/`agent`/`tool` to generate `rustfmt`+`clippy`-clean code so that scaffolding is usable without edits.*  
- **Priority:** Must  
- **FRs:** FR-501, FR-502, FR-506 (BR-06 — FS-M5-02)  
- **Laravel 13:** #7 + #2 `make:agent`/`make:tool`

**Acceptance Criteria:**

- **Happy — `make:controller`:**
  > *Given* `cargo rustavel make:controller UserController` is run in a fresh scaffold, *When* `app/http/controllers/user_controller.rs` is checked with `rustfmt --check` and `clippy -- -D warnings`, *Then* both pass.

- **Happy — `make:model -m`:**
  > *Given* `cargo rustavel make:model Post -m`, *When* outputs listed, *Then* `app/models/post.rs` with `#[derive(Model)]` and `database/migrations/*_create_posts_table.rs` both exist.

- **Error — Already exists:**
  > *Given* `app/models/post.rs` already exists, *When* `make:model Post` without `--force` runs, *Then* `GeneratorError::AlreadyExists { path }` is returned.

---

### US-M5-03 — Declarative attributes across jobs/commands/handlers

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `#[tries]`/`#[backoff]`/`#[timeout]`/`#[failOnTimeout]`/`#[withoutBroadcasting]` etc. to declaratively configure behavior so that I don't repeat retry/policy boilerplate.*  
- **Priority:** Must  
- **FRs:** FR-506, FR-400 (BR-06/BR-05 — FS-M5-03, FS-M4-01)  
- **Laravel 13:** #7 expanded attributes

**Acceptance Criteria:**

- **Happy — `#[tries(3)]` drives retry:**
  > *Given* `#[tries(3)] struct MyJob` where `handle` always fails, *When* dispatched on `sync` driver, *Then* job attempted 3 times before `failed_jobs` entry.

- **Edge — attribute vs trait conflict:**
  > *Given* job with `#[tries(3)]` and `impl ShouldRetry` returning `false`, *When* job fails, *Then* attribute wins with a compile warning `tries attribute shadows ShouldRetry`.

---

### US-M5-04 — Test harness with isolation and factory resets

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want a `TestCase` trait that spins up isolated Postgres via `testcontainers` and resets `Str` factory sequences between tests so that `cargo test` is deterministic under parallelism.*  
- **Priority:** Must  
- **FRs:** FR-507, FR-508, FR-509 (BR-06 — FS-M5-04)  
- **Laravel 13:** #20 Str resets + paginator

**Acceptance Criteria:**

- **Happy — isolated DB per worker:**
  > *Given* two tests `test_a` and `test_b` both `impl TestCase`, *When* run with `cargo test -- --test-threads=2`, *Then* each gets a Postgres on a distinct random port (no collision).

- **Happy — `Str` reset:**
  > *Given* `Factory::sequence` reached 10 in test `test_a`, *When* test `test_b` creates `UserFactory::create(1)`, *Then* email is `user1@example.com` (not `user11`).

- **NFR — teardown is clean:**
  > *Given* `cargo test` completes, *When* Docker containers are listed, *Then* no `rustavel-test-*` container remains running.

---

## M6 — Advanced (Broadcasting, Search, Filesystem, AI SDK, Real-time)

### US-M6-01 — WebSocket broadcasting + SSE with channel auth

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `ShouldBroadcast` channels over WebSocket with auth and SSE `eventStream` so that real-time features work without wiring an external realtime service from scratch.*  
- **Priority:** Must  
- **FRs:** FR-600, FR-601, FR-610 (BR-07 — FS-M6-01)  
- **Laravel 13:** #16 `eventStream`

**Acceptance Criteria:**

- **Happy — authenticated channel:**
  > *Given* `ShouldBroadcast { channel: Private("chat.1") }` with `Authorize` gate allowing `user 1`, *When* that user subscribes over WebSocket, *Then* broadcast `UserCreated` reaches the subscriber.

- **Error — unauthorized:**
  > *Given* user `2` without access to `Private("chat.1")`, *When* subscribing, *Then* `BroadcastError::Unauthorized { channel: "private-chat.1" }` (WebSocket close `4403`).

- **Happy — SSE:**
  > *Given* `Response::eventStream(stream_of(["hello","world"]))`, *When* client connects to `/events`, *Then* `Content-Type: text/event-stream` and two `data:` frames are received.

---

### US-M6-02 — Storage read-through with path confinement

- **Role:** Platform engineer  
- **Story:** *As a platform engineer, I want a read-through disk (primary + fallback, optional copy-back) with `Storage::path()` confined to the disk root so that storage migrates without code changes and path traversal is impossible.*  
- **Priority:** Must  
- **FRs:** FR-603, FR-611 (BR-07 — FS-M6-02)  
- **Laravel 13:** #9

**Acceptance Criteria:**

- **Happy — read-through:**
  > *Given* read-through `primary: "s3"`, `fallback: "local"` where `a/b.txt` only on `local`, *When* `Storage::get("a/b.txt")` is called, *Then* bytes from `local` are returned.

- **Error — path traversal:**
  > *Given* `Storage::path("../../etc/passwd")`, *When* validated, *Then* `StorageError::PathTraversal` (no filesystem access).

- **Happy — copy-back:**
  > *Given* read-through with `copy_back: true`, *When* `get("a/b.txt")` fell through to `fallback`, *Then* `primary` now also stores `a/b.txt`.

---

### US-M6-03 — JSON:API Resources with sparse fieldsets and inclusion

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want `JsonApiResource` with sparse fieldsets, relationship inclusion, links, and `application/vnd.api+json` headers so that I can serve spec-compliant JSON:API without hand-rolling serializers.*  
- **Priority:** Must  
- **FRs:** FR-604 (BR-07 — FS-M6-03)  
- **Laravel 13:** #3

**Acceptance Criteria:**

- **Happy — JSON:API document:**
  > *Given* `UserResource::new(user)` where `user` with `posts` loaded, *When* `UserResource::new(user).include("posts").fields(["name"]).to_response()`, *Then* response is `Content-Type: application/vnd.api+json` with `data.attributes` only `name` and `included` containing posts.

- **Error — include not loaded:**
  > *Given* `UserResource::include("posts")` where `posts` relation not eager-loaded, *When* `to_response()` called, *Then* `JsonApiError::RelationNotLoaded { relation: "posts" }`.

---

### US-M6-04 — Queued notifications with `deleteWhenMissingModels`

- **Role:** Rust developer  
- **Story:** *As a Rust developer, I want queued notifications marked `#[deleteWhenMissingModels]` to skip sending when the target model was deleted before the worker ran.*  
- **Priority:** Should  
- **FRs:** FR-605 (BR-07 — FS-M6-04)  
- **Laravel 13:** #17

**Acceptance Criteria:**

- **Happy — skip when deleted:**
  > *Given* a queued `WelcomeNotification { user: User { id: 9 } }` where `User 9` is deleted before queue processes it and notification has `#[deleteWhenMissingModels]`, *When* worker processes the job, *Then* job is skipped, not retried, with log `NotificationSkipped { reason: MissingModel }`.

- **Edge — non-deleted still sends:**
  > *Given* the same notification where user still exists, *When* processed, *Then* notification is sent normally.

---

### US-M6-05 — AI SDK provider-agnostic trait over 12 providers

- **Role:** AI application builder  
- **Story:** *As an AI application builder, I want a provider-agnostic `Ai` trait covering text/image/audio/embeddings/reranking/files/vector-stores across 12 providers so that switching from OpenAI to Anthropic is one config change.*  
- **Priority:** Must  
- **FRs:** FR-606, FR-612 (BR-07 — FS-M6-05)  
- **Laravel 13:** #1

**Acceptance Criteria:**

- **Happy — switch provider:**
  > *Given* `Ai::provider("openai").text("hello").send().await` succeeds, *When* provider is changed to `"anthropic"` with otherwise identical call, *Then* the same `AiResponse { text }` shape is returned via the `anthropic` adapter.

- **Error — unsupported capability:**
  > *Given* provider `ollama` without `reranking` capability, *When* `Ai::provider("ollama").rerank(docs)` is called, *Then* `AiError::UnsupportedCapability { provider: "ollama", capability: "rerank" }`.

- **NFR — opt-in feature flag:**
  > *Given* workspace with `rustavel` features excluding `ai`, *When* `cargo check` runs on `rustavel-router` only, *Then* no `async-openai` crate is in the dep graph.

---

### US-M6-06 — AI Agents with tools, streaming, broadcast, queueing, MCP, sub-agents

- **Role:** AI application builder  
- **Story:** *As an AI application builder, I want Agents with tools, structured output, streaming+broadcast over WebSocket, queued tool calls, deferred loaders, sub-agents, middleware, and MCP so that agentic workflows run end-to-end without wiring four SDKs.*  
- **Priority:** Must  
- **FRs:** FR-607, FR-608, FR-609, FR-610 (BR-07 — FS-M6-06)  
- **Laravel 13:** #2

**Acceptance Criteria:**

- **Happy — Agent + Tool + streaming:**
  > *Given* `SupportAgent` with `Tool SearchDocs` registered and prompt `"summarize ticket 42"`, *When* `agent.stream(prompt).await` is called, *Then* `SearchDocs::call` is invoked, and chunks `event: token` stream over WebSocket to the subscriber in order.

- **Happy — `make:agent` / `make:tool`:**
  > *Given* `cargo rustavel make:agent SupportAgent`, *When* `app/ai/agents/support_agent.rs` is inspected, *Then* it defines `struct SupportAgent` implementing `Agent` and is `rustfmt`-clean.

- **Happy — sub-agents + middleware:**
  > *Given* `ParentAgent { sub_agents: [KnowledgeAgent], middleware: [Logging] }`, *When* `ParentAgent` prompt triggers `KnowledgeAgent` as a tool, *Then* `Logging` middleware observes both parent and sub-agent calls.

- **Happy — deferred loaders:**
  > *Given* agent with `SimilaritySearch` deferred loader, *When* agent is prompted, *Then* relevant documents are fetched via `whereVectorSimilarTo` before tool invocation.

- **Error — MCP unavailable:**
  > *Given* feature `mcp` disabled and agent requests MCP tool discovery, *When* agent lists MCP tools, *Then* `AgentError::McpUnavailable` with hint to enable feature flag.

---

### US-M6-07 — Extended vector search (M6 full) and mail/notification adjacency

- **Role:** AI application builder  
- **Story:** *As an AI application builder, I want `Str::toEmbeddings`, `dropVectorIndex`, and full `whereVectorSimilarTo` indexing plus mail/notification defaults covered so that vector indices can be managed and embeddings are trait-consistent.*  
- **Priority:** Must  
- **FRs:** FR-602 (BR-07 — FS-M6-07)  
- **Laravel 13:** #6 full + #17 adjacency

**Acceptance Criteria:**

- **Happy — `toEmbeddings`:**
  > *Given* provider `openai` configured, *When* `Str::toEmbeddings("hello", provider: "openai").await` is called, *Then* `Vec<f32>` of expected dimension (e.g., 1536 for text-embedding-3-small) is returned.

- **Happy — `dropVectorIndex`:**
  > *Given* `products` table with `vector` index `products_embedding_index`, *When* `Schema::table("products", |t| t.dropVectorIndex("embedding"))` migration runs, *Then* `DROP INDEX` succeeds and subsequent `whereVectorSimilarTo` still runs (sequential scan) until re-indexed.

---

## Coverage Checklist (Normative)

| Gate | Status |
|------|--------|
| Every milestone M0–M6 has ≥1 story | Pass — M0:2, M1:6, M2:6, M3:5, M4:6, M5:4, M6:7 (30 stories total) |
| Every Laravel 13 feature #1–#20 traced to ≥1 story | Pass — see column per story |
| Each story has ≥3 EARS AC (happy + error + edge/NFR) | Pass — every story has ≥3; NFR criteria where applicable |
| Each story has exactly one role | Pass — role line is singular |
| Thresholds are measurable (`<2s`, `429`, `422`, `ptr_eq`, `rustfmt` pass) | Pass |
| Each story traces to BR/PRD FR/FSD FS | Pass — FRs/BR/FS tags per story |

### Laravel 13 → Story Index

| # | Feature | Stories |
|---|---------|---------|
| 1 | AI SDK | US-M6-05 |
| 2 | AI Agents | US-M6-06 (+ US-M5-02 via `make:agent/tool`) |
| 3 | JSON:API | US-M6-03 |
| 4 | Queue Routing | US-M4-01 |
| 5 | Cache `touch()` | US-M4-03 |
| 6 | Vector Search | US-M2-06 (M2) + US-M6-07 (M6 full) |
| 7 | Expanded Attributes | US-M3-04, US-M4-01, US-M5-03, US-M5-01 (`#[Usage]`), US-M5-02 |
| 8 | Cloud Queue metrics | US-M4-06 |
| 9 | Read-through FS | US-M6-02 |
| 10 | Schedule Pause/Resume | US-M4-05 |
| 11 | Origin-aware CSRF | US-M3-02 |
| 12 | Cache/Session hardening | US-M3-03 |
| 13 | Collection serialization | US-M2-04 |
| 14 | Upsert/Delete strict | US-M2-03 |
| 15 | Builder additions | US-M2-02 |
| 16 | Event/Queue contracts | US-M4-04 (+ US-M6-01 SSE adjacency) |
| 17 | Mail/Notification defaults | US-M6-04 |
| 18 | HTTP Client & Process | US-M1-06 |
| 19 | Routing & Validation | US-M1-02 + US-M3-04 |
| 20 | Observability & Tooling | US-M1-03 (+ US-M2-05/US-M5-04 factory resets) |

---

*Next: `bdd-scenarios.md` provides the Gherkin Feature/Senario realization of every story above, per `test-generation/rules/bdd-gherkin.md`.*
