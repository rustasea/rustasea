# RustaSea × Laravel 13.x — API-Surface Parity Map

> **Last updated:** 2026-09-11
> **Scope:** Maps the Laravel 13.x **API surface** (namespaces, contracts/interfaces, traits, notable classes) to RustaSea crates/modules, with an adoption status per row.
> **Companion docs:** [`docs/laravel-13-research.md`](laravel-13-research.md) (feature-level research) · [`docs/milestones.md`](milestones.md) (M0–M6 implementation status).

## 1. Purpose

`docs/laravel-13-research.md` answers *what Laravel 13 ships* at the feature level.
This document answers a different question: **which Laravel 13.x namespaces, interfaces, traits, and classes already have a RustaSea counterpart, and how complete is it?**

It is the API-surface parity layer. It is intentionally **not** a 1:1 inventory of every symbol on `api.laravel.com` (that index contains ~1000 classes). Per the agreed scope, it covers the **13 core surfaces** — Container, Contracts, Database/Eloquent, Routing, Http, Cache, Queue, Events, Auth, Validation, Support, Console, Filesystem — plus the advanced surfaces already present in the workspace (Broadcasting, Search, Storage, JSON:API, Testing, AI).

## 2. Methodology

1. Fetched the five authoritative Laravel 13.x API reference pages (see §3) and extracted the namespace list, the full interface list, the full trait list, and the notable-class list.
2. Enumerated RustaSea's real public API from the workspace source under `crates/` (traits, structs, enums, and re-exports), rather than from aspirational docs.
3. Mapped each Laravel surface to the closest RustaSea crate/module and assigned a status from §4.
4. Recorded gaps and an adoption order aligned to milestones **M0–M6** (as defined in [`docs/milestones.md`](milestones.md)).

**Name-integrity rule:** every Laravel name below is taken from a fetched reference page (or the raw `doc-index`); every RustaSea name is taken from the source tree. No symbol is invented. Because Laravel's class-level names for Routing/Database are only partially present in the truncated `classes.html` fetch, class-level rows use the fully-fetched `interfaces.html` / `traits.html` names and the raw `doc-index.html` cross-check.

## 3. Sources

| # | URL | Used for |
|---|---|---|
| 1 | https://api.laravel.com/docs/13.x/namespaces.html | Complete `Illuminate\*` namespace list |
| 2 | https://api.laravel.com/docs/13.x/interfaces.html | Complete interface/contract list |
| 3 | https://api.laravel.com/docs/13.x/traits.html | Complete trait list |
| 4 | https://api.laravel.com/docs/13.x/classes.html | Notable classes (fetch truncated; used for the first ~half of the A–Z class list) |
| 5 | https://api.laravel.com/docs/13.x/doc-index.html | Master A–Z symbol index (page is 8.7 MB; fetched raw and grepped for cross-verification, see §7) |

> Laravel's first-party **AI SDK** (`laravel/ai`) is a separate Composer package and is **not** part of the `api.laravel.com/docs/13.x` core namespace set; its Rust counterpart (`rustasea-ai`) is therefore covered as an advanced surface, not as an `Illuminate\*` row.

## 4. Status Legend

| Status | Meaning |
|---|---|
| **Adopted** | A working RustaSea equivalent exists and is reachable from a real code path. |
| **Partial** | An equivalent exists but is a stub, unwired, or covers only part of the Laravel contract. |
| **Planned** | No equivalent yet; listed on the gap roadmap (M0–M6). |
| **N-A** | Not applicable in Rust by design (e.g. global facades, PHP magic methods) or explicitly out of scope. |

## 5. Table A — Laravel Namespace → RustaSea Crate/Module

