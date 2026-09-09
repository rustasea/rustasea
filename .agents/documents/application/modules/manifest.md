# RustaSea — Application Module Manifest

> **Status:** P8 Final — 2026-09-07 | **Task:** TASK-013 | **Audit:** D1-W01 resolved
> **Parents:** `requirements/brd.md` (BR-01..BR-09) · `requirements/prd.md` (FR-000..FR-612) · `requirements/fsd.md` (FS-M0-01..FS-M6-07) · `requirements/tdd.md` (BC-0..BC-6) · `design/architecture.md` · `design/domain.md` · `design/database.md` · `design/api-contracts.md` · `design/component-inventory.md` · `design/decisions/ADR-00*.md` · `application/testing/` · `design/capacity.md` · `application/README.md`
> **Conformance:** `technical-documentation/rules/api-module.md` Part B — manifest-driven module docs (B-FSM G1b manifest gate passed; B-Gate verified). Every `User Story → FS → FR → Module → API → Test` linkage is enumerated here; full archetypes under `application/modules/<module>/` (M0–M6) are now materialized.

## 1. Module Index

| # | Module slug | Module (PascalCase) | Crate(s) | Milestone | BR | FRs | FSD Features | BC | Stories | API Specs (in `api-contracts.md`) | Test Tags (BDD) | QA Artifacts | Output Paths | Archetype |
|---|-------------|---------------------|----------|-----------|----|-----|--------------|----|---------|-----------------------------------|-----------------|--------------|--------------|-----------|
| 1 | `foundation` | `Foundation` | `rustasea-foundation` + `rustasea-config` + `rustasea-container` (+ `rustasea` umbrella) | M0 | BR-01 | FR-000..008 | FS-M0-01..04 | BC-0 | US-M0-01..03 | api-contracts.md §1 (M0) — builder/trait contracts | @foundation, @container, @observability-tooling | stubs/m0-foundation.stub.rs | `application/modules/foundation/overview.md` + `application/modules/foundation/boot.md` | foundation |
| 2 | `http-routing` | `HttpRouting` | `rustasea-router` + `rustasea-http` + `rustasea-macros` (route/middleware) | M1 | BR-02 | FR-100..109 | FS-M1-01..06 | BC-1 | US-M1-01..03 | api-contracts.md §1 (routes, middleware, Http client, route:list) | @routing, @routing-validation, @observability-tooling, @http-client-process | contracts/route-list.schema.json + snapshots | `application/modules/http-routing/overview.md` + `.../routing.md`, `middleware.md`, `http-client.md` | http-routing |
| 3 | `data-orm` | `DataOrm` | `rustasea-orm` + `rustasea-macros` (Model) | M2 | BR-03 | FR-200..210 | FS-M2-01..06 | BC-2 | US-M2-01..03 | api-contracts.md §1 (builder) + tdd.md BC-2 | @orm, @query-builder-additions, @upsert-delete, @collection-serialization, @vector-search | fixtures/vector-dim.json, fixtures/allowlist-corpus.json, stubs/m2-orm.stub.rs | `application/modules/data-orm/overview.md` + `.../model-relations.md`, `query-builder.md`, `migrations.md`, `vector.md` | data |
| 4 | `identity-access` | `IdentityAccess` | `rustasea-auth` + `rustasea-validation` | M3 | BR-04 | FR-300..311 | FS-M3-01..06 | BC-3 | US-M3-01..03 | api-contracts.md §2 (auth, CSRF, validation) | @auth, @csrf-origin, @cache-session-hardening, @attributes, @throttle | jwt-claims.schema.json, fixtures/csrf-matrix.json, allowlist-corpus.json | `application/modules/identity-access/overview.md` + `.../auth.md`, `csrf.md`, `validation.md` | security |
| 5 | `async-workloads` | `AsyncWorkloads` | `rustasea-queue` + `rustasea-cache` + `rustasea-events` + `rustasea-schedule` | M4 | BR-05 | FR-400..410 | FS-M4-01..05 | BC-4 | US-M4-01..03 | api-contracts.md §3 (queue/cache/events/schedule) | @queue-routing, @queue, @cache-touch, @contracts-expansion, @schedule, @queue-metrics | contracts/job-payload.schema.json, stubs/m4-queue-cache-schedule.stub.rs, capacity.md | `application/modules/async-workloads/overview.md` + `.../queue.md`, `cache.md`, `events.md`, `schedule.md` | async |
| 6 | `developer-platform` | `DeveloperPlatform` | `rustasea-cli` + `rustasea-macros` + `rustasea-testing` + `xtask` | M5 | BR-06 | FR-500..509 | FS-M5-01..04 | BC-5 | US-M5-01..03 | api-contracts.md §4 (CLI) | @cli, @generators, @testing | stubs/m5-cli-testing.stub.rs, harness.stub.rs | `application/modules/developer-platform/overview.md` + `.../cli.md`, `generators.md`, `testing-harness.md` | dx |
| 7 | `intelligence-delivery` | `IntelligenceDelivery` | `rustasea-broadcast` + `rustasea-storage` + `rustasea-search` + `rustasea-ai` (+ `rustasea-jsonapi`) | M6 | BR-07 | FR-600..612 | FS-M6-01..07 | BC-6 | US-M6-01..03 | api-contracts.md §5 (broadcast/storage/jsonapi/ai) | @broadcast, @storage-readthrough, @jsonapi, @ai-sdk, @ai-agents, @vector-search | jsonapi.schema.json, path-traversal.corpus.json, model-inspector schemas | `application/modules/intelligence-delivery/overview.md` + `.../broadcast.md`, `storage.md`, `jsonapi.md`, `ai-sdk.md`, `ai-agents.md` | intelligence |

