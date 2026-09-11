# RustaSea — API Contracts by Milestone

> **Status:** Draft — P3 (TASK-008)  
> **Date:** 2026-09-07  
> **Source:** `brd.md` · `prd.md` (FR-000–FR-612) · `fsd.md` · `architecture.md` · `domain.md` · `bdd-scenarios.md`  
> **Conventions:** `AppState = Arc<AppState>` via `axum::extract::State`; JSON uses `serde_json`; errors are typed per-domain with `code`+`hint` (FSD §4.1).
> **Lineage (P3 → P6):** Authored as a P3 draft and adopted as the design parent by the P6 blueprint (`fsd.md` §8, `tdd.md` §8; `blueprint-audit.md` D4 PASS). The "P3" label records provenance, not unfinished status.
> **Planning vs as-built:** This document records planning intent, not implementation status. Live status: [`docs/milestones.md`](../../../docs/milestones.md) — the authoritative as-built status source (TASK-003).

---

## 1. M0 — Bootstrap & Core (No HTTP Endpoints)

**Contracts (trait-level, no wire API):**

| Contract | Kind | Signature | Notes |
|----------|------|-----------|-------|
| `Application::configure` | builder | `Application::configure() -> AppBuilder` | Collects providers; see `tdd.md §3 BC-0` |
| `ServiceProvider` | trait | `register(&mut Container)` + `boot(&AppState)` | DAG via `depends_on()`; `Runner` for HTTP/Queue/Schedule |
| `Container::Make` | query | `Make::<T> -> Result<Arc<T>>` / `Make::<Option<T>> -> Option<Arc<T>>` | `Singleton` is `ptr_eq` stable |
| `ConfigRegistry` | query | `AppState::config::<T: Deserialize>() -> &T` | Layer `defaults < file < .env < env`; `ConfigError::Parse {file,line}` |

**CLI:** `cargo rustasea new <app>` — produces `bootstrap/app.rs`, `config/*.toml`, `routes/web.rs`, `.env.example` (FR-005).

---

## 2. M1 — Routing & HTTP

### 2.1 Route Declarations (code contract)

```rust
// Functional
RouteRegistry::get("/users", handler::index).name("users.index").middleware(Throttle::per_minute(60).by_ip())
RouteRegistry::group("/api/v1", |g| { g.get("/users", ...); }).middleware(Cors::allow_origins(["https://app.example.com"]))
RouteRegistry::resource("users", UserController) // expands to 7 routes (FSD FS-M1-01)
RouteRegistry::get("/{user:slug}", ...).domain("{tenant}.example.com") // domain-aware

// Declarative
#[route(GET, "/users/{user:slug}")]
#[middleware("auth:jwt", "throttle:60,1")]
async fn show(State(state): State<AppState>, Path(params): Path<Params>, Json(body): Json<Body>) -> impl IntoResponse

// Response helpers
Json(user)               // Content-Type: application/json
View("users/index", ctx) // askama/minijinja via resources/views
Redirect::to("/login")
Response::eventStream(stream) // Content-Type: text/event-stream  (see also §7)
```

### 2.2 Middleware Stack

| Middleware | Config | Behavior |
|-----------|--------|----------|
| `Throttle::per_minute(n).by_ip()` | per-middleware | 429 + `Retry-After` when exceeded; respects `trusted_proxies` for `X-Forwarded-For` |
| `Cors::allow_origins([...])` | per-route/group | Only allow-listed `Origin` gets `Access-Control-Allow-Origin` |

### 2.3 HTTP Client

```rust
Http::get(url).header(k, v).timeout(Duration::from_secs(5))
    .throw(|resp| resp.status().is_server_error())
    .send().await -> Result<Response, HttpError>
// HttpError::Timeout { kind: Connect|Total|Idle } distinguished (FR-107/#18)
// throw callback that itself errors -> HttpError::ThrowCallback
```

### 2.4 Route Introspection