| Laravel namespace | RustaSea crate / module | Status | Notes |
|---|---|---|---|
| `Illuminate\Container` | `rustasea-foundation` (`Container`, `Application`) | **Adopted** | `bind` / `singleton` / `instance` / `get` are real (`crates/rustasea-foundation/src/lib.rs:63-104`). |
| `Illuminate\Contracts\Container` | `rustasea-foundation` (`Container`) | **Partial** | No separate contracts crate; contextual binding / `SelfBuilding` auto-wiring absent. |
| `Illuminate\Contracts` (general) | per-crate traits (no `rustasea-contracts` crate) | **Partial** | Contracts are expressed as Rust traits co-located with each crate (ADR-driven), not a mirrored namespace tree. |
| `Illuminate\Support` | `rustasea` facade re-exports; `rustasea-search::Str` | **Partial** | `Arrayable`/`Jsonable` map to `serde`; collections map to `Vec`/`Iterator`. |
| `Illuminate\Support\Facades` | — | **N-A** | ADR-005: no global facades; managers live in `AppState` and flow through `axum::extract::State`. |
| `Illuminate\Config` | `rustasea-config` (`ConfigLoader`) | **Partial** | TOML + env overlay real; only `config/app` is auto-loaded today. |
| `Illuminate\Console` | `rustasea-cli` (`Artisan`, `Command`, `CommandRegistry`) | **Adopted** | `cargo artisan` registry + generators are real. |
| `Illuminate\Console\Scheduling` | `rustasea-schedule` (`Schedule`, `Scheduler`, `ScheduleCommand`) | **Partial** | Pause/resume + ticks real; cron-cache mutexes / background tasks absent. |
| `Illuminate\Database` | `rustasea-orm` (`Database`, `DbPool`, `DatabaseServiceProvider`) | **Partial** | sqlx pool real; query execution / migrations still stubs (P2). |
| `Illuminate\Database\Eloquent` | `rustasea-orm` (`Model`, `QueryBuilder`, `Relation`, `SoftDeletes`, `Timestamps`) | **Partial** | `#[derive(Model)]` real; eager-loading hydration pending. |
| `Illuminate\Database\Migrations` | `rustasea-orm` (`Migration`, `Migrator`, `MigrationRecord`) | **Partial** | Traits + records exist; runner not yet executed against a live pool. |
| `Illuminate\Database\Query` | `rustasea-orm` (`QueryBuilder`, `Value`, `JsonFilter`) | **Partial** | Fluent builder real; execution path stub. |
| `Illuminate\Events` | `rustasea-events` (`Dispatcher`, `Event`, `Listener`) | **Partial** | Inline dispatch real; queue-backed listeners unwired. |
| `Illuminate\Routing` | `rustasea-router` (`Router`, `RouteEntry`, `ControllerRef`) | **Partial** | DSL + controller dispatch real; `route:list` introspection empty. |
| `Illuminate\Http` | `rustasea-http` (`AppState`, `JsonResponse`, `HttpError`) | **Partial** | Request/response + CORS real; idle timeout declared, not enforced. |
| `Illuminate\Http\Client` | `rustasea-http` (`HttpClient`) | **Adopted** | reqwest wrapper with `throw` / `try_throw` semantics. |
| `Illuminate\Http\Resources\JsonApi` | `rustasea-jsonapi` (`JsonApiResource`, `Document`, `ResourceBuilder`) | **Adopted** | Sparse fieldsets, links, JSON:API content type real. |
| `Illuminate\Cache` | `rustasea-cache` (`Store`, `CacheManager`, `Repository`, `Lock`) | **Partial** | Memory store real; Redis store returns `StoreUnavailable`. |
| `Illuminate\Queue` | `rustasea-queue` (`Queue`, `QueueDriver`, `Job`, `QueueRegistry`) | **Partial** | Sync driver real; database/redis drivers are name constants. |
| `Illuminate\Bus` | `rustasea-queue` (`BatchHandle`, `BatchId`) | **Partial** | Batch handles exist; no durable batch repository. |
| `Illuminate\Auth` | `rustasea-auth` (`AuthManager`, `Guard`, `JwtGuard`, `SessionGuard`) | **Partial** | JWT/CSRF/throttle real; session guard placeholder. |
| `Illuminate\Auth\Access` | `rustasea-macros` (`#[authorize]` metadata) | **Planned** | No runtime Gate/Policy evaluation (GAP-003). |
| `Illuminate\Validation` | `rustasea-validation` (`Validatable`, `Rules`, `ErrorBag`, `FormRequest`) | **Partial** | Rules + ErrorBag + form requests real; DB presence verifier absent. |
| `Illuminate\Hashing` | `rustasea-auth` (`PasswordVerifier`, `Argon2Verifier`) | **Partial** | Argon2 verifier real; no hasher manager / rehash policy. |
| `Illuminate\Session` | `rustasea-auth` (`SessionGuard`, `SessionPolicy`) | **Partial** | `tower-sessions` declared but session store not wired (GAP-007). |
| `Illuminate\Cookie` | `rustasea-http` (CORS + `SecurityConfig`) | **Partial** | No queued-cookie jar / cookie encryption layer yet. |
| `Illuminate\Filesystem` | `rustasea-storage` (`Storage`, `StorageManager`, `LocalDisk`, `ObjectDisk`, `ReadThrough`) | **Adopted** | `object_store`-backed disks + read-through with path confinement. |
| `Illuminate\Broadcasting` | `rustasea-broadcast` (`ShouldBroadcast`, `BroadcastEvent`, `Channel`, `BroadcastHub`) | **Partial** | WS + SSE real; driver matrix (Pusher/Ably/Redis) absent. |
| `Illuminate\Pagination` | `rustasea-orm` (`Paginator`, `PageMeta`) | **Partial** | Paginator type real; not wired into query execution. |
| `Illuminate\Pipeline` | — | **N-A** | Tower middleware chains replace the PHP pipeline; no `Illuminate\Pipeline` analogue required. |
| `Illuminate\Encryption` | — | **Planned** | No encrypter / key-rotation service. |
| `Illuminate\Translation` | — | **Planned** | No translator / locale loader. |
| `Illuminate\View` | `resources/views` | **Planned** | Template engine integration (`askama` / `minijinja`) not declared. |
| `Illuminate\Mail` | — | **Planned** | No mailable / mailer transport. |
| `Illuminate\Notifications` | `rustasea-queue` (`NotificationGuard`) | **Partial** | Missing-model skip guard only; no channel dispatcher. |
| `Illuminate\Log` | `tracing` (workspace dependency) | **N-A** | Structured logging is handled by `tracing`, not a Laravel-style `Log` facade. |
| `Illuminate\Redis` | `rustasea-cache` (`RedisStore`) | **Partial** | Type exists; operations return `StoreUnavailable`. |
| `Illuminate\Process` | — | **N-A** | Process spawning handled by `tokio::process` directly. |
| `Illuminate\Concurrency` | `tokio` (workspace dependency) | **N-A** | Concurrency is native `tokio`; no `Concurrency` facade. |
| `Illuminate\Foundation` | `rustasea-foundation` + `bootstrap/` | **Partial** | App boot + graceful shutdown real; provider/command registries empty. |
| `Illuminate\Testing` | `rustasea-testing` (`TestCase`, `TestConfig`) | **Partial** | `TestCase` real; `testcontainers` unused; no DB refresh traits. |
| `Illuminate\Image` | — | **Planned** | No image driver/transformation surface. |
| `Illuminate\JsonSchema` | — | **Planned** | No JSON-schema contract. |
| AI SDK (`laravel/ai`, separate package) | `rustasea-ai` (`AiProvider`, `Agent`, `Tool`) | **Partial** | Provider-agnostic traits + agents real; adapters are deterministic stubs (feature `ai`). |

