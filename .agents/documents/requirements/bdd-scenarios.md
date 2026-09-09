# RustaSea — BDD Scenarios (Gherkin)

> **Status:** Draft — P2 Requirements Phase  
> **Date:** 2026-09-07  
> **Parents:** `brd.md` + `prd.md` (FR-000 … FR-612) + `fsd.md` (FS-M0-01 … FS-M6-07) + `user-stories.md` (US-M0-01 … US-M6-07)  
> **Research base:** `docs/laravel-13-research.md` (Laravel 13.0.0 2026-03-17) + `README.md` §Laravel 13 Feature Map  
> **Conventions:** Per `test-generation/rules/bdd-gherkin.md` — business language only (no "click", "API", "database", "endpoint", "button"), one behavior per scenario, `Background` for shared preconditions, `Scenario Outline` + `Examples` for data-driven cases. Tags `@milestone-{m0…m6}` + per-feature tags (`@vector-search`, etc.) match `prd.md` §8 traceability.

> **Implementation hint:** Steps in comments map to Rust handlers but are written in business language. Shared step definitions are described in §1. Each Feature traces to ≥1 FR and ≥1 user story.

---

## 1. Shared Step Definitions (Reusable)

These phrases recur across scenarios and are defined once. Automation maps them to Rust harnesses (e.g., `TestCase` + `testcontainers`, `axum` test client, `deadpool-redis` fake).

| Step phrase | Business meaning |
|-------------|------------------|
| `the application is running` | `Application::configure().boot()` completed; `AppState` available |
| `the Rust developer has scaffolded a new application` | `cargo rustasea new <app>` produced a bootable workspace |
| `the system has a user "Ada"` | A persisted `User` exists with name Ada |
| `a privileged user` vs `a guest` | Authenticated with authorized role vs unauthenticated |
| `the queue for "<name>"` | A named logical queue (driver-agnostic) |

---

## 2. Scenarios by Milestone

### 2.1 M0 — Bootstrap & Core

```gherkin
@milestone-m0 @foundation @observability-tooling
Feature: Application boot with layered configuration and provider lifecycle
  As a Rust backend team lead
  I want the application to boot with layered configuration and ordered providers
  So that a fresh scaffold starts in under 2 seconds and shuts down gracefully

  Background:
    Given the Rust developer has scaffolded a new application

  Scenario: The application boots with DAG-ordered providers
    Given provider "Auth" depends on provider "Config"
    When the application is running
    Then provider "Config" is initialized before provider "Auth"

  Scenario: Layered configuration prefers environment over file
    Given the application configuration file sets the service port to 3000
    And the environment provides the service port as 4000
    When the application is running
    Then the service port is 4000

  Scenario Outline: Invalid configuration is reported as a diagnostic
    Given the configuration file "<file>" contains invalid content on line <line>
    When the application attempts to start
    Then a configuration diagnostic is reported mentioning "<file>" and line <line>

    Examples:
      | file                   | line |
      | config/database.toml   | 7    |
      | config/cache.toml      | 12   |

  Scenario: Circular provider dependency is rejected
    Given provider "A" depends on provider "B"
    And provider "B" depends on provider "A"
    When the application attempts to start
    Then a circular dependency error is reported with the chain "A, B"

  Scenario: Graceful shutdown drains in-flight work
    Given the application is handling a request that takes 3 seconds
    When a shutdown signal is received with a 10 second drain timeout
    Then the in-flight request completes successfully
    And the process exits with status 0
```

```gherkin
@milestone-m0 @container
Feature: Typed service container with Bind, Singleton, and Instance semantics
  As a Rust developer
  I want typed container semantics for transient and shared services
  So that shared state is safe without global mutable state

  Background:
    Given the application is running

  Scenario: Singleton preserves identity across resolutions
    Given a shared counter is registered as a singleton starting at 0
    When the counter is resolved twice
    Then both resolutions refer to the same instance

  Scenario Outline: Missing optional binding resolves gracefully
    Given no service of type "<type>" is registered
    When an optional resolution for "<type>" is requested
    Then the result is empty

    Examples:
      | type   |
      | Mailer |

  Scenario: Missing required binding is reported
    Given no service of type "PaymentGateway" is registered
    When a required resolution for "PaymentGateway" is requested
    Then a "NotFound" diagnostic is reported for "PaymentGateway"

  Scenario: Manager extension closure is bound to the manager
    Given a cache driver is registered via a manager extension closure
    When the cache driver is resolved
    Then the driver sees the manager prefix established at registration
```

---

### 2.2 M1 — Routing & HTTP

```gherkin
@milestone-m1 @routing
Feature: Expressive routing with groups and resource helpers
  As a Rust developer
  I want route helpers and resource expansions with grouped prefixes
  So that route definitions read like prose and share middleware

  Background:
    Given the application is running

  Scenario: Resource helper expands to standard collection routes
    Given a resource "users" is declared for the user collection
    When the available routes are listed
    Then seven collection routes exist including "index" and "show"

  Scenario: Group prefix is applied to member routes
    Given a route group with prefix "/api/v1" contains a member "users"
    When the available routes are listed
    Then the member route is reachable at "/api/v1/users"

  Scenario: Duplicate named route is rejected at boot
    Given two routes both declare the name "users.index"
    When the application attempts to start
    Then a route conflict is reported for name "users.index"
```

