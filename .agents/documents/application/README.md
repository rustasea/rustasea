# Rustavel — Application Documentation

> **Status:** P8 Complete — 2026-09-07 | **Task:** TASK-013
> **Parents:** `requirements/{brd,prd,fsd,tdd,bdd-scenarios,user-stories}.md` · `design/{architecture,domain,database,api-contracts,component-inventory,flows}.md` · `design/decisions/ADR-00*.md` · `tasks/sprints/{manifest,sprint-0*.md}`
> **Conformance:** `technical-documentation/rules/api-module.md` Part A (OpenAPI) + Part B (Module Deep Docs) · `test-planning/qa-design` · `test-generation/bdd-gherkin+api-contract-test` · `security-audit` · `non-functional-testing/chaos`

## 1. Purpose

This tree is the **application-level deep documentation** produced at P8. It turns the planning/design docs (P2–P6) into module, API, and testing documentation that is navigable by crate and milestone. Every crate in the workspace has an `overview.md` and at least one `feature.md`; every public HTTP/CLI surface has an OpenAPI/CLI spec; every feature has linked test scenarios covering positive, negative, security, and chaos cases.

## 2. Documentation Tree

```
.agents/documents/application/
├── README.md                          ← you are here
├── modules/
│   ├── manifest.md                    ← module index (7 modules, 20+ features, no-gap proof)
│   ├── foundation/overview.md + *.md  (M0 — Application, Container, Config, Shutdown)
│   ├── http-routing/overview.md + *.md (M1 — Router, Middleware, Http Client)
│   ├── data-orm/overview.md + *.md    (M2 — Model, Query Builder, Migrations, Vector)
│   ├── identity-access/overview.md+*.md (M3 — Auth, CSRF, Validation)
│   ├── async-workloads/overview.md+*.md (M4 — Queue, Cache, Events, Schedule)
│   ├── developer-platform/overview.md+*.md (M5 — CLI, Generators, Testing Harness)
│   └── intelligence-delivery/overview.md+*.md (M6 — Broadcast, Storage, JSON:API, AI)
├── api/
│   ├── README.md                      ← API catalog + global standards
│   ├── foundation/api-*.md            (M0 contracts — builder/trait)
│   ├── http-routing/api-*.md          (M1 routes + Http client + route:list)
│   ├── data-orm/api-*.md              (M2 builder + pagination envelope)
│   ├── identity-access/api-*.md       (M3 login/refresh/me + CSRF + validation 422)
│   ├── async-workloads/api-*.md       (M4 queue/cache/events/schedule)
│   ├── developer-platform/api-*.md    (M5 CLI + Artisan::call)
│   └── intelligence-delivery/api-*.md (M6 broadcast/SSE/storage/jsonapi/ai)
└── testing/
    ├── README.md                      ← testing catalog (from P6)
    ├── contracts/*.schema.json + fixtures + stubs
    └── {module}/overview.md + test-*.md  (one testing doc per module)
```

## 3. Module Index (7 modules → 20 crates)

| # | Module slug | Crate(s) | Milestone | BR | Overview |
|---|-------------|----------|-----------|----|----------|
| 1 | `foundation` | `rustavel-foundation`, `rustavel-config`, `rustavel-container`, `rustavel` umbrella | M0 | BR-01 | [modules/foundation/overview.md](modules/foundation/overview.md) |
| 2 | `http-routing` | `rustavel-router`, `rustavel-http`, `rustavel-macros` (route/middleware) | M1 | BR-02 | [modules/http-routing/overview.md](modules/http-routing/overview.md) |
| 3 | `data-orm` | `rustavel-orm`, `rustavel-macros` (Model) | M2 | BR-03 | [modules/data-orm/overview.md](modules/data-orm/overview.md) |
| 4 | `identity-access` | `rustavel-auth`, `rustavel-validation` | M3 | BR-04 | [modules/identity-access/overview.md](modules/identity-access/overview.md) |
| 5 | `async-workloads` | `rustavel-queue`, `rustavel-cache`, `rustavel-events`, `rustavel-schedule` | M4 | BR-05 | [modules/async-workloads/overview.md](modules/async-workloads/overview.md) |
| 6 | `developer-platform` | `rustavel-cli`, `rustavel-macros`, `rustavel-testing`, `xtask` | M5 | BR-06 | [modules/developer-platform/overview.md](modules/developer-platform/overview.md) |
| 7 | `intelligence-delivery` | `rustavel-broadcast`, `rustavel-storage`, `rustavel-search`, `rustavel-ai`, `rustavel-jsonapi` | M6 | BR-07 | [modules/intelligence-delivery/overview.md](modules/intelligence-delivery/overview.md) |