## 6. Table B — Key Interface / Trait → RustaSea Equivalent

> Rows are grouped by the 13 focus surfaces. "Rationale" states why the status is what it is.

### Container & Contracts

| Laravel interface/trait | RustaSea equivalent | Status | Rationale |
|---|---|---|---|
| `Illuminate\Contracts\Container\Container` | `rustasea-foundation::Container` | **Partial** | Resolve/bind/singleton/instance present; contextual bindings and auto-construction (`SelfBuilding`) missing. |
| `Illuminate\Contracts\Container\ContextualBindingBuilder` | — | **Planned** | No per-consumer binding surface. |
| `Illuminate\Contracts\Container\SelfBuilding` | — | **Planned** | No reflection-based auto-wiring. |
| `Illuminate\Contracts\Support\Arrayable` | `serde::Serialize` | **N-A** | Serialization is idiomatic `serde`, not a bespoke interface. |
| `Illuminate\Contracts\Support\Jsonable` | `serde_json` | **N-A** | JSON encoding is handled by `serde_json`. |
| `Illuminate\Contracts\Support\DeferrableProvider` | `rustasea-foundation::ServiceProvider` | **Partial** | `register`/`boot` lifecycle real; no deferred-provider optimization. |
| `Illuminate\Contracts\Support\MessageBag` / `MessageProvider` | `rustasea-validation::ErrorBag` | **Adopted** | Field-keyed aggregation implemented. |
| `Illuminate\Contracts\Support\ValidatedData` | `rustasea-validation::FormRequest` | **Partial** | `validated()` payload exists; contract breadth smaller. |
| `Illuminate\Contracts\Support\Responsable` | `rustasea-http::JsonResponse` | **Partial** | JSON/status helpers real; not a generic response contract. |
| `Illuminate\Contracts\Debug\ExceptionHandler` | — | **Planned** | Error handling delegated to axum/tower; no handler contract. |
| `Illuminate\Container\Attributes\Singleton` | `rustasea-foundation::Container::singleton` | **Partial** | Programmatic singleton binding; no attribute-driven binding. |