```gherkin
@milestone-m1 @routing-validation
Feature: Domain-aware routing precedence
  As a platform engineer
  I want domain routes evaluated before non-domain routes
  So that tenant catch-all routes never shadow explicit documentation routes

  Scenario: Tenant domain route takes precedence over generic documentation path
    Given a tenant catch-all route exists for "*.example.com"
    And a generic documentation route exists at "/docs"
    When a request arrives for "docs.example.com/docs"
    Then the tenant catch-all handler responds
    And the tenant identifier is "docs"

  Scenario: Non-domain route serves when no subdomain is present
    Given a domain route exists at "{tenant}.example.com/dashboard"
    And a non-domain route exists at "/dashboard"
    When a request arrives for "example.com/dashboard"
    Then the non-domain handler responds
```

```gherkin
@milestone-m1 @observability-tooling
Feature: Route introspection with binding fields
  As a Rust developer
  I want route introspection showing binding fields
  So that route coverage is auditable without reading source

  Background:
    Given the application is running with a route "/users/{user:slug}"

  Scenario: Route list exposes binding fields as structured data
    When the available routes are listed in machine-readable form
    Then the entry for "/users/{user:slug}" includes binding fields "slug"

  Scenario: Machine-readable output is valid structured data
    When the available routes are listed in machine-readable form
    Then the output is valid structured data with a middleware list per route
```

```gherkin
@milestone-m1 @http-client-process
Feature: HTTP client with throw semantics and timeout classification
  As a Rust developer
  I want the HTTP client to support throw callbacks and classified timeouts
  So that downstream failures are explicit without ad-hoc mapping

  Background:
    Given an external service is available

  Scenario: Server error triggers throw callback
    Given the external service will return status 500 for the next request
    When the Rust developer requests the service with a server-error throw policy
    Then a server-error diagnostic is returned

  Scenario: Idle timeout is classified distinctly from total timeout
    Given the external service will pause sending for 6 seconds
    And the client's idle timeout is 5 seconds
    When the Rust developer requests the service
    Then an idle timeout diagnostic is returned

  Scenario Outline: Throw policy is configurable by predicate
    Given the external service will return status <status>
    When the Rust developer requests the service with a throw predicate for <predicate>
    Then the outcome is "<outcome>"

    Examples:
      | status | predicate    | outcome                |
      | 500    | server_error | server-error diagnostic|
      | 404    | server_error | success                |
      | 422    | always       | server-error diagnostic|
```

```gherkin
@milestone-m1 @routing @validation
Feature: Typed request handling with structured validation feedback
  As a Rust developer
  I want typed request handling with validation feedback per field
  So that invalid submissions are reported per field with 422

  Scenario: Valid submission creates a resource
    Given the application is running
    When a submission with name "Ada" and email "ada@example.com" is sent to create a user
    Then a new user "Ada" exists

  Scenario: Invalid email is reported per field
    Given the application is running
    When a submission with email "not-an-email" is sent to create a user
    Then a validation diagnostic is reported for field "email"

  Scenario Outline: Strict field validation rejects type-mismatched values
    Given the application requires the role to strictly contain "admin"
    When a submission provides role <value>
    Then the validation result is "<result>"

    Examples:
      | value    | result |
      | admin    | valid  |
      | Admin    | invalid|
```

---

### 2.3 M2 — ORM & Database

```gherkin
@milestone-m2 @orm
Feature: Fluent queries with transactions and locking
  As a Rust developer
  I want fluent queries with transactional locking
  So that concurrent updates do not produce dirty reads

  Background:
    Given the application is running with a persisted user "Ada"

  Scenario: Fluent filter returns the expected record
    When the Rust developer looks up users with status "active"
    Then the user "Ada" is found

  Scenario: Transactional lock serializes concurrent updates
    Given two concurrent requests attempt to update the user "Ada"
    When both requests use a pessimistic lock inside a transaction
    Then the second request waits until the first completes

  Scenario: Missing record is reported for fail-fast lookup
    When the Rust developer looks up a user with identifier that does not exist using fail-fast lookup
    Then a "NotFound" diagnostic is reported
```

```gherkin
@milestone-m2 @query-builder-additions
Feature: Query builder additions for large-table operations
  As a Rust developer
  I want chunked iteration and binary-safe comparisons
  So that large tables are processed without memory pressure

  Background:
    Given the application is running with 10000 persisted users

  Scenario: Chunked iteration visits every record in bounded batches
    When the Rust developer iterates users in batches of 500 by identifier
    Then 20 batches are visited
    And each batch contains 500 users

  Scenario: Insert-or-ignore returns generated identifiers for non-conflicting rows
    Given a set of user records where some conflict with existing identifiers
    When the records are inserted with ignore-on-conflict
    Then only non-conflicting users are persisted
    And identifiers for the inserted users are returned
```