No-gap checklist: every story in `requirements/user-stories.md` appears in exactly one row; every FR 000..612 appears (prd.md §8 is truth); every FS M0-01..M6-07 appears; every milestone M0..M6 has ≥1 module; every module maps to ≥1 test @tag; slugs kebab-case; every module has a planned `overview.md`.

## 2. Conventions

- Output root: `.agents/documents/application/modules/<module>/` per `api-module.md`.
- Per-feature docs to contain: Overview, User Stories, Business Logic (pseudocode), Sequence Diagram (Mermaid), Data Model (Mermaid ERD), Public Interface, Dependencies, Limitations, Compliance, UI Layout (CLI form states where applicable), Implementation Tasks, Cross-References + skill reference table.
- Component inventory: `design/component-inventory.md`; flows: `design/flows.md`; design tokens: `design/design-system.md`; schemas: `application/testing/contracts/`; fixtures: `application/testing/fixtures/`; stubs: `application/testing/stubs/`.
- Traceability source of truth: `prd.md §8` (FR→Laravel #→BR→BDD tag), `fsd.md §5` (dependency DAG), `fsd.md §8` + `tdd.md §8` (requirements→architecture→tests). This manifest mirrors them.

## 3. Materialized Tree (P8 — D1-W01 resolved)

Verification 2026-09-07 — every row now has `overview.md` + ≥1 `feature.md` with Mermaid validated:

| Module | `overview.md` (1 diagram) | Feature docs (each 2 diagrams) | `api/<module>/api-*.md` (OpenAPI 3.0) | `testing/<module>/` |
|--------|---------------------------|--------------------------------|---------------------------------------|---------------------|
| `foundation` (3 features) | ✅ `foundation/overview.md` | ✅ `boot.md` · `container.md` · `config.md` | ✅ `api/foundation/api-bootstrap.md` (2 YAML snippets) | ✅ `testing/foundation/overview.md` + `test-boot-container.md` |
| `http-routing` (3) | ✅ `http-routing/overview.md` | ✅ `routing.md` · `middleware.md` · `http-client.md` | ✅ `api/http-routing/api-routing.md` + `api-http-client.md` | ✅ `testing/http-routing/overview.md` + `test-routing.md` |
| `data-orm` (4) | ✅ `data-orm/overview.md` | ✅ `model-relations.md` · `query-builder.md` · `migrations.md` · `vector.md` | ✅ `api/data-orm/api-query-builder.md` | ✅ `testing/data-orm/overview.md` + `test-orm.md` |
| `identity-access` (3) | ✅ `identity-access/overview.md` | ✅ `auth.md` · `csrf.md` · `validation.md` | ✅ `api/identity-access/api-auth.md` + `api-validation.md` | ✅ `testing/identity-access/overview.md` + `test-auth-validation.md` |
| `async-workloads` (4) | ✅ `async-workloads/overview.md` | ✅ `queue.md` · `cache.md` · `events.md` · `schedule.md` | ✅ `api/async-workloads/api-queue.md` + `api-cache.md` + `api-events-schedule.md` | ✅ `testing/async-workloads/overview.md` + `test-queue-cache.md` |
| `developer-platform` (3) | ✅ `developer-platform/overview.md` | ✅ `cli.md` · `generators.md` · `testing-harness.md` | ✅ `api/developer-platform/api-cli.md` | ✅ `testing/developer-platform/overview.md` + `test-cli.md` |
| `intelligence-delivery` (5) | ✅ `intelligence-delivery/overview.md` | ✅ `broadcast.md` · `storage.md` · `jsonapi.md` · `ai-sdk.md` · `ai-agents.md` | ✅ `api/intelligence-delivery/api-broadcast.md` + `api-storage.md` + `api-jsonapi.md` + `api-ai.md` | ✅ `testing/intelligence-delivery/overview.md` + `test-advanced.md` |

**Totals:** 7 `overview.md` (7 diagrams) + 25 feature docs (50 diagrams) + 14 API specs (16 OpenAPI YAML snippets) + 14 testing docs — 60 docs under `application/` including `README.md` + `api/README.md` + `testing/README.md` + `blueprint-audit.md` + `modules/manifest.md`.

## 4. Next Steps

- Sprint task decomposition (`tasks/plan/*.md`) consumes this manifest as input to sprint planning per `project-planning/planning-docs.md` S5..S9.
- MCP task tree: one task per FSD feature and per generator group, linked to module rows above.

---

> **P8 Final 2026-09-07 — TASK-013:** all 7 modules have `overview.md` + ≥1 `feature.md`; file list verified in §3. Branch: `application/` tree produced under `technical-documentation/rules/api-module.md` Part A+B. Audit gap D1-W01 (blueprint-audit.md §0/§9) closed.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