**Crate coverage:** every workspace crate appears in exactly one module row; see [modules/manifest.md](modules/manifest.md) for the FR→FS→FR→Module→API→Test trace.

## 4. API Catalog

All HTTP endpoints use `Content-Type: application/vnd.api+json` (JSON:API 1.1) unless noted. Auth is `Bearer JWT` (HS256 via `jsonwebtoken`). Pagination envelope: `data[] + meta{current_page,per_page,total,last_page} + links{first,prev,next,last}`.

| Module | Spec | Endpoints / Contracts | Auth |
|--------|------|-----------------------|------|
| foundation | [api/foundation/api-bootstrap.md](api/foundation/api-bootstrap.md) | `Application::configure`, Container `Make<T>` (trait contracts) | — |
| http-routing | [api/http-routing/api-routing.md](api/http-routing/api-routing.md) | `GET /users`, `POST /users`, `GET /users/{id}`, route:list, `throttle`, `cors` | Optional throttle |
| http-routing | [api/http-routing/api-http-client.md](api/http-routing/api-http-client.md) | `Http::get().throw().timeout()` (client contract) | — |
| data-orm | [api/data-orm/api-query-builder.md](api/data-orm/api-query-builder.md) | `where`/`paginate`/`chunkBy`/`upsert`/`vector` (builder contract) | Bearer |
| identity-access | [api/identity-access/api-auth.md](api/identity-access/api-auth.md) | `POST /login`, `/loginUsingId`, `/refresh`, `/logout`, `GET /me`, CSRF | Bearer + CSRF |
| identity-access | [api/identity-access/api-validation.md](api/identity-access/api-validation.md) | `422 ErrorBag` shape (validation contract) | Bearer |
| async-workloads | [api/async-workloads/api-queue.md](api/async-workloads/api-queue.md) | `dispatch`/`chain`/`batch`/`queue:work`/`queue:retry`, metrics | Bearer |
| async-workloads | [api/async-workloads/api-cache.md](api/async-workloads/api-cache.md) | `get`/`put`/`touch`/`remember`, `Lock::get`/`block` | — |
| async-workloads | [api/async-workloads/api-events-schedule.md](api/async-workloads/api-events-schedule.md) | `dispatch`/`dispatchAfterResponse`, `schedule:run`/`pause`/`resume` | — |
| developer-platform | [api/developer-platform/api-cli.md](api/developer-platform/api-cli.md) | `cargo rustavel list`, `make:*`, `Artisan::call` | — |
| intelligence-delivery | [api/intelligence-delivery/api-broadcast.md](api/intelligence-delivery/api-broadcast.md) | `ws://…/broadcasting/auth`, `GET /events` (SSE) + `ShouldBroadcast` | Bearer (private) |
| intelligence-delivery | [api/intelligence-delivery/api-storage.md](api/intelligence-delivery/api-storage.md) | `GET /storage/{path}` read-through + `PathTraversal` | Bearer |
| intelligence-delivery | [api/intelligence-delivery/api-jsonapi.md](api/intelligence-delivery/api-jsonapi.md) | JSON:API `data/included/links/meta`, sparse fieldsets | Bearer |
| intelligence-delivery | [api/intelligence-delivery/api-ai.md](api/intelligence-delivery/api-ai.md) | `Ai::provider().text().stream()`, embeddings, `Agent`/`Tool`, MCP | Bearer (agent) |