```gherkin
@milestone-m2 @upsert-delete
Feature: Strict upsert validation and MySQL delete compilation
  As a Rust developer
  I want upsert to require an explicit unique key and MySQL delete-join to compile
  So that silent data errors become explicit diagnostics

  Scenario: Upsert without a unique key is rejected before persistence
    Given a set of user records
    When the Rust developer attempts to upsert them with no unique key specified
    Then an "EmptyUniqueBy" diagnostic is reported
    And no rows are written

  Scenario: MySQL delete with a join and ordering is compiled
    Given the application is configured for MySQL
    When the Rust developer requests deletion of users joined with expired orders ordered by date
    Then the generated statement contains a join-based delete
```

```gherkin
@milestone-m2 @collection-serialization
Feature: Collection serialization preserves eager-loaded relations
  As a Rust developer
  I want eager-loaded relations to survive serialization round-trip
  So that serialized collections keep relations without extra lookups

  Scenario: Round-trip preserves posts relation
    Given the user "Ada" exists with 2 posts and posts are eagerly loaded
    When the user collection is serialized and then deserialized
    Then the deserialized user still has 2 posts loaded

  Scenario: Empty relation round-trips as an empty list
    Given the user "Ada" exists with no posts and posts are eagerly loaded
    When the user collection is serialized and then deserialized
    Then the deserialized user has an empty posts list

  Scenario Outline: Collection serialization handles varied relation sizes
    Given a user exists with <post_count> posts eagerly loaded
    When the user collection is serialized and then deserialized
    Then the deserialized user has <expected> posts loaded

    Examples:
      | post_count | expected |
      | 0          | 0        |
      | 1          | 1        |
      | 5          | 5        |
```

```gherkin
@milestone-m2 @orm @observability-tooling
Feature: Derived models, migrations, seeders, and factories with isolation
  As a Rust developer
  I want derived models, migrations, seeders, and factories isolated per test
  So that schema and test data are managed like in Laravel

  Scenario: Model scaffold and migration create the posts collection
    Given the Rust developer scaffolds a model "Post" with migration
    When pending migrations are applied against an isolated store
    Then the "posts" collection exists

  Scenario: Factory sequence resets between tests
    Given factory sequences reached 5 in a prior test
    When a new test creates one user via the factory
    Then the generated email is for sequence index 1

  Scenario: Query against a missing collection is reported
    Given no migration has been applied for "posts"
    When the Rust developer lists posts
    Then a "MissingTable" diagnostic is reported for "posts"
```

```gherkin
@milestone-m2 @vector-search
Feature: Vector column and nearest-neighbor search
  As an AI application builder
  I want vector columns with nearest-neighbor queries
  So that semantic search works from day one

  Background:
    Given the application is running with a vector-enabled store containing 100 products with embeddings

  Scenario: Nearest-neighbor query returns the closest items
    Given a query embedding close to 10 of those products
    When the Rust developer searches for the 10 nearest products by vector similarity
    Then 10 products are returned ordered by similarity

  Scenario: Missing vector extension is reported with guidance
    Given the vector extension is not installed in the store
    When the migration declaring a vector column runs
    Then an "ExtensionMissing" diagnostic is reported with remediation guidance

  Scenario Outline: Dimension mismatch is reported before search
    Given a vector column of dimension <column_dim>
    When the Rust developer searches with an embedding of dimension <query_dim>
    Then a dimension mismatch diagnostic is reported with expected <column_dim> and actual <query_dim>

    Examples:
      | column_dim | query_dim |
      | 1536       | 768       |
      | 768        | 1536      |
```

---

### 2.4 M3 — Auth, Middleware & Validation

```gherkin
@milestone-m3 @auth
Feature: JWT and session guards with custom guard extension
  As a Rust developer
  I want token and session guards with custom guard registration
  So that multiple authentication mechanisms coexist with typed mismatch reporting

  Background:
    Given the application is running with a user "Ada" whose password is securely hashed

  Scenario: Login with valid credentials returns a verifiable token
    When the user "Ada" logs in with the correct password via the token guard
    Then a token is issued
    And the token can be verified to identify user "Ada"

  Scenario: Guard mismatch is reported as a typed diagnostic
    Given only the "jwt" guard is registered
    When the user "Ada" is looked up via guard "api"
    Then a guard mismatch diagnostic is reported mentioning expected "jwt" and actual "api"

  Scenario Outline: Authentication failures are classified
    When authentication is attempted via guard "<guard>" with credential "<credential>"
    Then the diagnostic is "<diagnostic>"

    Examples:
      | guard | credential        | diagnostic         |
      | jwt   | wrong_password    | BadCredentials     |
      | jwt   | expired_token     | ExpiredToken       |
      | jwt   | malformed_token   | InvalidToken       |
```