### Database / Eloquent

| Laravel interface/trait | RustaSea equivalent | Status | Rationale |
|---|---|---|---|
| `Illuminate\Contracts\Database\Eloquent\Builder` | `rustasea-orm::QueryBuilder` | **Partial** | Fluent builder real; no Eloquent-level model hydration. |
| `Illuminate\Contracts\Database\Query\Builder` | `rustasea-orm::QueryBuilder` | **Partial** | Clause compilation real; execution stub (`execution.rs`). |
| `Illuminate\Contracts\Database\Eloquent\CastsAttributes` | — | **Planned** | Attribute casting not implemented (serde only). |
| `Illuminate\Contracts\Database\Eloquent\Castable` | — | **Planned** | No castable type contract. |
| `Illuminate\Contracts\Database\Eloquent\SupportsPartialRelations` | `rustasea-orm::Relation` | **Partial** | Relation declarations exist; partial-relation loading pending. |
| `Illuminate\Database\Eloquent\Scope` | `rustasea-orm::ScopeRegistry` | **Partial** | Global-scope registry real; not applied during execution. |
| `Illuminate\Database\Eloquent\SoftDeletes` (trait) | `rustasea-orm::SoftDeletes` | **Partial** | Marker/inference via `#[derive(Model)]`; query filtering not enforced. |
| `Illuminate\Database\Eloquent\Concerns\HasTimestamps` | `rustasea-orm::Timestamps` | **Partial** | Timestamp inference real; write-path population pending. |
| `Illuminate\Database\Eloquent\Concerns\HasRelationships` | `rustasea-orm::{Relation, RelationKind}` | **Partial** | Relation declarations exist; eager-loading loader absent. |
| `Illuminate\Database\Eloquent\Concerns\HasUuids` | `#[derive(Model)]` + `uuid::Uuid` `id` | **Partial** | UUID `id` enforced at derive time; no ULID variant. |
| `Illuminate\Database\Eloquent\Factories\HasFactory` | `rustasea-orm::Factory` / `SqlSeeder` | **Partial** | Factory + seeder traits exist; not executed against a pool. |
| `Illuminate\Database\ConnectionInterface` | `rustasea-orm::{Database, DbPool}` | **Partial** | Pool connect/ping real; CRUD round-trip pending (GAP-011). |
| `Illuminate\Database\ConnectionResolverInterface` | `rustasea-orm::Database` | **Partial** | Single-manager resolution; multi-connection resolver thinner. |
| `Illuminate\Database\Migrations\MigrationRepositoryInterface` | `rustasea-orm::{Migrator, MigrationRecord}` | **Partial** | Repository records modelled; migration runner stubbed. |
| `Illuminate\Database\Eloquent\Relations\Concerns\InteractsWithPivotTable` | `rustasea-orm::Relation` | **Planned** | Pivot-table interaction not implemented. |