Details: [api/README.md](api/README.md) — global standards + per-spec links + OpenAPI YAML index.

## 5. Testing Index

| Module | Overview | Specs |
|--------|----------|-------|
| foundation | [testing/foundation/overview.md](testing/foundation/overview.md) | [testing/foundation/test-boot-container.md](testing/foundation/test-boot-container.md) |
| http-routing | [testing/http-routing/overview.md](testing/http-routing/overview.md) | [testing/http-routing/test-routing.md](testing/http-routing/test-routing.md) |
| data-orm | [testing/data-orm/overview.md](testing/data-orm/overview.md) | [testing/data-orm/test-orm.md](testing/data-orm/test-orm.md) |
| identity-access | [testing/identity-access/overview.md](testing/identity-access/overview.md) | [testing/identity-access/test-auth-validation.md](testing/identity-access/test-auth-validation.md) |
| async-workloads | [testing/async-workloads/overview.md](testing/async-workloads/overview.md) | [testing/async-workloads/test-queue-cache.md](testing/async-workloads/test-queue-cache.md) |
| developer-platform | [testing/developer-platform/overview.md](testing/developer-platform/overview.md) | [testing/developer-platform/test-cli.md](testing/developer-platform/test-cli.md) |
| intelligence-delivery | [testing/intelligence-delivery/overview.md](testing/intelligence-delivery/overview.md) | [testing/intelligence-delivery/test-advanced.md](testing/intelligence-delivery/test-advanced.md) |

Contracts/fixtures/stubs: [testing/README.md](testing/README.md) + [testing/contracts/](testing/contracts/) + [testing/fixtures/](testing/fixtures/) + [testing/stubs/](testing/stubs/).

## 6. How to Use

- **Implement a feature:** open its `modules/<module>/<feature>.md` → read User Stories + Business Logic + Sequence/ERD + Public Interface → check `api/<module>/api-<feature>.md` for wire contract → write tests per `testing/<module>/test-<feature>.md`.
- **Review a crate boundary:** read `modules/<module>/overview.md` (architecture diagram + dependency table) + `design/architecture.md §3 DAG`.
- **Run the executable trace:** `cargo test -- --list | grep stub:` (stubs are `#[ignore]` until crates land); `cargo insta test --accept` for snapshots.

## 7. Skill Reference

| Doc layer | Generating skill | Rule file |
|-----------|-----------------|-----------|
| Module deep docs | `technical-documentation` | `rules/api-module.md` Part B |
| API specs (OpenAPI) | `technical-documentation` | `rules/api-module.md` Part A |
| QA design | `test-planning` | `rules/test-scenarios.md` + `boundary-taxonomy.md` |
| BDD scenarios | `test-generation` | `rules/bdd-gherkin.md` |
| Contract tests | `test-generation` | `rules/api-contract-test.md` |
| Security triage | `security-audit` | `rules/security-triage.md` |
| Chaos/resilience | `non-functional-testing` | `rules/chaos-engineering.md` |
| Review audit | `review-audit` | `rules/blueprint-audit.md` |

Cross-links at the bottom of every module/feature/API/testing doc point back to this table.

## 8. Conventions

- **Audience:** developer documentation (HOW/WHY) — not end-user guides.
- **Language:** English throughout; code fences tagged (`rust`, `json`, `yaml`, `mermaid`, `bash`).
- **Mermaid:** every `overview.md` has 1 diagram; every feature doc has 2–3 (sequence + ERD + optional flow). Validated via `mermaid-js validate_and_render_mermaid_diagram`.
- **Traceability:** `User Story → FS (fsd.md) → FR (prd.md) → Module/Crate → API → BDD tag → QA/TC` is enumerated in `modules/manifest.md` and mirrored in each feature doc's Cross-References table.
