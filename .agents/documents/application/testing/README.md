# RustaSea — Application Testing Stubs

> **Owner:** vheins/rustasea | **Task:** TASK-010 | **Date:** 2026-09-07
> **Parents:** `.agents/documents/testing/{test-plan.md,test-cases.md,qa-design.md}` · `prd.md` · `fsd.md` · `bdd-scenarios.md`

Application-level executable stubs tracing to `test-cases.md` `TC-*` and `qa-design.md` smoke/contract/fixture specs. Each stub is a minimal `cargo test` harness that **compiles but is #[ignore]-marked** until its crate exists — the test runner discovers the stub, the harness reports `ignored`, and the trace is `cargo test -- --list` visible. Remove `#[ignore = "stub: crate not yet implemented"]` when the crate ships.

## Stub Index

| Stub | Crate(s) | Concern Coverage (DB/Service/State/UI) | FR Trace | File |
|------|----------|-----------------------------------------|----------|------|
| M0 Foundation | `rustasea-foundation`, `rustasea-config`, `rustasea` | DB: migration table · Service: container+M0-02 · State: DAG+shutdown · UI: scaffold | FR-000..008 | `stubs/m0-foundation.stub.rs` |
| M1 Router+HTTP | `rustasea-router`, `rustasea-http`, `rustasea-macros` | Service: route registration+Http client · State: domain+throttle · UI: route:list+ErrorBag+CORS | FR-100..109 | `stubs/m1-router-http.stub.rs` |
| M2 ORM | `rustasea-orm`, `rustasea-macros`(Model), `pgvector` | DB: tables+soft-delete+vector+migrations · Service: builder+upsert · State: tx+factory · UI: serde round-trip | FR-200..210 | `stubs/m2-orm.stub.rs` |
| M3 Auth+Validation | `rustasea-auth`, `rustasea-validation` | Service: guards+validator · State: JWT+CSRF · UI: middleware+ErrorBag+429 | FR-300..311 | `stubs/m3-auth-validation.stub.rs` |
| M4 Queue/Cache/Schedule/Events | `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule` | DB: jobs/failed_jobs · Service: Job+touch+metrics · State: routing+chain+pause · UI: queue:*+Cache | FR-400..410 | `stubs/m4-queue-cache-schedule.stub.rs` |
| M5 CLI+Testing | `rustasea-cli`, `rustasea-macros`, `rustasea-testing` | Service: Artisan::call+Shutdownable · State: AlreadyExists+prompt · UI: list+make:*+paginator | FR-500..509 | `stubs/m5-cli-testing.stub.rs` |
| M6 Advanced | `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai` | Service: Storage+AiProvider+toEmbeddings · State: broadcast+agent+MCP · UI: WS/SSE+JsonApi+make:agent | FR-600..612 | `stubs/m6-advanced.stub.rs` |
| Cross | Migration/Contract/Perf/Security | DB: migration round-trip · Service: bench+triage Sec-01..09 · UI: snapshots | Cross | `stubs/cross-nfr-contracts.stub.rs` |
| Harness | `rustasea-testing` support | Shared `TestCase`/`Factory`/`testcontainers` · fakes (`InMemory`/`wiremock`/`FakeRedis`) | NFR+FS-M5-04 | `stubs/harness.stub.rs` |

## Contracts

JSON-Schema fixtures and snapshot placeholders: `contracts/`.

| Contract | Schema | Snapshot |
|----------|--------|----------|
| `route:list --json` `RouteEntry` | `contracts/route-list.schema.json` | `contracts/__snapshots__/route-list.json.snap` |
| JSON:API 1.1 `ResourceDocument` | `contracts/jsonapi.schema.json` | `contracts/__snapshots__/jsonapi-user.json.snap` |
| Queue `Job<T>` payload | `contracts/job-payload.schema.json` | — |
| JWT `Claims {sub,exp}` | `contracts/jwt-claims.schema.json` | — |
| ModelInspector metadata | `contracts/model-inspector.schema.json` | `contracts/__snapshots__/model-inspector.json.snap` |

## Fixtures

| Fixture | File | Used By |
|---------|------|---------|
| Path traversal corpus (`..`, `%2e%2e`, long chains, symlink) | `fixtures/path-traversal.corpus.json` | TC-M6-05, Sec-03 |
| CSRF cross-site matrix (6 rows) | `fixtures/csrf-matrix.json` | TC-M3-04 |
| Vector dim corpus (1536↔768 etc.) | `fixtures/vector-dim.json` | TC-M2-21 |
| Allow-list deserialization corpus | `fixtures/allowlist-corpus.json` | TC-M3-06, Sec-02 |

All contract invalid ⇒ TDD: write failing test → implement crate → pass.

## How to run (before crates exist)

```bash
cargo test -p rustasea-foundation --test m0_foundation -- --include-ignored  # lists stubs
cargo test --workspace -- --list | grep 'stub:'                                # inventory
cargo insta test --accept                                                      # snapshots (once crates land)
```

Stub tests assert their own trace tags so CI verifies coverage without running the real suite. When a crate lands, the stub's `#[ignore]` is removed and its assertions become live.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