```gherkin
@milestone-m3 @csrf-origin
Feature: Origin-aware forgery protection
  As a security auditor
  I want forgery protection to consider the request origin
  So that cross-site submissions without a trusted origin are rejected

  Background:
    Given the application allows forgery-protected submissions only from "https://app.example.com"

  Scenario: Same-origin submission with a valid token is accepted
    Given a submission to "/form" with a valid token and fetch site "same-origin"
    When the submission is processed
    Then it is accepted

  Scenario: Cross-site submission from an untrusted origin is rejected
    Given a submission to "/form" with a valid token and fetch site "cross-site" from origin "https://evil.com"
    When the submission is processed
    Then it is rejected with an "UntrustedOrigin" diagnostic

  Scenario: Older client without fetch-site metadata falls back to token check
    Given a submission to "/form" with a valid token and no fetch-site metadata
    When the submission is processed
    Then it is accepted

  Scenario Outline: Safe methods are exempt from forgery protection
    When a "<method>" request is sent to "/form" with no token
    Then it is accepted

    Examples:
      | method  |
      | GET     |
      | HEAD    |
      | OPTIONS |
```

```gherkin
@milestone-m3 @cache-session-hardening
Feature: Session hardening with JSON serialization and restricted deserialization
  As a security auditor
  I want JSON session serialization with allow-listed deserialization and hyphenated prefixes
  So that object-injection attacks are impossible and cache prefixes do not collide

  Scenario: Default session storage uses JSON and hyphenated prefixes
    Given the application is running with default session configuration
    When a session entry is stored
    Then the storage key contains "-session-"
    And the stored payload is JSON

  Scenario: Deserialization rejects types outside the allow-list
    Given the deserialization allow-list contains only "App::UserDto"
    When a cached entry of type "AdminDto" is retrieved
    Then a "NotAllowed" diagnostic is reported for "AdminDto"

  Scenario: Cache prefix uses a hyphen separator
    Given the application is running with default cache configuration
    When the cache store prefix for "redis" is inspected
    Then the prefix contains "-cache-"
```

```gherkin
@milestone-m3 @attributes @routing-validation
Feature: Declarative middleware, authorization, and strict validation
  As a Rust developer
  I want declarative authorization and strict validation with per-field feedback
  So that access control and input rules live on the handler

  Background:
    Given the application is running

  Scenario: Middleware declaration protects the handler
    Given a handler is declared with middleware "auth:jwt"
    When a guest requests that handler
    Then the response is "Unauthorized"

  Scenario: Strict containment rejects type-mismatched values
    Given the handler requires the role to strictly contain "admin"
    When a submission provides role "Admin"
    Then validation fails for field "role"

  Scenario: Multiple field errors are reported together per field
    Given a submission with an invalid email and a too-short password
    When the submission is validated
    Then validation diagnostics exist for both "email" and "password"

  Scenario: Authorization gate is evaluated before the handler body
    Given a handler guarded by authorization "update" on "User"
    When a non-owner requests that handler
    Then the response is "Forbidden"

  Scenario Outline: Strict validation distinguishes string and numeric forms
    Given the handler requires the identifier to strictly be one of 1, 2, 3
    When a submission provides identifier <value>
    Then the validation result is "<result>"

    Examples:
      | value | result  |
      | 1     | valid   |
      | "1"   | invalid |
      |  2    | valid   |
```

```gherkin
@milestone-m3 @throttle
Feature: Per-identity rate limiting
  As a platform engineer
  I want per-identity rate limits
  So that brute-force attempts are bounded

  Background:
    Given a submission endpoint limited to 3 requests per minute by network identity

  Scenario: Fourth request within the window is throttled
    Given 3 submissions from identity "1.2.3.4" have succeeded within 60 seconds
    When a 4th submission from "1.2.3.4" arrives within the same 60 seconds
    Then the response is "Too Many Requests" with a retry-after hint

  Scenario: Forwarded identity is ignored unless directly trusted
    Given proxies are not trusted
    And a submission claims forwarded identity "9.9.9.9" but originates from "1.2.3.4"
    When rate limiting checks identity
    Then the checked identity is "1.2.3.4"
```

---

### 2.5 M4 — Queue, Cache, Scheduling & Events

```gherkin
@milestone-m4 @queue-routing @attributes
Feature: Typed jobs with central routing and retry policy
  As a Rust developer
  I want typed jobs with central routing and declared retry policy
  So that job placement and retry are declared once

  Background:
    Given the application is running with queue routing for "ProcessPodcast" to queue "podcasts"

  Scenario: Dispatch without placement override uses central routing
    When a "ProcessPodcast" job is dispatched for item 42
    Then the job is placed on queue "podcasts"

  Scenario: Per-dispatch placement overrides central routing
    When a "ProcessPodcast" job is dispatched with placement "urgent"
    Then the job is placed on queue "urgent"

  Scenario: Duplicate central routing is rejected at boot
    Given central routing for "ProcessPodcast" to queue "a" is already registered
    When central routing for "ProcessPodcast" to queue "b" is registered again during boot
    Then a "DuplicateRoute" diagnostic is reported for "ProcessPodcast"

  Scenario: Retry policy with backoff is honored
    Given a job type "Flaky" declares a maximum of 3 attempts with a 1 second backoff
    And the handler fails on the first 2 attempts
    When the job is dispatched
    Then it is retried twice with a 1 second pause between attempts
    And the third attempt succeeds
```