### Routing / Http

| Laravel interface/trait | RustaSea equivalent | Status | Rationale |
|---|---|---|---|
| `Illuminate\Contracts\Routing\Registrar` | `rustasea-router::Router` | **Partial** | Registration DSL real; full registrar contract (bindings, fallbacks) thinner. |
| `Illuminate\Contracts\Routing\ResponseFactory` | `rustasea-http::JsonResponse` | **Partial** | JSON/status helpers only. |
| `Illuminate\Contracts\Routing\UrlGenerator` | — | **Planned** | No named-route URL generator. |
| `Illuminate\Contracts\Routing\UrlRoutable` | — | **Planned** | No implicit model route binding. |
| `Illuminate\Routing\Contracts\ControllerDispatcher` | `rustasea-router::dispatch` | **Adopted** | Real controller dispatch landed in GAP-002. |
| `Illuminate\Routing\Contracts\CallableDispatcher` | `rustasea-router::Handler` | **Partial** | Handler trait real; closure/callable dispatch breadth narrower. |
| `Illuminate\Routing\Controllers\HasMiddleware` | `rustasea-macros::#[middleware]` | **Partial** | Attribute metadata emitted; runtime consumer pending (GAP-003). |
| `Illuminate\Contracts\Http\Kernel` | `rustasea-http::AppState` | **Partial** | Middleware stack assembled via axum/tower; no single Kernel contract. |
| `Illuminate\Http\Client\Factory` | `rustasea-http::HttpClient` | **Partial** | Client real; no fake/record-replay factory. |
| `Illuminate\Http\Resources\JsonApi\Concerns\ResolvesJsonApiElements` | `rustasea-jsonapi::ResourceBuilder` | **Adopted** | Element resolution + document shaping implemented. |

### Cache / Queue / Events

| Laravel interface/trait | RustaSea equivalent | Status | Rationale |
|---|---|---|---|
| `Illuminate\Contracts\Cache\Store` | `rustasea-cache::Store` | **Adopted** | `get`/`put`/`forget` store contract implemented by memory store. |
| `Illuminate\Contracts\Cache\Repository` | `rustasea-cache::{Repository, RepositoryLike}` | **Adopted** | Repository wrapper + trait implemented. |
| `Illuminate\Contracts\Cache\Lock` | `rustasea-cache::{Lock, LockGuard}` | **Partial** | In-memory lock/guard real; distributed locks absent. |
| `Illuminate\Contracts\Cache\Factory` | `rustasea-cache::CacheManager` | **Partial** | Manager selects stores; driver matrix incomplete. |
| `Illuminate\Contracts\Queue\Queue` | `rustasea-queue::Queue` | **Partial** | Push/dispatch surface real; only sync driver wired. |
| `Illuminate\Contracts\Queue\Job` | `rustasea-queue::{Job, ErasedJob}` | **Partial** | Job + erased-job traits real; worker loop absent. |
| `Illuminate\Contracts\Queue\ShouldQueue` | `rustasea-queue::Job` | **Partial** | Implemented via trait; no queue-backed listener wiring. |
| `Illuminate\Contracts\Queue\ShouldBeUnique` | — | **Planned** | No unique-job locking. |
| `Illuminate\Contracts\Queue\Factory` | `rustasea-queue::QueueRegistry` | **Partial** | Registry + routing real; connection factory thinner. |
| `Illuminate\Queue\Connectors\ConnectorInterface` | `rustasea-queue::QueueDriver` | **Partial** | Driver trait real; redis/database drivers are constants. |
| `Illuminate\Contracts\Events\Dispatcher` | `rustasea-events::Dispatcher` | **Partial** | Inline dispatch + `dispatchAfterResponse` real; queue path unwired. |
| `Illuminate\Events\Dispatcher` (class) | `rustasea-events::Dispatcher` | **Partial** | Concrete dispatcher present with the same caveat. |

