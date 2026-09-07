# Rustavel — API Catalog

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Source:** `design/api-contracts.md` · `requirements/prd.md` FR-000..612 · `requirements/fsd.md` FS-M0-01..M6-07
> **Conformance:** `technical-documentation/rules/api-module.md` Part A — every endpoint has Overview/Auth/Params/Body/Responses/Usage/OpenAPI

## 1. Global Standards

| Concern | Value |
|---------|-------|
| Base URL | `http://localhost:3000` (dev) — version prefix `/api/v1` where applicable |
| Content-Type | `application/vnd.api+json` for JSON:API resources; `application/json` for auth/validation 422 `ErrorBag` |
| Accept | `application/vnd.api+json` or `application/json` |
| Auth | `Authorization: Bearer <JWT>` (HS256, `jsonwebtoken` + `argon2`); `401` on missing/invalid/expired |
| Date format | ISO 8601 `YYYY-MM-DDTHH:mm:ssZ` (RFC3339) |
| Error shape | Typed `code`+`hint`+`source` chain per `fsd.md §4.1` — see Error Catalogue below |
| Rate limit signal | `429` + `Retry-After: <seconds>` (throttle middleware) |
| Pagination envelope | `{ data: T[], meta: {current_page,per_page,total,last_page}, links: {first,prev,next,last} }` |

## 2. Error Catalogue (wire-level)

| HTTP | Typed error | When |
|------|-------------|------|
| 200 | — | `Json<T>`, `View`, `EventStream` chunk |
| 201 | — | Resource created (`POST /users`, `POST /login` is 200) |
| 204 | — | `logout`, `schedule:resume` when already running |
| 401 | `AuthError::InvalidToken` / `ExpiredToken` / `BadCredentials` / `GuardMismatch` | Guard mismatch or bad creds |
| 403 | `CsrfError::UntrustedOrigin` / `BroadcastError::Unauthorized` / `StorageError::PathTraversal` | CSRF cross-site, channel auth, path confinement |
| 404 | `QueryError::NotFound` / `StorageError::NotFound` | `firstOrFail` missing, storage missing |
| 408 | `LockError::AlreadyHeld` (timeout path) | `Lock::block` timeout |
| 422 | `ErrorBag` JSON | Validation strict failure (`#[validate]`) |
| 429 | `Throttle` | `per_minute` exceeded; `Retry-After` header |
| 500 | `JobError::MaxAttemptsExceeded` / `AgentError::ToolNotFound` | Job dead-letter, agent mis-config |
| 501 | `UnsupportedCapability` / `McpUnavailable` | Provider lacks capability, MCP flag off |

## 3. Spec Index

All specs include 8 headers: Overview (method/path/summary/side effects), Auth, Params, Body, Responses (2xx+4xx/5xx with examples), Usage (runnable `curl` + success body), OpenAPI 3.0 YAML snippet. Each YAML snippet is valid OpenAPI 3.0 with `security` definitions.

| Module | Spec file | Endpoints / Contracts | Auth |
|--------|-----------|----------------------|------|
| foundation | [api-bootstrap.md](foundation/api-bootstrap.md) | `Application::configure`, `Container::Make<T>` — trait contracts (no HTTP) | — |
| http-routing | [api-routing.md](http-routing/api-routing.md) | `GET /users`, `POST /users`, `GET /users/{id}`, `PUT/PATCH /users/{id}`, `DELETE /users/{id}`, `GET /api/v1/route:list --json` shape, `Throttle`, `Cors` | Bearer optional |
| http-routing | [api-http-client.md](http-routing/api-http-client.md) | `Http::get().header().timeout().throw().send()` client contract + `HttpError` kinds | — |
| data-orm | [api-query-builder.md](data-orm/api-query-builder.md) | `QueryBuilder` fluent methods, `paginate`/`cursor`/`chunkBy`, `upsert`, `whereVectorSimilarTo`, `toSql`/`toRawSql`, pagination envelope | Bearer |
| identity-access | [api-auth.md](identity-access/api-auth.md) | `POST /login`, `POST /loginUsingId`, `POST /refresh`, `POST /logout`, `GET /me`, CSRF origin decision table | Bearer + CSRF |
| identity-access | [api-validation.md](identity-access/api-validation.md) | `422 ErrorBag` shape, strict `contains`/`in_array`, `#[validate]` vs `Validatable` | — |
| async-workloads | [api-queue.md](async-workloads/api-queue.md) | `Queue::route`, `dispatch`/`onQueue`/`onConnection`/`chain`/`batch`, `queue:work`/`failed`/`retry`, metrics `pendingSize`/… | — |
| async-workloads | [api-cache.md](async-workloads/api-cache.md) | `get`/`put`/`touch`/`remember`, `Lock::get`/`block`, hyphenated prefixes | — |
| async-workloads | [api-events-schedule.md](async-workloads/api-events-schedule.md) | `Dispatcher::dispatch`/`dispatchAfterResponse`, `Schedule::command().daily().cron()` … + `schedule:list`/`run`/`pause`/`resume` | — |
| developer-platform | [api-cli.md](developer-platform/api-cli.md) | `cargo rustavel list`, `make:*` generators, `Artisan::call`, `#[usage]`/`#[help]`/`#[hidden]` | — |
| intelligence-delivery | [api-broadcast.md](intelligence-delivery/api-broadcast.md) | `ws://…/broadcasting/auth`, `GET /events` SSE `eventStream`, `ShouldBroadcast` + channel auth | Bearer (private) |
| intelligence-delivery | [api-storage.md](intelligence-delivery/api-storage.md) | `Storage::disk("s3").get/put/path/exists` read-through + `PathTraversal` | — |
| intelligence-delivery | [api-jsonapi.md](intelligence-delivery/api-jsonapi.md) | `JsonApiResource` sparse fieldsets / `include` / `RelationNotLoaded` | Bearer |
| intelligence-delivery | [api-ai.md](intelligence-delivery/api-ai.md) | `Ai::provider("openai").text().stream()`, embeddings/rerank/files, `Agent`/`Tool`, MCP, sub-agents, middleware | Bearer |

## 4. Verification Checklist (A-Gate per spec)

- [x] Error response codes have realistic examples (JSON bodies with `errors[]`, `status`, `title`, `detail`).
- [x] YAML snippet passes validation (OpenAPI 3.0, `openapi: 3.0.3`, `info`, `paths`/`components`, `security`).
- [x] Parameter constraints captured (min/max, type, required, example).
- [x] `curl` example uses valid CLI syntax (headers, `--json`/`-d`, `Authorization`).

## 5. Skill Reference

| Layer | Skill | Rule |
|-------|-------|------|
| API specs | `technical-documentation` | `rules/api-module.md` Part A |
| Contract tests | `test-generation` | `rules/api-contract-test.md` |
| Security triage | `security-audit` | `rules/security-triage.md` |

Cross-links at the bottom of each `api-*.md` point to this catalog, to the module overview, and to the testing doc.