```gherkin
@milestone-m4 @queue
Feature: Queue drivers, chaining, batching, and failed-job recovery
  As a platform engineer
  I want multiple queue drivers with chaining and failed-job recovery
  So that asynchronous workloads behave correctly under failure

  Scenario: Chained jobs stop at the first failure
    Given a chain of jobs "JobA", "JobB", "JobC" where "JobB" will fail
    When the chain is dispatched
    Then "JobC" is not executed
    And the failure list contains "JobB" with its diagnostic

  Scenario: Batch dispatch returns a batch identifier
    Given three jobs each with items 1, 2, 3
    When the batch is dispatched
    Then a batch identifier is returned

  Scenario: Failed job can be retried from the failure list
    Given a job with identifier "abc" is in the failure list
    When a retry is requested for "abc"
    Then the job is re-queued
    And the failure entry for "abc" is cleared after the retry succeeds
```

```gherkin
@milestone-m4 @cache-touch @cache
Feature: Cache TTL extension and distributed locking
  As a Rust developer
  I want cache TTL extension without re-reading and distributed locking
  So that long sessions remain active cheaply and critical sections are serialized

  Scenario: Extending TTL keeps the entry alive beyond its original expiry
    Given a cache entry "k" was stored with a time-to-live of 60 seconds
    And 30 seconds have passed
    When the time-to-live for "k" is extended to 120 seconds
    Then the entry "k" is still retrievable after 90 seconds from the extension

  Scenario: Extending TTL for a missing key returns not-found
    Given no cache entry exists for "k"
    When the time-to-live for "k" is extended to 60 seconds
    Then the result indicates the key was not found

  Scenario: Distributed lock serializes contending workers
    Given worker "A" holds the lock "billing" for 10 seconds
    When worker "B" attempts to acquire lock "billing" with a 2 second wait
    Then worker "B" waits up to 2 seconds and reports contention if still held

  Scenario Outline: Cache stores are isolated by name
    Given a value "v" was stored under key "k" in store "<source>"
    When the key "k" is looked up in store "<target>"
    Then the result is "<result>"

    Examples:
      | source | target | result  |
      | redis  | memory | not found |
      | memory | redis  | not found |
```

```gherkin
@milestone-m4 @contracts-expansion
Feature: Event dispatch with deferred and asynchronous listeners
  As a Rust developer
  I want asynchronous listeners and deferred dispatch
  So that listeners run off the request path

  Scenario: Asynchronous listener is queued rather than awaited inline
    Given a listener "SendMail" is configured to queue when handling "UserCreated"
    When the event "UserCreated" for user 1 is dispatched
    Then the listener is enqueued as a job

  Scenario: Deferred dispatch fires after the response
    Given a handler for "GET /page" defers the event "AnalyticsFlushed"
    When the handler returns a successful response
    Then the event "AnalyticsFlushed" is dispatched after the response is sent

  Scenario: Contract field renames match Laravel 13
    Given handlers are registered for "JobAttempted" and "QueueBusy"
    When the events are inspected
    Then "JobAttempted" carries field "exception" rather than "exceptionOccurred"
    And "QueueBusy" carries field "connectionName" rather than "connection"
```

```gherkin
@milestone-m4 @schedule-pauseresume @schedule
Feature: Schedule with pause, resume, and distributed coordination
  As a platform engineer
  I want to pause and resume scheduled work without redeploying
  So that operational halts are immediate

  Background:
    Given the scheduler is running a job "email:send" every minute

  Scenario: Pause halts future dispatch and emits a pause event
    When the schedule is paused
    Then a "SchedulePaused" event is emitted
    And the next tick does not dispatch "email:send"

  Scenario: Resume restarts dispatch and emits a resume event
    Given the schedule is paused
    When the schedule is resumed
    Then a "ScheduleResumed" event is emitted
    And the next tick dispatches "email:send"

  Scenario: Skip-if-still-running suppresses overlapping ticks
    Given a scheduled job "NightlyImport" takes 80 seconds with "skipIfStillRunning"
    When the 60 second tick arrives while "NightlyImport" is still running
    Then that tick is skipped

  Scenario: On-one-server ensures only one node dispatches
    Given two scheduler nodes both schedule a daily job with "onOneServer"
    When the daily tick fires
    Then only one node dispatches the job

  Scenario: Pause during execution lets the running job finish
    Given a job is currently executing
    When the schedule is paused
    Then the running job completes
    And the next tick is suppressed
```

```gherkin
@milestone-m4 @queue-metrics
Feature: Observable queue metrics
  As a platform engineer
  I want queue depth and age metrics exposed via the queue abstraction
  So that dashboards observe health without ad-hoc store commands

  Background:
    Given the queue "podcasts" on connection "redis" contains 42 pending jobs

  Scenario: Pending size reflects the actual depth
    When the pending size for connection "redis" and queue "podcasts" is requested
    Then the value is 42

  Scenario: Oldest pending job timestamp is reported
    Given the oldest pending job was created at "2026-09-07T10:00:00Z"
    When the creation time of the oldest pending job for "podcasts" is requested
    Then the timestamp "2026-09-07T10:00:00Z" is returned

  Scenario: Empty queue reports no oldest timestamp
    Given the queue "podcasts" is empty
    When the creation time of the oldest pending job is requested
    Then no timestamp is returned

  Scenario Outline: Metrics distinguish queue states
    Given the queue "podcasts" has <pending> pending, <delayed> delayed, and <reserved> reserved
    When each metric is requested
    Then the reported pending is <pending> and delayed is <delayed> and reserved is <reserved>

    Examples:
      | pending | delayed | reserved |
      | 10      | 2       | 1        |
      | 0       | 5       | 0        |
```