### Auth / Validation / Session

| Laravel interface/trait | RustaSea equivalent | Status | Rationale |
|---|---|---|---|
| `Illuminate\Contracts\Auth\Guard` | `rustasea-auth::Guard` | **Adopted** | Guard trait + JWT implementation real. |
| `Illuminate\Contracts\Auth\StatefulGuard` | `rustasea-auth::SessionGuard` | **Partial** | Session guard is a placeholder (GAP-007). |
| `Illuminate\Contracts\Auth\Authenticatable` | `rustasea-auth::AuthUser` | **Adopted** | User identity type implemented. |
| `Illuminate\Contracts\Auth\UserProvider` | `rustasea-auth::UserLookup` | **Partial** | Lookup trait real; DB/Eloquent providers thinner. |
| `Illuminate\Contracts\Auth\Factory` | `rustasea-auth::AuthManager` | **Adopted** | Named guard registration + `Auth::extend` real. |
| `Illuminate\Contracts\Auth\Access\Gate` | — | **Planned** | No policy/gate evaluator (GAP-003). |
| `Illuminate\Contracts\Auth\Access\Authorizable` | `rustasea-macros::#[authorize]` | **Planned** | Attribute metadata only; no runtime enforcement. |
| `Illuminate\Contracts\Auth\CanResetPassword` | — | **Planned** | No password-reset broker. |
| `Illuminate\Contracts\Auth\MustVerifyEmail` | `rustasea-auth::EmailVerification` | **Partial** | Verification trait + memory impl real; mail transport absent. |
| `Illuminate\Contracts\Auth\PasswordBroker` | — | **Planned** | No token broker. |
| `Illuminate\Auth\GuardHelpers` (trait) | `rustasea-auth::Guard` default methods | **Partial** | Shared guard defaults; surface smaller. |
| `Illuminate\Contracts\Validation\Validator` | `rustasea-validation::Rules` | **Partial** | Rule engine real; contract breadth smaller. |
| `Illuminate\Contracts\Validation\ValidatesWhenResolved` | `rustasea-validation::FormRequest` | **Adopted** | Form-request validation-on-resolve implemented. |
| `Illuminate\Contracts\Validation\Rule` | `rustasea-validation::Rules` | **Partial** | Rule registration real; not a per-rule trait object. |
| `Illuminate\Contracts\Validation\DataAwareRule` / `ValidatorAwareRule` | `rustasea-validation::Rules` | **Partial** | Data-aware validation exists; aware-rule contracts folded into `Rules`. |
| `Illuminate\Validation\Concerns\ValidatesAttributes` (trait) | `rustasea-validation::rules` | **Partial** | Core rules implemented; full Laravel rule catalogue not ported. |
| `Illuminate\Contracts\Session\Session` | `rustasea-auth::SessionPolicy` | **Partial** | Policy/hardening real; store-backed session pending. |

### Support / Console / Filesystem / Advanced

