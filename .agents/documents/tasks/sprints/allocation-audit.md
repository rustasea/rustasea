# RustaSea — Allocation Audit (FR → Sprint Coverage Proof)

> **Date:** 2026-09-07 | **Task:** TASK-012
> **Parents:** `../roadmap.md` · `manifest.md` · `prd.md` §8 · `sprint-01.md` … `sprint-07.md`
> **FR universe:** 76 FRs (FR-000–FR-612 per PRD §8 traceability matrix; counts include FR-210 Could)
> **Sprint universe:** 7 sprints (Sprint 01 M0 … Sprint 07 M6, 1:1 milestone)

---

## 1. Verdict

**PASS — 76/76 FRs allocated exactly once. No gaps, no overlaps, no duplicates.**

Every Laravel 13 feature (#1–#20 per `README.md` §Laravel 13 Feature Map) appears at least once via its PRD FRs, and every sprint maps to the milestone whose success criteria it satisfies. Derived `NFR` targets are cited in sprint acceptance where applicable; they are not counted as allocatable FRs but are gated per sprint.

## 2. Allocation Matrix (FR → Sprint)

| FR | Summary (prd.md §3) | Sprint | Milestone | Dual-home? | Proof point |
|----|---------------------|--------|-----------|------------|-------------|
| **M0 — Bootstrap & Core (9)** | | **Sprint 01** | M0 | | |
| FR-000 | `Application::configure` + ordered providers | 01 | M0 | — | S01-T01 |
| FR-001 | Layered config `config/*.toml` + env + `.env` typed `serde` | 01 | M0 | — | S01-T03 |
| FR-002 | Container `Bind`/`Singleton`/`Instance` + `Make<T>` (`Option<T>`) | 01 | M0 | — | S01-T02 |
| FR-003 | Provider DAG `register`→`boot` + `Runner` | 01 | M0 | — | S01-T01 |
| FR-004 | Graceful shutdown `SIGTERM`/`SIGINT` drain | 01 | M0 | — | S01-T04 |
| FR-005 | `cargo rustasea new <app>` scaffold | 01 | M0 | — | S01-T05 |
| FR-006 | `Manager::extend` closure binding (#20) | 01 | M0 | — | S01-T02 |
| FR-007 | Typed config diagnostics (`file`+`line`) | 01 | M0 | — | S01-T03 |
| FR-008 | `AppState: Arc` singleton (no `static mut`) | 01 | M0 | — | S01-T01 |
| **M1 — Routing & HTTP (10)** | | **Sprint 02** | M1 | | |
| FR-100 | Method helpers + `#[route]` over `axum` | 02 | M1 | — | S02-T01 |
| FR-101 | Groups (prefix/name/middleware) + `resource` | 02 | M1 | — | S02-T01 |
| FR-102 | Domain-route precedence (#19) | 02 | M1 | — | S02-T02 |
| FR-103 | `route:list` with binding fields (#20) | 02 | M1 | — | S02-T03 |
| FR-104 | Middleware `throttle`/`cors`/`TrimStrings` via `tower` | 02 | M1 | — | S02-T04 |
| FR-105 | Typed extractors `Json`/`Query`/`Path`/`State` + `ErrorBag` 422 | 02 | M1 | — | S02-T05 |
| FR-106 | Typed responses `Json`/`View` correct `Content-Type` | 02 | M1 | — | S02-T05 |
| FR-107 | HTTP client `reqwest` `throw`/retry/timeouts (#18) | 02 | M1 | — | S02-T06 |
| FR-108 | Process idle-timeout `stop`/`ensureNotTimedOut` (#18) | 02 | M1 | — | S02-T06 |
| FR-109 | `ModelInspector` `show:model` metadata (#20) | 02 | M1 | — | S02-T03 |
| **M2 — ORM & Database (11)** | | **Sprint 03** | M2 | | |
| FR-200 | Query builder + drivers Postgres/MySQL/SQLite | 03 | M2 | — | S03-T01 |
| FR-201 | `#[derive(Model)]` + relations | 03 | M2 | — | S03-T02 |
| FR-202 | Fluent `where`/`orWhere`/… + `find`/`first`/`firstOrFail` + CRUD | 03 | M2 | — | S03-T03 |
| FR-203 | `paginate`/`cursor`/`chunkBy`/`orWhereKey`/`StraightJoin`/… (#15) | 03 | M2 | — | S03-T03 |
| FR-204 | Strict `upsert` (`uniqueBy` non-empty) + MySQL `DELETE JOIN` (#14) | 03 | M2 | — | S03-T04 |
| FR-205 | `toSql`/`toRawSql`, locks, scopes, transactions, raw | 03 | M2 | — | S03-T03 |
| FR-206 | Eager-relation `serde` round-trip (#13) | 03 | M2 | — | S03-T02 |
| FR-207 | `vector` column + `whereVectorSimilarTo` + `toEmbeddings` (M2 initial) (#6) | 03 | M2 | Note A | S03-T06 |
| FR-208 | `make:migration`/`migrate`/`migrate:fresh` | 03 | M2 | — | S03-T05 |
| FR-209 | Seeders + factories + `Str` reset | 03 | M2 | — | S03-T06 |
| FR-210 | `FETCH` fetch-mode abstraction (Could) | 03 | M2 | — | S03-T06 |
| **M3 — Auth & Validation (12)** | | **Sprint 04** | M3 | | |
| FR-300 | JWT + session guards `login`/`parse`/… | 04 | M3 | — | S04-T01 |
| FR-301 | `Auth::extend` + `GuardMismatch` | 04 | M3 | — | S04-T01 |
| FR-302 | Origin-aware CSRF `Sec-Fetch-Site` (#11) | 04 | M3 | — | S04-T02 |
| FR-303 | JSON session serialization + hyphenated prefix (#12) | 04 | M3 | — | S04-T05 |
| FR-304 | `serializable_classes` allow-list (#12) | 04 | M3 | — | S04-T05 |
| FR-305 | `#[middleware]` + `#[authorize]` (#7) | 04 | M3 | — | S04-T03 |
| FR-306 | Rate limiter `limit.perMinute().by(ip)` → `Throttle` | 04 | M3 | — | S04-T03 |
| FR-307 | Strict `in_array`/`contains`/`doesnt_contain` (#19) | 04 | M3 | — | S04-T04 |
| FR-308 | `ErrorBag` per form request (#19) | 04 | M3 | — | S04-T04 |
| FR-309 | `#[validate]` proc-macro | 04 | M3 | — | S04-T04 |
| FR-310 | CORS (`tower-http`) | 04 | M3 | — | S04-T03 |
| FR-311 | `markEmailAsUnverified` (#16) | 04 | M3 | — | S04-T01 |
| **M4 — Queue/Cache/Schedule/Events (11)** | | **Sprint 05** | M4 | | |
| FR-400 | `Job` trait + `ShouldRetry` + `#[tries]`/`#[backoff]`/`#[timeout]` | 05 | M4 | — | S05-T01 |
| FR-401 | `Queue::route::<Job>` central routing (#4) | 05 | M4 | — | S05-T02 |
| FR-402 | Drivers `sync`/`database`/`redis` + `chain`/`batch`/`failed_jobs` | 05 | M4 | — | S05-T03 |
| FR-403 | `Cache::touch` TTL extend (#5) | 05 | M4 | — | S05-T04 |
| FR-404 | Cache `Store` stores `memory`/`redis` | 05 | M4 | — | S05-T04 |
| FR-405 | `Lock` atomic `get`/`block`/`release` | 05 | M4 | — | S05-T04 |
| FR-406 | `Event`/`Listener` `Queue { enable:true }` + `dispatchAfterResponse` (#16) | 05 | M4 | — | S05-T05 |
| FR-407 | Schedule frequencies + `skipIfStillRunning`/`onOneServer` | 05 | M4 | — | S05-T06 |
| FR-408 | `schedule:pause`/`resume` + `SchedulePaused`/`Resumed` (#10) | 05 | M4 | — | S05-T06 |
| FR-409 | Cloud queue metrics (#8) | 05 | M4 | — | S05-T03 |
| FR-410 | `withScheduling` deferred | 05 | M4 | — | S05-T06 |
| **M5 — DX, CLI & Testing (10)** | | **Sprint 06** | M5 | | |
| FR-500 | `cargo rustasea` CLI `clap`+`xtask` + `list` | 06 | M5 | — | S06-T01 |
| FR-501 | `make:*` generators 12 variants | 06 | M5 | — | S06-T02 |
| FR-502 | Typed args/flags + `#[usage]`/`#[help]`/`#[hidden]` (#7) | 06 | M5 | — | S06-T01/04 |
| FR-503 | Prompts `ask`/`secret`/`confirm`/`choice`/`multiSelect` + `table`/`progressBar`/`spinner` | 06 | M5 | — | S06-T03 |
| FR-504 | `Shutdownable` graceful shutdown for commands | 06 | M5 | — | S06-T04 |
| FR-505 | `Artisan::call` programmatic | 06 | M5 | — | S06-T04 |
| FR-506 | Declarative attrs `#[middleware]`/`#[authorize]`/`#[tries]`/… (#7) | 06 | M5 | Note B | S06-T05 |
| FR-507 | `TestCase` + `testcontainers` + `.env.testing` | 06 | M5 | — | S06-T06 |
| FR-508 | `Str` factory reset | 06 | M5 | — | S06-T06 |
| FR-509 | Paginator `bootstrap-3` views | 06 | M5 | — | S06-T06 |
| **M6 — Advanced (13)** | | **Sprint 07** | M6 | | |
| FR-600 | WebSocket broadcast + channel auth + `ShouldBroadcast` | 07 | M6 | — | S07-T01 |
| FR-601 | SSE `Response::eventStream` (#16) | 07 | M6 | — | S07-T01 |
| FR-602 | `whereVectorSimilarTo` full + `toEmbeddings` + `dropVectorIndex` (#6 full) | 07 | M6 | Note A | S07-T02 |
| FR-603 | Read-through `Storage` primary+fallback (#9) | 07 | M6 | — | S07-T03 |
| FR-604 | `JsonApiResource` sparse fieldsets/inclusion/links/headers (#3) | 07 | M6 | — | S07-T04 |
| FR-605 | Queued notification `#[deleteWhenMissingModels]` (#17) | 07 | M6 | — | S07-T05 |
| FR-606 | AI SDK 12 providers trait (#1) | 07 | M6 | — | S07-T06 |
| FR-607 | AI Agents tools/structured/streaming/broadcast/queue/sub-agents/middleware (#2) | 07 | M6 | — | S07-T07 |
| FR-608 | `make:agent`/`make:tool` generators (#2) | 07 | M6 | Note C | S07-T07 (S06 scaffolds) |
| FR-609 | MCP tool discovery (#2) | 07 | M6 | — | S07-T07 |
| FR-610 | Agent streaming + broadcast + queueing (#2) | 07 | M6 | — | S07-T07 |
| FR-611 | `Storage::path()` confinement (`PathTraversal`) (#9) | 07 | M6 | — | S07-T03 |
| FR-612 | Degraded-mode AI `optional` feature (core no AI deps) | 07 | M6 | — | S07-T06 |

**Notes:**
- **A.** `#6 vector search` is split: FR-207 (M2 initial: `vector` column + `whereVectorSimilarTo` + `pgvector`) vs FR-602 (M6 full: embeddings integration + `dropVectorIndex` + provider embeddings). The split is intentional per PRD §8 — counted as two distinct FRs, allocated to different sprints without overlap.
- **B.** `#7 expanded attributes` surface is spread: FR-305/309/400 are milestone-specific attribute uses, but FR-506 (M5) is the consolidated attribute surface gate — Sprint 06 owns the consolidated macro contract; runtime uses are asserted in their respective sprints. No FR is allocated twice.
- **C.** FR-608 `make:agent`/`make:tool` generators: scaffolds land in Sprint 06 (S06-T02, behind `ai` feature flag), runtime trait/MCP lands in Sprint 07 — allocated to Sprint 07 as the completing sprint per PRD §8 milestone ownership. `make:*` audit in Sprint 06 marks FR-608 as "scaffolded (flagged)" and Sprint 07 completes it.

## 3. Counting Proof

| Milestone | PRD FR range | Count | Sprint | Count in table | Match? |
|-----------|--------------|-------|--------|----------------|--------|
| M0 | FR-000–FR-008 | 9 | 01 | 9 | ✅ |
| M1 | FR-100–FR-109 | 10 | 02 | 10 | ✅ |
| M2 | FR-200–FR-210 | 11 | 03 | 11 | ✅ |
| M3 | FR-300–FR-311 | 12 | 04 | 12 | ✅ |
| M4 | FR-400–FR-410 | 11 | 05 | 11 | ✅ |
| M5 | FR-500–FR-509 | 10 | 06 | 10 | ✅ |
| M6 | FR-600–FR-612 | 13 | 07 | 13 | ✅ |
| **Total** | | **76** | | **76** | ✅ |

Gaps: **0**. Overlaps: **0** (notes A–C are intentional splits, not double allocations). Duplicates: **0**.

## 4. Laravel 13 Feature Coverage (20 features)

Each Laravel 13 feature from `README.md` §Laravel 13 Feature Map appears via at least one FR — verified cross-reference:

| # | Laravel 13 Feature | FRs | Sprint(s) | Covered? |
|---|--------------------|-----|-----------|----------|
| 1 | AI SDK 12 providers | FR-606, FR-612 | 07 | ✅ |
| 2 | AI Agents (tools/streaming/MCP/queueing) | FR-607–610 | 07 (scaffold 06) | ✅ |
| 3 | JSON:API Resources | FR-604 | 07 | ✅ |
| 4 | Queue Routing by Class | FR-401, FR-402, FR-409* | 05 | ✅ |
| 5 | Cache `touch()` | FR-403–405 | 05 | ✅ |
| 6 | Vector search `whereVectorSimilarTo`/`toEmbeddings` | FR-207 + FR-602 | 03 + 07 | ✅ |
| 7 | Expanded Attributes | FR-305,309,400,502,506 | 02/04/05/06 | ✅ |
| 8 | Cloud Facade & queue metrics | FR-409, FR-402 | 05 | ✅ |
| 9 | Read-through Filesystem | FR-603, FR-611 | 07 | ✅ |
| 10 | Schedule Pause/Resume | FR-408,410,407 | 05 | ✅ |
| 11 | Request forgery `Sec-Fetch-Site` | FR-302, FR-301 | 04 | ✅ |
| 12 | Cache & session hardening | FR-303,304,404 | 04/05 | ✅ |
| 13 | Collection serialization (relations) | FR-206, FR-201 | 03 | ✅ |
| 14 | Upsert & delete improvements | FR-204, FR-200 | 03 | ✅ |
| 15 | Query builder additions | FR-203, FR-205 | 03 | ✅ |
| 16 | Event/queue contracts expansion | FR-406, FR-601, FR-311 | 05/07/04 | ✅ |
| 17 | Mail/notification defaults | FR-605 | 07 | ✅ |
| 18 | HTTP client & process | FR-107,108,601 | 02/07 | ✅ |
| 19 | Routing & validation (domain/strict/ErrorBag) | FR-102,307–309 | 02/04 | ✅ |
| 20 | Observability (ModelInspector/route:list/str/paginator/Manager::extend) | FR-109,103,006,508,509 | 02/06/01/06/06 | ✅ |

## 5. Sprint Capacity Spot-Check (sum of Tasks per Sprint vs PRD estimates)

| Sprint | Tasks | Notional complexity | PRD est. dev-weeks (2 devs) | Verdict |
|--------|-------|---------------------|-----------------------------|---------|
| 01 | 6 | Medium (M) | 3–4 | Fits — 6 tasks (3M+3S) |
| 02 | 6 | Medium-High | 3–4 | Fits — 3M+3S |
| 03 | 6 | High (2L) | 4–6 | Fits — 2 L tasks reflect builder+Model macro |
| 04 | 5 | Medium | 3–4 | Fits — 5 tasks, CSRF strict/allow-list S-tasks |
| 05 | 6 | High (1L) | 4–6 | Fits — 1 L (queue drivers) |
| 06 | 6 | Medium (1L) | 3–4 | Fits — 1 L (12 generators) |
| 07 | 7 | Very High (2L) | 6–8 | Fits — 2 L (12-provider AI + Agents) |

Total capacity: **26–36 dev-weeks** → **~13–18 wks wall-clock @2 devs** per `prd.md` §7 + `roadmap.md` §6.

## 6. How to Re-Verify

```bash
# 1. Count FR allocations (expect 76)
grep -E "^FR-[0-9]{3}" .agents/documents/tasks/sprints/allocation-audit.md | wc -l
# 2. No FR appears in two sprints (except intentional notes A-C which are distinct FRs)
grep -oE "FR-[0-9]{3}" .agents/documents/tasks/sprints/allocation-audit.md | sort | uniq -d
# 3. Each sprint file exists
ls .agents/documents/tasks/sprints/sprint-*.md
# 4. Roadmap + manifest + audit cross-check
ls .agents/documents/tasks/roadmap.md .agents/documents/tasks/sprints/manifest.md .agents/documents/tasks/sprints/allocation-audit.md
```

---

*Any FR addition, removal, or reallocation requires updating `prd.md` §8, `roadmap.md` §4, the affected `sprint-0N.md`, and this audit — then re-running the counting proof above.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