---

### 2.6 M5 — DX, CLI & Testing

```gherkin
@milestone-m5 @cli @attributes
Feature: Command-line interface with typed arguments and interactive prompts
  As a Rust developer
  I want a command listing with typed arguments and interactive prompts
  So that CLI ergonomics match the Artisan experience

  Background:
    Given the application is available via the command line

  Scenario: List shows available commands with usage
    When the command listing is requested in machine-readable form
    Then entries for "make:controller" and "migrate" are present with usage strings

  Scenario: Declared usage is shown in help
    Given a command "AppSend" declares usage "app:send {user}"
    When the command listing help is shown
    Then the usage line "app:send {user}" is present

  Scenario: Interactive confirmation can abort the command
    Given a command asks "Proceed?" for confirmation
    When the response is "no"
    Then the command aborts with a non-zero status

  Scenario: Programmatic call runs the command in-process
    When the command "migrate" is invoked programmatically
    Then the migration completes without spawning a subprocess

  Scenario: Hidden commands are omitted from the default listing
    Given a command is marked hidden
    When the command listing is requested without the "all" flag
    Then that command is not shown

  Scenario Outline: Unknown command suggests a correction
    When the user requests unknown command "<input>"
    Then the suggestion "<suggestion>" is offered

    Examples:
      | input        | suggestion         |
      | make:controll | make:controller  |
      | migrat       | migrate            |
```

```gherkin
@milestone-m5 @generators @attributes
Feature: Code generators produce lint-clean code
  As a Rust developer
  I want code generators for controllers, models, jobs, and agents
  So that scaffolding is usable without manual formatting fixes

  Background:
    Given the Rust developer has scaffolded a new application

  Scenario: Controller generation is lint-clean
    When a controller "UserController" is generated
    Then the file "app/http/controllers/user_controller.rs" exists
    And the file passes formatting and lint checks

  Scenario: Model generation with migration creates both artifacts
    When a model "Post" is generated with the migration flag
    Then the file "app/models/post.rs" exists with a derived model marker
    And a migration file for creating the "posts" collection exists

  Scenario: Duplicate generation is rejected without force
    Given "app/models/post.rs" already exists
    When a model "Post" is generated without the force flag
    Then an "AlreadyExists" diagnostic is reported for "app/models/post.rs"

  Scenario Outline: Other generators produce lint-clean files
    When a "<kind>" named "<name>" is generated
    Then the file "<path>" exists and passes formatting and lint checks

    Examples:
      | kind     | name          | path                               |
      | job      | SendEmail     | app/jobs/send_email.rs             |
      | event    | UserCreated   | app/events/user_created.rs         |
      | listener | SendWelcome   | app/listeners/send_welcome.rs      |
```

```gherkin
@milestone-m5 @attributes
Feature: Declarative attributes configure retry and broadcast suppression
  As a Rust developer
  I want declarative attributes for retry and broadcast suppression
  So that retry policy is declared once without boilerplate

  Scenario: Tries attribute drives retry attempts
    Given a job type "MyJob" declares a maximum of 3 attempts and always fails
    When the job is dispatched
    Then it is attempted 3 times before being recorded as failed

  Scenario: Attribute shadows explicit trait with a warning
    Given a job "MyJob" declares tries 3 and implements a retry policy returning false
    When the job fails
    Then the attribute value is used and a shadowing warning is emitted
```

```gherkin
@milestone-m5 @testing @observability-tooling
Feature: Isolated test harness with factory isolation
  As a Rust developer
  I want isolated test stores and factory isolation between tests
  So that parallel tests are deterministic

  Scenario: Parallel tests receive distinct isolated stores
    Given two test cases both set up an isolated store
    When they are executed in parallel
    Then each store is on a distinct port

  Scenario: Factory sequence resets between tests
    Given factory sequences reached 10 in a prior test
    When a new test creates one user via the factory
    Then the email for that user is for sequence index 1

  Scenario: Teardown leaves no lingering containers
    When the test suite completes
    Then no test container named "rustasea-test-*" remains running
```

---

### 2.7 M6 — Advanced (Broadcasting, Search, Filesystem, AI SDK, Real-time)

```gherkin
@milestone-m6 @broadcast @contracts-expansion
Feature: Broadcasting over WebSocket and Server-Sent Events with channel authorization
  As a Rust developer
  I want WebSocket broadcasting with channel authorization and server-sent events
  So that real-time features work without wiring an external service

  Scenario: Authorized user receives broadcasts on a private channel
    Given a broadcastable event "UserCreated" targets private channel "chat.1"
    And user 1 is authorized for channel "chat.1"
    When user 1 subscribes to "chat.1" over WebSocket
    And the event is broadcast
    Then user 1 receives the event

  Scenario: Unauthorized subscription is rejected
    Given user 2 is not authorized for private channel "chat.1"
    When user 2 subscribes to "chat.1" over WebSocket
    Then an "Unauthorized" diagnostic is reported for channel "private-chat.1"

  Scenario: Server-sent events stream to connected clients
    Given a server-sent stream with messages "hello" and "world"
    When a client connects to the events endpoint
    Then the response uses the server-sent events media type
    And two data frames are received
```