| Laravel interface/trait | RustaSea equivalent | Status | Rationale |
|---|---|---|---|
| `Illuminate\Support\Enumerable` | `std::iter` / `Vec` | **N-A** | Iteration is idiomatic Rust; no `Enumerable` interface. |
| `Illuminate\Support\Traits\Macroable` | — | **N-A** | Rust has no runtime method injection; proc-macros cover the use case. |
| `Illuminate\Support\Traits\Conditionable` | — | **N-A** | Conditional chaining is expressed with combinators/`if`. |
| `Illuminate\Support\Traits\Tappable` | — | **N-A** | `inspect`/`tap` helpers are trivial in Rust and not a shared trait. |
| `Illuminate\Console\Command` | `rustasea-cli::Command` | **Adopted** | Command trait + registry real. |
| `Illuminate\Contracts\Console\Kernel` | `rustasea-cli::Artisan` | **Partial** | Console entrypoint real; full kernel contract thinner. |
| `Illuminate\Contracts\Console\PromptsForMissingInput` | `rustasea-cli::prompt` | **Partial** | Prompting helpers real; contract not formalized. |
| `Illuminate\Console\Attributes\Usage` / `Help` / `Hidden` | `rustasea-macros::#[usage]` / `#[help]` / `#[hidden]` | **Adopted** | Attribute surface present and consumed by `artisan list`. |
| `Illuminate\Console\Scheduling\ManagesFrequencies` (trait) | `rustasea-schedule::ScheduleBuilder` | **Partial** | Frequency builder real; full cron grammar thinner. |
| `Illuminate\Contracts\Filesystem\Filesystem` | `rustasea-storage::Storage` | **Adopted** | `get`/`put`/`delete` storage trait implemented. |
| `Illuminate\Contracts\Filesystem\Factory` | `rustasea-storage::StorageManager` | **Partial** | Manager + disks real; driver matrix smaller. |
| `Illuminate\Contracts\Filesystem\Cloud` | `rustasea-storage::ObjectDisk` | **Partial** | `object_store`-backed disk real; visibility/temporary-URL surface thinner. |
| `Illuminate\Filesystem\FilesystemAdapter` | `rustasea-storage::{LocalDisk, ObjectDisk}` | **Partial** | Disk adapters real; adapter method breadth smaller. |
| `Illuminate\Contracts\Broadcasting\ShouldBroadcast` | `rustasea-broadcast::ShouldBroadcast` | **Adopted** | Broadcast marker implemented. |
| `Illuminate\Contracts\Broadcasting\Broadcaster` | `rustasea-broadcast::BroadcastHub` | **Partial** | WS/SSE hub real; third-party broadcasters absent. |
| `Illuminate\Contracts\Broadcasting\Factory` | `rustasea-broadcast::BroadcastHub` | **Partial** | Hub selection real; connection factory thinner. |
| `Illuminate\Foundation\Testing\TestCase` (class) | `rustasea-testing::TestCase` | **Partial** | Base test case real; refresh/DB traits absent. |
| `Illuminate\Foundation\Testing\RefreshDatabase` (trait) | — | **Planned** | No DB refresh/transaction test trait. |
| `Illuminate\Foundation\Testing\WithFaker` (trait) | `rustasea-testing::StrFactory` | **Partial** | Deterministic string factory real; faker surface smaller. |

## 7. Verification

**Spot-check (≥10 names) against the fetched references and the raw `doc-index.html`:**

| # | Name | Source page | Result |
|---|---|---|---|
| 1 | `Illuminate\Container\Container` | namespaces + classes + doc-index | present |
| 2 | `Illuminate\Contracts\Container\Container` | interfaces | present |
| 3 | `Illuminate\Contracts\Support\MessageBag` | interfaces | present |
| 4 | `Illuminate\Contracts\Database\Eloquent\Builder` | interfaces | present |
| 5 | `Illuminate\Contracts\Queue\Queue` | interfaces | present |
| 6 | `Illuminate\Contracts\Auth\Guard` | interfaces | present |
| 7 | `Illuminate\Contracts\Validation\ValidatesWhenResolved` | interfaces | present |
| 8 | `Illuminate\Contracts\Routing\Registrar` | interfaces | present |
| 9 | `Illuminate\Contracts\Filesystem\Filesystem` | interfaces | present |
| 10 | `Illuminate\Database\Eloquent\SoftDeletes` | traits | present |
| 11 | `Illuminate\Support\Traits\Macroable` | traits | present |
| 12 | `Illuminate\Console\Scheduling\ManagesFrequencies` | traits | present |
| 13 | `Illuminate\Routing\Contracts\ControllerDispatcher` | interfaces | present |
| 14 | `Illuminate\Console\Attributes\Usage` | classes | present |
| 15 | `Illuminate\Contracts\Broadcasting\ShouldBroadcast` | interfaces | present |