**`cargo rustasea route:list [--json]` (FR-103/#20):**

```json
[
  {
    "method": "GET",
    "path": "/users/{user:slug}",
    "name": "users.show",
    "middleware": ["throttle:60,1", "auth:jwt"],
    "binding_fields": ["slug"]
  }
]
```

Human table is default; `--json` is machine-readable with per-route `binding_fields` from `{param:field}` syntax.

---

## 3. M2 — ORM & Database (No Additional HTTP Endpoints)

See `tdd.md §3 BC-2` for `Model`/`QueryBuilder` trait signatures and `database.md` for DDL/migration/vector contracts. HTTP surface from M2 is via M1 route handlers returning `Json<T>` over `QueryBuilder` results; API contract is the builder itself (fluent methods, `paginate`/`cursor`/`chunkBy` envelopes).

**Pagination envelope:** `Paginated<T> { data: Vec<T>, meta: { current_page, per_page, total, last_page }, links: { first, prev, next, last } }` (consistent with `JsonApiResource` `meta`/`links` in §7).

**Vector query (M2 initial):** `Model::query().where_vector_similar_to("embedding", &query_vec, limit: 10).await -> Vec<T>` ordering by `embedding <=> $1`.

---

## 4. M3 — Auth, Middleware & Validation

### 4.1 Auth

```
POST /login            Body: { email, password } -> 200 { access_token, refresh_token, token_type: "Bearer", expires_in } | 401 BadCredentials
POST /loginUsingId     Body: { user_id: UUID }  -> 200 { access_token, ... }   (internal / testing)
POST /refresh          Header: Authorization: Bearer <token> -> 200 { access_token, refresh_token }
POST /logout           Header: Authorization: Bearer <token> -> 204
GET  /me               Header: Authorization: Bearer <token> -> 200 { id, email } | 401 InvalidToken/ExpiredToken
```

`Auth::guard("jwt")` (jsonwebtoken HS256 + argon2) vs `Auth::guard("session")` (tower-sessions). `Auth::extend("custom", |app| MyGuard)` at boot. Wrong guard name -> `AuthError::GuardMismatch { expected, actual }`.

**CSRF (origin-aware, FR-302/#11):**

| Header | Token | Result |
|--------|-------|--------|
| `Sec-Fetch-Site: cross-site` + invalid origin + valid token | valid | 403 `CsrfError::UntrustedOrigin` |
| `Sec-Fetch-Site: same-origin` + valid token | valid | 200 |
| Missing `Sec-Fetch-Site` + valid token | valid | 200 (degrades to token-only) |
| `GET`/`HEAD`/`OPTIONS` | — | exempt |

### 4.2 Validation

```rust
#[validate]
struct CreateUser {
    #[validate(length(min=3))] name: String,
    #[validate(email)] email: String,
    #[validate(contains_strict = "admin")] role: String, // strict: type+value, "1" != 1
}
async fn store(Json(input): Json<CreateUser>) -> impl IntoResponse // 422 on failure
```

**Error shape (422):**

```json
{
  "message": "The given data was invalid.",
  "errors": {
    "email": ["The email must be a valid email address."],
    "password": ["The password must be at least 8 characters."]
  }
}
```

`ErrorBag` keys by field; multiple errors per field coexist. `in_array`/`contains`/`doesnt_contain` are strict. Declarative `#[middleware("auth:jwt")]` and `#[authorize("update", User)]` on handlers -> 401/403 before body.

---

## 5. M4 — Queue, Cache, Scheduling & Events

### 5.1 Queue (typed `Job<T>`)

```rust
#[tries(3)] #[backoff(1)] #[timeout(30)]
struct ProcessPodcast { id: u64 }
impl Job for ProcessPodcast { async fn handle(self) -> Result<(), JobError> { ... } }

// Central routing (FR-401/#4)
Queue::route::<ProcessPodcast>(connection: "redis", queue: "podcasts")?; // boot-time; duplicate -> DuplicateRoute
ProcessPodcast { id: 42 }.dispatch().await?;                     // routed to podcasts
ProcessPodcast { id: 42 }.dispatch().on_queue("urgent").await?;  // override
Job::chain([JobA, JobB, JobC]).dispatch().await?;                // stops on first failure
Job::batch([Job{1}, Job{2}]).dispatch().await?;                  // -> BatchId
```

**Drivers:** `sync` (immediate) / `database` (polls `jobs` table) / `redis` (deadpool-redis BRPOP). `failed_jobs` table + `queue:failed` / `queue:retry {id}` CLI.

**CLI:**

```
cargo rustasea queue:work [--connection=redis] [--queue=podcasts] [--max-jobs=100]
cargo rustasea queue:failed
cargo rustasea queue:retry <id>
```

### 5.2 Cache

```rust
Cache::store("redis").put("k", "v", Duration::from_secs(60)).await?;
Cache::store("redis").get::<String>("k").await?;           // Option<String>
Cache::store("redis").touch("k", Duration::from_secs(120)).await?; // bool: false if missing
Cache::store("redis").remember("k", Duration::from_secs(60), || async { Ok(compute().await) }).await?;
Cache::lock("billing", Duration::from_secs(10)).get().await?;        // Option<LockGuard>
Cache::lock("billing", Duration::from_secs(10)).block(Duration::from_secs(5)).await?; // waits or AlreadyHeld
// Stores: memory (moka) + redis (deadpool-redis) behind Store+Repository; driver-agnostic API
```

Hyphenated prefixes `-cache-` / `-session-` and JSON serialization default (M3 hardening) apply. `touch` on missing key -> `false`, not error.

### 5.3 Events

```rust
impl Event for UserCreated {}
struct SendWelcomeEmail;
impl Listener<UserCreated> for SendWelcomeEmail {
    const QUEUE: bool = true; // -> enqueued as Job, not awaited inline
    async fn handle(&self, e: UserCreated) -> Result<(), EventError> { ... }
}
Dispatcher::dispatch(UserCreated { id }).await?;
Dispatcher::dispatch_after_response(AnalyticsFlushed).await?; // flushed after HTTP Response sent
// Events: JobAttempted { exception } (renamed), QueueBusy { connectionName } (renamed)
```

### 5.4 Schedule

```rust
Schedule::command("emails:send").daily().at("08:00").skip_if_still_running().on_one_server().register();
Schedule::command("report").cron("0 * * * *").register();
```

```
cargo rustasea schedule:list
cargo rustasea schedule:run        // tick every 60s; respects paused flag
cargo rustasea schedule:pause     // sets schedule_paused=true; emits SchedulePaused
cargo rustasea schedule:resume    // clears flag; emits ScheduleResumed
```

**Cloud queue metrics (FR-409/#8):**

```rust
Queue::pending_size("redis", "podcasts").await?;                         // usize
Queue::delayed_size("redis", "podcasts").await?;
Queue::reserved_size("redis", "podcasts").await?;
Queue::creation_time_of_oldest_pending_job("redis", "podcasts").await?;  // Option<DateTime<Utc>> RFC3339; None if empty
```

---

## 6. M5 — DX, CLI & Testing (No Wire API)

### CLI

```
cargo rustasea list [--json] [--all]        # enumerates commands with usage/help/hidden
cargo rustasea make:controller UserController [--resource]
cargo rustasea make:model Post -m           # + migration
cargo rustasea make:provider AppProvider
cargo rustasea make:command SendEmails
cargo rustasea make:job ProcessPodcast
cargo rustasea make:event UserCreated
cargo rustasea make:listener SendWelcomeEmail
cargo rustasea make:observer UserObserver
cargo rustasea make:test UserTest
cargo rustasea make:seeder UserSeeder
cargo rustasea make:agent SupportAgent      # M6
cargo rustasea make:tool SearchDocs         # M6
cargo rustasea migrate [--fresh] [--seed]
cargo rustasea route:list [--json]
cargo rustasea show:model User
cargo rustasea schedule:list|run|pause|resume
cargo rustasea queue:work|failed|retry
```

Attributes `#[usage("...")]` / `#[help("...")]` / `#[hidden]` on commands control `list` output. `Artisan::call("migrate", args)` is in-process invocation for tests.

### Testing

```rust
struct MyTest: TestCase { fn setup(&mut self) -> AppState { /* isolated PG on random port via testcontainers */ } }
// Per-package .env.testing overlay; Str factory sequences reset between tests; parallel isolation guaranteed
```

---

## 7. M6 — Advanced (Broadcasting, Search, Filesystem, AI SDK)

### 7.1 Broadcasting (WebSocket + SSE)

```rust
impl ShouldBroadcast for UserCreated {
    fn broadcast_on(&self) -> Channel { Channel::Private("chat.1".into()) }
}
// Client:
//   ws://host/broadcasting/auth  (auth gate Authorize)
//   Subscription to private-chat.1 without auth -> 403 Unauthorized / ws close 4403
// Server push: { event: "UserCreated", channel: "private-chat.1", data: {...} }

// SSE
Response::eventStream(stream_of(["hello","world"])) // Content-Type: text/event-stream, data: frames
```

### 7.2 Filesystem (read-through)

```rust
Storage::disk("s3").get("a/b.txt").await?;           // read-through primary->fallback; copy_back if configured
Storage::disk("local").put("a/b.txt", bytes).await?;
Storage::disk("s3").path("a/b.txt")?;                // canonicalized; "../../etc/passwd" -> StorageError::PathTraversal
Storage::disk("s3").exists("a/b.txt").await?;
// Config: { primary: "s3", fallback: "local", copy_back: true }
```

### 7.3 JSON:API Resources

```rust
UserResource::new(user).include("posts").fields(["name","email"]).to_response()
// Response: Content-Type: application/vnd.api+json
// { data: { type:"users", id, attributes:{name,email}, relationships:{posts:{data:[...]}}, links:{self:"..."} },
//   included:[{type:"posts", id, attributes:{...}}], links:{self:"..."}, meta:{...} }
// Sparse fieldsets: fields[users]=name,email ; include=posts ; include for not-eager-loaded relation -> JsonApiError::RelationNotLoaded
```

### 7.4 AI SDK (12 providers, feature-flagged)

```rust
// Provider-agnostic
Ai::provider("openai").text("hello").send().await?;       // -> AiResponse { text, usage, tool_calls }
Ai::provider("anthropic").text("hello").send().await?;    // same shape, different adapter
Ai::provider("ollama").rerank(docs).await?;               // -> AiError::UnsupportedCapability if provider lacks reranking

// Embeddings (M6 full)
Str::to_embeddings("hello", provider: "openai").await?;   // -> Vec<f32> (e.g., 1536)
Ai::provider("openai").embeddings("hello").await?;        // -> Vec<f32>

// Agents
#[derive(Agent)]
struct SupportAgent { tools: [SearchDocs], middleware: [Logging], sub_agents: [KnowledgeAgent] }
SupportAgent::prompt("summarize ticket 42").stream().await?; // -> Stream<Item=AiChunk> { event:"token", data:"..." }
Ai::agent(|a| a.tool(MyTool)).prompt("hi").stream().await?;  // anonymous agent
// make:agent / make:tool generators (rustfmt-clean)
// Deferred loaders: SimilaritySearch / FileStorage / ToolSearch inject before tool call
// MCP: feature = "mcp" -> MCP tool discovery; without feature -> AgentError::McpUnavailable
// Streaming -> broadcast over WebSocket: chunks arrive in order with event: token
```

Feature gating: workspace with only `rustasea-router` has no `async-openai` in `cargo tree`; `rustasea-ai` is `optional` with per-provider features `features=["openai","anthropic",...]`.

---

## 8. Error Catalogue (Wire-Level)

| HTTP Status | Typed Error | When |
|-------------|-------------|------|
| 200 | — | Success (`Json<T>`, `View`, `EventStream` chunk) |
| 204 | — | `logout` / `schedule:resume` when already running |
| 401 | `AuthError::InvalidToken` / `ExpiredToken` / `BadCredentials` / `GuardMismatch` | Auth guard mismatch or bad creds |
| 403 | `CsrfError::UntrustedOrigin` / `BroadcastError::Unauthorized` / `StorageError::PathTraversal` | CSRF cross-site, channel auth, path confinement |
| 404 | `QueryError::NotFound` / `StorageError::NotFound` | `firstOrFail` missing, storage missing |
| 422 | `ErrorBag` JSON | Validation (`#[validate]` strict failure) |
| 429 | `Throttle` | `per_minute` exceeded; `Retry-After` header |
| 500 | `JobError::MaxAttemptsExceeded` / `AgentError::ToolNotFound` | Job dead-letter, agent mis-config |
| 501 | `UnsupportedCapability` / `McpUnavailable` | Provider lacks capability, MCP flag off |

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