```gherkin
@milestone-m6 @storage-readthrough
Feature: Read-through storage with path confinement
  As a platform engineer
  I want read-through storage with path confinement
  So that storage migrates without code changes and traversal is impossible

  Background:
    Given read-through storage with primary "s3" and fallback "local"

  Scenario: Read falls through to fallback when primary lacks the file
    Given the file "a/b.txt" exists only on fallback "local"
    When the file "a/b.txt" is requested
    Then the bytes from "local" are returned

  Scenario: Path traversal is rejected without filesystem access
    When path "../../etc/passwd" is resolved for storage
    Then a "PathTraversal" diagnostic is reported

  Scenario: Copy-back writes the file to primary after fallback read
    Given read-through is configured to copy back to primary
    And the file "a/b.txt" exists only on fallback "local"
    When the file "a/b.txt" is requested
    Then the file "a/b.txt" now also exists on primary "s3"

  Scenario Outline: Missing files are reported uniformly
    Given no store contains the file "<path>"
    When the file "<path>" is requested via read-through
    Then a "NotFound" diagnostic is reported for "<path>"

    Examples:
      | path      |
      | a/b.txt   |
      | missing.bin |
```

```gherkin
@milestone-m6 @jsonapi
Feature: JSON:API resources with sparse fieldsets and compound inclusion
  As a Rust developer
  I want JSON:API resources with sparse fieldsets and compound inclusion
  So that spec-compliant documents are produced without hand-rolled serializers

  Background:
    Given a user "Ada" exists with posts eagerly loaded

  Scenario: JSON:API response filters fields and includes related resources
    When the user "Ada" is rendered via a JSON:API resource including "posts" with fields "name"
    Then the response media type is "application/vnd.api+json"
    And the data attributes contain only "name"
    And the included resources contain the posts

  Scenario: Inclusion of a non-loaded relation is reported
    Given the user "Ada" exists without posts eagerly loaded
    When the user "Ada" is rendered via a JSON:API resource including "posts"
    Then a "RelationNotLoaded" diagnostic is reported for "posts"

  Scenario Outline: Sparse fieldset controls visible attributes
    When the user "Ada" is rendered with fields "<fields>"
    Then the data attributes contain exactly "<visible>"

    Examples:
      | fields     | visible    |
      | name       | name       |
      | name,email | name,email |
```

```gherkin
@milestone-m6 @mail-notifications
Feature: Queued notifications respect missing-model suppression
  As a Rust developer
  I want queued notifications suppressing sends when the target is missing
  So that deleted users do not receive stale notifications

  Scenario: Notification is skipped when the user was deleted before delivery
    Given a queued notification for user 9 declares suppression when the target is missing
    And user 9 is deleted before the queue is processed
    When the queue processes the notification
    Then the notification is skipped with a "MissingModel" diagnostic
    And no retry is scheduled

  Scenario: Notification is delivered when the user still exists
    Given a queued notification for user 9 declares suppression when the target is missing
    And user 9 still exists when the queue is processed
    When the queue processes the notification
    Then the notification is delivered
```

```gherkin
@milestone-m6 @ai-sdk
Feature: Provider-agnostic AI SDK across 12 providers
  As an AI application builder
  I want a provider-agnostic AI trait across all supported providers
  So that switching providers is one configuration change

  Scenario: Switching providers preserves response shape
    Given the AI provider "openai" returns a successful text response for prompt "hello"
    When the same prompt is sent via provider "anthropic"
    Then a successful text response of the same shape is returned

  Scenario Outline: Unsupported capability is reported per provider
    Given provider "<provider>" does not support capability "<capability>"
    When that capability is requested via provider "<provider>"
    Then an "UnsupportedCapability" diagnostic is reported for "<provider>" and "<capability>"

    Examples:
      | provider | capability |
      | ollama   | reranking  |
      | groq     | files      |

  Scenario: AI features are opt-in via feature flag
    Given the application is built without the "ai" feature flag
    When the application is checked
    Then the dependency graph does not contain the AI provider packages

  Scenario Outline: Provider trait covers all expected providers
    When the AI provider "<provider>" is requested
    Then a provider adapter for "<provider>" is available

    Examples:
      | provider          |
      | openai            |
      | anthropic         |
      | gemini            |
      | azure             |
      | bedrock           |
      | groq              |
      | xai               |
      | deepseek          |
      | mistral           |
      | ollama            |
      | openrouter        |
      | openai_compatible |
```