**RustaSea spot-check (source tree):** `rustasea-foundation::Container` (`crates/rustasea-foundation/src/lib.rs:35`), `rustasea-orm::Model` (`crates/rustasea-orm/src/model.rs:107`), `rustasea-cache::Store` (`crates/rustasea-cache/src/store.rs:17`), `rustasea-queue::QueueDriver` (`crates/rustasea-queue/src/driver.rs:29`), `rustasea-events::Dispatcher` (`crates/rustasea-events/src/dispatcher.rs:90`), `rustasea-auth::Guard` (`crates/rustasea-auth/src/guard.rs:120`), `rustasea-validation::FormRequest` (`crates/rustasea-validation/src/form_request.rs`), `rustasea-router::Router` (`crates/rustasea-router/src/router.rs:15`), `rustasea-http::HttpClient` (`crates/rustasea-http/src/lib.rs:237`), `rustasea-storage::Storage` (`crates/rustasea-storage/src/storage.rs:13`), `rustasea-jsonapi::JsonApiResource` (`crates/rustasea-jsonapi/src/resource.rs:83`), `rustasea-broadcast::ShouldBroadcast` (`crates/rustasea-broadcast/src/lib.rs:36`).

`doc-index.html` note: the live page is 8.7 MB and exceeded the fetch tool's 5 MB response cap, so it was retrieved as a raw document and grepped for each mapped name (all returned non-zero matches, except `Illuminate\Hashing\Hasher`, which is not a class in this release — the contract is `Illuminate\Contracts\Hashing\Hasher`, used above).

## 8. Gaps & Recommended Adoption Order (M0–M6)

The dominant pattern: **RustaSea already has the shape of most Laravel surfaces (a trait or type), but the execution path behind them is frequently a stub.** Parity work is therefore mostly *finishing* existing surfaces rather than inventing new ones.

| Order | Milestone | Gap to close | Target Laravel surface |
|---|---|---|---|
| 1 | **M0** Bootstrap & Core | Contextual bindings + provider DAG + auto-construction; populate provider/command registries | `Contracts\Container\ContextualBindingBuilder`, `SelfBuilding`, `Contracts\Foundation\Application` |
| 2 | **M1** Routing & HTTP | Wire `route:list` to the live router; enforce idle timeout; add URL generation + implicit binding | `Contracts\Routing\Registrar`, `UrlGenerator`, `UrlRoutable`, `Contracts\Http\Kernel` |
| 3 | **M2** ORM & Database | Execute builder through `DbPool`; real migrations/seeders/factories; casts; eager loading | `Contracts\Database\Eloquent\Builder`, `CastsAttributes`, `ConnectionInterface`, `MigrationRepositoryInterface` |
| 4 | **M3** Auth, Middleware & Validation | Runtime `#[authorize]`/Gate; store-backed session guard; password broker/reset | `Contracts\Auth\Access\Gate`, `Authorizable`, `StatefulGuard`, `PasswordBroker` |
| 5 | **M4** Queue, Cache, Scheduling & Events | Redis/database drivers; worker loop; persistent failed jobs; queue-backed listeners; distributed locks | `Contracts\Queue\Factory`, `ShouldBeUnique`, `Contracts\Cache\Lock`, `Contracts\Events\Dispatcher` |
| 6 | **M5** DX, CLI & Testing | `make:middleware`/`make:request`, `artisan new`, real cycle detection; DB refresh test traits | `Contracts\Console\Kernel`, `Foundation\Testing\RefreshDatabase`, `WithFaker` |
| 7 | **M6** Advanced | Real AI adapters; pgvector index; template engine; encryption/translation/mail surfaces | `rustasea-ai` adapters; `Illuminate\Encryption`, `Translation`, `View`, `Mail` analogues |

> The ordering above mirrors the P0–P5 gap program in [`docs/milestones.md`](milestones.md): foundation unblockers first (M0–M2), then security/async correctness (M3–M4), then DX and advanced surfaces (M5–M6).

## 9. Related Documents

- [`README.md`](../README.md) — project goal, milestones, and crate layout.
- [`docs/laravel-13-research.md`](laravel-13-research.md) — Laravel 13 feature research.
- [`docs/milestones.md`](milestones.md) — evidence-backed M0–M6 status.