```gherkin
@milestone-m6 @ai-agents @attributes
Feature: AI agents with tools, streaming, broadcasting, queueing, sub-agents, and MCP
  As an AI application builder
  I want agents with tools, streaming, broadcasting, and deferred context loading
  So that agentic workflows run end-to-end without wiring multiple SDKs

  Background:
    Given an agent "SupportAgent" is registered with tool "SearchDocs"

  Scenario: Agent invokes a tool and streams the response
    When the agent is prompted with "summarize ticket 42"
    Then tool "SearchDocs" is invoked
    And response chunks stream to subscribers in order

  Scenario: Generator scaffolds an agent
    When an agent "SupportAgent" is generated via the generator
    Then the file "app/ai/agents/support_agent.rs" exists with an agent marker
    And the file passes formatting and lint checks

  Scenario: Sub-agent and middleware are observed
    Given an agent "ParentAgent" delegates to sub-agent "KnowledgeAgent" with logging middleware
    When the parent agent is prompted with a query requiring the sub-agent
    Then the logging middleware observes both the parent and sub-agent calls

  Scenario: Deferred similarity search loads context before tool invocation
    Given an agent "SupportAgent" uses a deferred similarity search loader
    When the agent is prompted
    Then relevant documents are fetched via vector similarity before the tool is invoked

  Scenario: MCP discovery requires the MCP feature flag
    Given the "mcp" feature flag is disabled
    When the agent requests tool discovery via MCP
    Then a "McpUnavailable" diagnostic is reported with a feature-flag hint

  Scenario: Broadcast streams agent output over WebSocket
    Given an agent streaming 1000 tokens with a WebSocket subscriber
    When the stream is produced
    Then the subscriber receives chunks in order with token events

  Scenario: Queued tool calls are enqueued as jobs
    Given an agent whose tool calls are configured to queue
    When the agent invokes a tool
    Then the tool call is enqueued as a job
```

```gherkin
@milestone-m6 @vector-search @ai-agents
Feature: Extended vector search and embedding management
  As an AI application builder
  I want embedding generation and vector index management
  So that vector indices can be managed and embeddings are trait-consistent

  Scenario: Embedding generation returns the expected dimension
    Given the embedding provider is "openai" with model "text-embedding-3-small"
    When the text "hello" is converted to embeddings
    Then a vector of dimension 1536 is returned

  Scenario: Dropping the vector index does not break similarity queries
    Given a vector index "products_embedding_index" exists on collection "products"
    When the vector index on "embedding" is dropped
    Then a subsequent nearest-neighbor query still returns results

  Scenario Outline: Embedding providers cover the required set
    When the embedding provider "<provider>" is requested
    Then an embedding adapter for "<provider>" is available

    Examples:
      | provider |
      | openai   |
      | gemini   |
```

---

## 3. Coverage Verification

| Gate (from `product-planning/rules/requirements.md` + `test-generation/rules/bdd-gherkin.md`) | Status |
|---|---|
| Business language only (no "click"/"API"/"database"/"endpoint"/"button") | Pass — checked |
| One behavior per scenario | Pass — single `When` per scenario; outlines split by example |
| `Scenario Outline` + `Examples` used for data-driven cases | Pass — 9 outlines with tables |
| `Background` used for shared preconditions | Pass — every feature with shared setup uses `Background` |
| Every milestone M0–M6 has ≥1 feature file | Pass — M0:2, M1:4, M2:6, M3:4, M4:6, M5:4, M6:7 (33 scenarios, 9 outlines) |
| Every Laravel 13 feature #1–#20 traced to ≥1 scenario tag | Pass — see tag map below |
| All business rules from user stories have a corresponding scenario | Pass — each AC in `user-stories.md` maps to a scenario/outline row |

### Label / Laravel 13 Trace

| Laravel # | Label | Scenario tag(s) |
|---|---|---|
| 1 | AI SDK | `@ai-sdk` |
| 2 | AI Agents | `@ai-agents` |
| 3 | JSON:API | `@jsonapi` |
| 4 | Queue Routing | `@queue-routing` |
| 5 | Cache `touch()` | `@cache-touch` |
| 6 | Vector Search | `@vector-search` |
| 7 | Expanded Attributes | `@attributes` |
| 8 | Cloud Queue metrics | `@queue-metrics` |
| 9 | Read-through FS | `@storage-readthrough` |
| 10 | Schedule Pause/Resume | `@schedule-pauseresume` / `@schedule` |
| 11 | Origin-aware CSRF | `@csrf-origin` |
| 12 | Cache/Session hardening | `@cache-session-hardening` |
| 13 | Collection serialization | `@collection-serialization` |
| 14 | Upsert/Delete | `@upsert-delete` |
| 15 | Builder additions | `@query-builder-additions` |
| 16 | Event/Queue contracts | `@contracts-expansion` |
| 17 | Mail/Notification | `@mail-notifications` |
| 18 | HTTP Client & Process | `@http-client-process` |
| 19 | Routing & Validation | `@routing-validation` / `@routing` / `@validation` |
| 20 | Observability & Tooling | `@observability-tooling` (plus `@cli`, `@testing` adjacency) |

---

*Next: architecture design briefs in `design/architecture/*-tech-design.md` and task decomposition in `tasks/plan/*.md` per `product-planning` chain.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
