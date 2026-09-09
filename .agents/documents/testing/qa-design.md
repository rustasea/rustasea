# RustaSea — QA Design

> **Owner:** vheins/rustasea | **Phase:** Implementation P5 | **Task:** TASK-010 | **Date:** 2026-09-07
> **Parents:** `test-plan.md` · `test-cases.md` · `bdd-scenarios.md` · `fsd.md` · `prd.md` (NFR-*)
> **Conventions:** QA scenario buckets per `test-planning/rules/test-scenarios.md` — Positive (happy paths), Negative (validation/rejection), Monkey (chaos/adversarial), Security. Boundary taxonomy per `test-planning/rules/boundary-taxonomy.md` (8 categories). Smoke suite per `test-execution/rules/smoke-test.md`.

---

## 1. QA Scenario Design

### 1.1 Scenario Buckets

Every story's EARS AC classifies to a bucket. Each bucket below lists the representative scenarios (refs to `test-cases.md` `TC-*` for executable trace).

| Bucket | Purpose | Milestones Covered | Representative Scenarios |
|--------|---------|--------------------|--------------------------|
| **Positive** | Happy path, valid inputs, nominal flows | M0..M6 all | M0 provider DAG order, M1 `resource` 7 routes, M2 `where("status","active").first()`, M3 `login→parse` JWT, M4 `Queue::route` + `touch` TTL, M5 `make:controller` + `list --json`, M6 `JsonApiResource` fieldset+include, `Storage` fallback read |
| **Negative** | Rejection, typed diagnostics, allow-lists, strictness | M0..M6 all | `BootError::Cycle`, `ConfigError::Parse`, `RouteError::Conflict`, `UpsertError::EmptyUniqueBy`, `GuardMismatch`, CSRF `UntrustedOrigin`, `SerializationError::NotAllowed`, `QueueError::DuplicateRoute`, `GeneratorError::AlreadyExists`, `RelationNotLoaded`, `PathTraversal` |
| **Monkey** | Concurrency, corruption, liveness, malformed inputs | M0/M2/M3/M4/M6 | Concurrent `transaction(select_for_update)` contenders, `chunkBy` OOM guard (10k→500), `Lock::block` contention, `schedule:pause` mid-execution, WS backpressure `Lagged`, traversal fuzz `..%2f`/`..\\`/symlink |
| **Security** | AuthZ, forgery, injection, traversal, deserialization | M3/M4/M6 concentrated | `Sec-Fetch-Site` matrix, JSON-deser allow-list, hyphenated prefix, `argon2` + constant-time verify, `Storage::path()` confinement, `X-Forwarded-For` spoof, CORS origin suffix trick |

### 1.2 Scenario Detail (executable shape: Given/When/Then per `bdd-scenarios.md`)

| Bucket | Milestone | Scenario Title | Gherkin Trace | Technique | Cases |
|--------|-----------|----------------|---------------|-----------|-------|
| Positive | M0 | DAG-ordered provider boot + layered env>file config | `@milestone-m0` Boot DAG / Layered config prefers environment | State Transition + EP | TC-M0-01/02 |
| Negative | M0 | Cycle detector + invalid config diagnostic | `@milestone-m0` Circular dependency / Invalid config diagnostic | State Transition / Error Guessing | TC-M0-03/04 |
| Positive | M1 | Resource helper 7 routes + group prefix | `@routing` Resource 7 routes / Group prefix | EP | TC-M1-01/02 |
| Negative | M1 | Duplicate named route + malformed pattern | `@routing` Duplicate name | EP | TC-M1-03 |
| Positive+Negative | M1 | Domain precedence + fallback | `@routing-validation` Tenant catch-all / Non-domain serves | Decision Table | TC-M1-04/05 |
| Positive+Negative | M1 | Route introspection JSON + binding fields | `@observability-tooling` Binding fields | Contract (snapshot) | TC-M1-06/07 |
| Positive+Negative | M1 | `Http` throw callbacks + idle timeout | `@http-client-process` throw / idle timeout | Decision Table + BVA | TC-M1-14..16 |
| Positive | M2 | Fluent filter + transactional lock | `@orm` Fluent filter / Transactional lock | EP + State Transition | TC-M2-03/04 |
| Positive | M2 | `chunkBy` 10k→20×500 + `insertOrIgnoreReturning` | `@query-builder-additions` chunked / insert-ignore | BVA | TC-M2-06/07 |
| Negative | M2 | `EmptyUniqueBy` before round-trip + MySQL DELETE JOIN | `@upsert-delete` upsert without key / MySQL delete | EP | TC-M2-09/10 |
| Positive | M2 | `serde` eager-relation round-trip | `@collection-serialization` Round-trip / Empty list | EP + Property | TC-M2-12/13, TC-PROP-01 |
| Positive | M2 | Migration+Migrations + factory reset | `@orm` Model scaffold / Factory reset | State Transition | TC-M2-14/15/17 |
| Positive+Negative | M2 | `pgvector` nearest-neighbor + extension-missing + dim mismatch | `@vector-search` Nearest-neighbor / ExtensionMissing / dim outline | Decision Table | TC-M2-19..21 |
| Positive | M3 | JWT login+parse | `@auth` login returns verifiable token | EP | TC-M3-01 |
| Negative | M3 | Guard mismatch + bad/expired/malformed credential | `@auth` Guard mismatch / failures classified | Decision Table | TC-M3-02/03 |
| Negative | M3 | CSRF origin decision table (6 rows) | `@csrf-origin` all scenarios | Decision Table | TC-M3-04 |
| Security | M3 | JSON session + allow-list + hyphen prefix | `@cache-session-hardening` JSON / NotAllowed / hyphen | EP | TC-M3-05..07 |
| Security | M3 | Rate limit per-IP + proxy spoof ignore | `@throttle` throttled / forwarded identity ignored | BVA + Error Guessing | TC-M3-12/13 |
| Positive+Negative | M4 | Typed job retry policy + central routing + override + duplicate | `@queue-routing` / `@queue` | EP + State Transition | TC-M4-01..04 |
| Positive+Negative | M4 | Chain stop + batch + failed→retry | `@queue` Chained stop / Batch / Retry | State Transition | TC-M4-05..07 |
| Positive | M4 | `touch` extends TTL + `Lock` contention + store isolation | `@cache-touch` TTL / missing / lock / source×target isolation | BVA + Decision Table | TC-M4-08..11 |
| Positive | M4 | Async listener + `dispatchAfterResponse` + field renames | `@contracts-expansion` all | State Transition + Contract | TC-M4-12..14 |
| Positive+Negative | M4 | Scheduler pause→resume+skip+onOneServer+mid-exec | `@schedule-pauseresume` / `@schedule` | State Transition | TC-M4-15..18 |
| Positive | M4 | Cloud queue metrics (42 pending + RFC3339 oldest) | `@queue-metrics` pending / oldest / states outline | BVA | TC-M4-19..21 |
| Positive | M5 | CLI `list --json` + `#[usage]` + prompts + `Artisan::call` | `@cli` list / usage / confirm / Artisan | Contract | TC-M5-01..04 |
| Positive | M5 | Generator `make:controller` + `make:model -m` + AlreadyExists | `@generators` controller/model/duplicate | Contract + BVA | TC-M5-07..09 |
| Positive | M5 | `#[tries]` + attr-shadows-trait | `@attributes` tries / shadowing | State Transition | TC-M5-11/12 |
| Positive | M5 | Isolated `testcontainers` PG + factory reset + teardown | `@testing` isolated stores / factory reset / teardown | State Transition | TC-M5-13/14/17 |
| Positive+Security | M6 | WS authorized/unauthorized + SSE | `@broadcast` authorized / unauthorized / SSE | State Transition + Contract | TC-M6-01..03 |
| Positive+Security | M6 | Read-through fallback + traversal corpus + copy-back | `@storage-readthrough` all | State Transition + Boundary | TC-M6-04..07 |
| Positive+Negative | M6 | JSON:API fieldset+include + not-loaded error + fieldset matrix | `@jsonapi` filters+includes / not-loaded / outline | Contract | TC-M6-08..10 |
| Positive | M6 | Provider switch + unsupported capability + flag gate + 12-providers | `@ai-sdk` switching / unsupported / flag / 12 | Contract + Decision Table | TC-M6-13..16 |
| Positive+Negative | M6 | Agent stream + `make:agent` + sub-agent+middleware + deferred + MCP gate | `@ai-agents` stream±WS / generator / sub-agent / deferred / MCP | State Transition | TC-M6-17..21 |
| Positive | M6 | `toEmbeddings` dim 1536 + `dropVectorIndex` + embedding providers | `@vector-search` embedding / Drop index / providers outline | EP | TC-M6-24..26 |
| Positive+Negative | Cross | Migration up→down→up idempotence | — (TC-MIG-*) | State Transition | TC-MIG-01/02 |

All rows above map to executable `TC-*` in `test-cases.md` (134 cases plus row expansions). Duplicate assertions across concerns are forbidden per 4-concern rule.

---

## 2. Boundary Analysis

Eight categories per `test-planning/rules/boundary-taxonomy.md`. Each bullet is a boundary with its sampled cases (tests MUST cover sampled cases named in parentheses).

### N — Numeric
- Tolerance: throttle `60→61` (TC-M1-08), rate-limit `3→4` (TC-M3-12), pool sizes `min/max/idle_timeout` edge (TC-M2-01).
- Overflow: `insertOrIgnoreReturning` with `u64::MAX` PK seed, `tries 255` max (attr) — ensure no truncate.
- Precision: `pgvector` cosine distance floating stability — nearest-10 ordering stable under jitter.
- BVA sets: TTL `0, 1, 60, 120, i64::MAX` seconds (TC-M4-08); `chunkBy` `1, 499, 500, 501, 10000` (TC-M2-06).

### S — String / Text
- Empty / char edge: empty `prefix("/api/v1")` normalization (TC-M1-02), handler `name=""` length validator (TC-M3-09), CSRF empty `Origin` fallback.
- Special chars: path `..%2f`, `..\\`, unicode/emoji in user names (`TC-S-01`), RTL emails (TC-S-02).
- Injection probes: `"' OR 1=1"` in `where("status", payload)` — prepared-statement bound, not string-concatenated.
- Embedding input: empty string `toEmbeddings("")` → error or zero-dim, not panic.

### C — Collection
- Empty / single / max / null elements: 0 users (TC-M2-05), 1 user (TC-M2-03), 10k users (TC-M2-06), collection with null element (factory) — `serde` round-trip of `None`-containing vec.
- Largest tables: `chunkBy` ensures no OOM at 10k; property test arbitrary 0..20 relations (TC-PROP-01).

### N — Null / Undefined
- Missing bindings: `Make::<Option<Mailer>>==None` (TC-M0-07) vs `Make::<Mailer>`→NotFound (TC-M0-08); missing migration `posts`→`MissingTable` (TC-M2-18); `touch` on missing key→`false` (TC-M4-09); empty-queue `creationTimeOfOldestPendingJob→None` (TC-M4-20).
- Null vs missing field: `Option<T>` in `#[validate]` vs omitted JSON key (TC-M3-09).

### D — Date / Time
- Epoch / leap / DST / TZ: scheduler `daily.at("00:00")` across DST boundary (skipped-tick invariant TC-M4-15 holds), JWT `exp` past vs future (TC-M3-03), queue oldest timestamp RFC3339 tz round-trip (TC-M4-20), migration timestamps FS `YYYY_MM_DD_HHMMSS` ordering (TC-MIG-01).

### C — Concurrency
- Race: two concurrent `select_for_update` (TC-M2-04), `Factory` sequence without reset (TC-M5-14), two `Make::<Counter>` Singleton concurrent `Arc::ptr_eq` (TC-M0-06), two schedulers racing `onOneServer` Lock (TC-M4-16), `Lock::block` contention (TC-M4-10).
- Deadlock/Reentrancy: `orchestrator→worker` signal-handler reentrancy — drain not invoked twice; `Cache::lock` lease expiry never stuck (NFR-Rel-03).

### E — External Dependency
- Timeout classification: connect vs total vs idle `HttpError::Timeout` kinds (TC-M1-15), Redis/DB down during queue/cache — fail `CacheError::StoreUnavailable` (TC-M4-11 env).
- 5xx vs 422 throw-policy predicate (TC-M1-14/16).
- Observability when store down: `pendingSize` returns typed error, not panic.

### B — Business Logic
- Impossible states: `BootError::Cycle` (TC-M0-04), `UpsertError::EmptyUniqueBy` without `unique_by` (TC-M2-09), `whereBinary` on non-binary column (TC-M2-08), duplicate `Queue::route` (TC-M4-04), `AlreadyExists` without `--force` (TC-M5-09), `RelationNotLoaded` without eager (TC-M6-09), `PathTraversal` across corpus (TC-M6-05), `NotAllowed` deserialization (TC-M3-06), `UnsupportedCapability` (TC-M6-14), `McUnavailable` without flag (TC-M6-21), `GuardMismatch` (TC-M3-02).
- Off-by-one: seven routes in `resource` (not 6 — `create`/`edit` present), throttling 60 inclusive (61st fails), 20 batches of 500 (not 19 for 10k).

---

## 3. Regression Suite & Prioritization

Priorities from `test-plan.md` §9.2. Regression command is scoped — **never** `cargo test --workspace` unfiltered in CI (per global tester authority: scope to changed crates + dependents; see Execution Commands table). Locally use `cargo test -p <crate> -- --test-threads=N`.

| Priority | Crate Scope | Command (representative) | When |
|----------|-------------|--------------------------|------|
| **P0** blocker | touching crate + dependents | `cargo test -p rustasea-foundation -p rustasea-auth -p rustasea-queue --test feature` | Every PR |
| **P1** major | feature crate only | `cargo test -p rustasea-router --test route_list -- --nocapture` | Every PR touching that crate |
| **P2** edge | matrix slice | `cargo test -p rustasea-orm --features pgvector,mysql --test chunk_upsert` | Nightly + PR if matrix touched |
| **P3** cosmetic | paginator/views | `cargo test -p rustasea-testing --test paginator` | Weekly |
| **P4** AI extras | `rustasea-ai` extras | `cargo test -p rustasea-ai --features all-providers --test reranking` | Weekly + pre-tag |

Bug naming follows `BUG-{ID}` taxonomy (see `test-cases.md` when a failure is triaged). Parallel strategy: unit shards by module, integration shards by driver (`postgres` | `mysql` | `sqlite` + `pgvector`), max parallelism bounded by `testcontainers` ports.

Flaky handling: `FLAKE` classification on non-deterministic fail→pass without code change; quarantine on 3rd occurrence with `DEBT-` task priority 3–4. Do **not** re-run to green.

---

## 4. Smoke Test Suite

> Per `test-execution/rules/smoke-test.md` — speed `<30s` (fast) / `<5min` (full smoke), infra connectivity first, env-aware execution, rollback triggers.

### 4.1 Fast Smoke (runs first on every PR — blocks merge)

Total budget **<30 s**, no external containers.

| # | Check | Assertion | Tool |
|---|-------|-----------|------|
| S-01 | `cargo fmt --check` + `cargo clippy -- -D warnings` | Green on staged crates | `cargo xtask check --scope <crate>` |
| S-02 | `cargo check --workspace` / scoped `cargo check -p <crate>` | Clean with standalone crate | `cargo check -p rustasea-router` (incremental-adoption probe NFR-Sca-02) |
| S-03 | `Application::configure().boot()` on in-memory config + `Str` factory smoke | Boots in `<2s` (assert `Instant::now <2s`) | `cargo test -p rustasea-foundation --test smoke_boot` |
| S-04 | Route table build + typed extractor compile probe | `Route::get("/users", [UserController,"index"])` compiles & reports `200` over `oneshot` | `cargo test -p rustasea-router --test smoke_routing -- --ignored smoke` tag |
| S-05 | `cargo insta` snapshot drift | No pending snapshots for `route:list --json` | `cargo insta test --accept` parity check |

### 4.2 Full Smoke (runs after fast smoke — `<5 min`, needs `testcontainers`)

| # | Check | Assertion | Tool |
|---|-------|-----------|------|
| FS-01 | Postgres+Redis reachable via `testcontainers` | Container health checks pass, `migrate` once | `TestCase` harness |
| FS-02 | ORM round-trip + migration | `UserFactory::create(1).whereVectorSimilarTo` smoke with SQLite, `serde` relation round-trip | `cargo test -p rustasea-orm --test smoke_orm` |
| FS-03 | Auth + CSRF | `login/parse/logout` cycle + `POST /form cross-site evil→403` | `cargo test -p rustasea-auth --test smoke_security` |
| FS-04 | Queue route + `touch` | `Queue::route::<ProcessPodcast>(queue:"podcasts")` smoke + `Cache::touch→get` TTL probe | `cargo test -p rustasea-queue -p rustasea-cache --test smoke_queue_cache` |
| FS-05 | CLI scaffold | `cargo rustasea new smoke-app --dry-run` + `route:list --json` valid | `xtask` helper |

Environment: fast smoke runs in CI containers without Docker; full smoke requires Docker socket. Nightly expands full smoke to all drivers (`postgres`+`mysql`+`sqlite`, `redis`).

### 4.3 Rollback Triggers (from `test-execution` smoke rules)

Any failure in S-01..S-05 or FS-01..05 triggers: (a) block merge, (b) defect classified `REAL_BUG|TEST_BUG|FLAKE|ENV_ERROR` per tester contract, (c) only `REAL_BUG`/`TEST_BUG` create `FIX-` via `task-write` (`phase:Testing`, priority 4–5, `suggested_skills:[backend|frontend|...]`). Environment failures (`ENV_ERROR` — missing Docker, `should_panic` without message, toolchain mismatch) are reported to orchestrator, not filed as tasks.

---

## 5. NFR Suites (Performance, Observability, Chaos)

### 5.1 Performance & Load (from `test-plan.md` §7.1 + NFR-Per-*)

| Suite | Metric | Harness | Gate |
|-------|--------|---------|------|
| `bench_boot` | Cold boot `<2 s` p50 (M0, 5 providers, CI 2 vCPU) | `criterion` benches `Application::configure().boot()` IQR, `cargo bench --bench boot` | Fail PR if p50 >2 s |
| `bench_http_p95` | `GET /users` p95 `<50 ms` (no DB, 1k RPS, 10k total, `c=100`) | `oha http://localhost:3000/users` + `ghz` | Fail if p95 >50 ms; no PR merging M1/M2 changes regressing p95 >15% without ADR |
| `bench_cache_p95` | `Cache::get` memory `<5 ms` / redis `<20 ms` p95 | `criterion` microbench + `k6` concurrent readers | Same 15% regression gate |
| `bench_check_gencode` | `cargo check` after `make:*` `<10 s` incremental | `cargo check --timings` in `target/.generated-bench` | Fail if >10 s on scaffold |

### 5.2 Observability (from `test-plan.md` §7.3 + NFR-Mai-01)

| Signal | How asserted (not just log string) | Fixture |
|--------|-------------------------------------|---------|
| `route:list --json` binding fields + middleware | `cargo insta` snapshot + JSON-Schema (`schemars`) | Snapshot file `application/testing/__snapshots__/route-list.json.snap` |
| `show:model` `ModelInspector` metadata | Structured `{attributes,relations,casts}` snapshot | Snapshot `model-inspector.json.snap` |
| Queue metrics `#8` | Values against known 42 pending + RFC3339 oldest; empty `None` | Live `TestCase` queue with seeded jobs |
| `SchedulePaused`/`ScheduleResumed` | Event spy `Vec<Event>` ordered assertion | `schedule:pause`/`resume` harness |
| Throttle `429` metric | Header `Retry-After` int + metrics counter exactly-once | `oha` burn-in before assertion |

### 5.3 Chaos & Resilience (from `test-plan.md` §7.2)

Nightly-only, not per-PR (per `test-plan.md` §9.1 ordering). Uses killable `testcontainers` + `tc` network partition stub.

| Chaos Scenario | Injected Fault | Expected | Crate |
|----------------|----------------|----------|-------|
| Redis unavailable mid-queue | `docker pause redis` / `SIGKILL` | `CacheError::StoreUnavailable`, queue jobs re-queued on retry, metrics report typed error not panic | `rustasea-queue`, `rustasea-cache` |
| Postgres lost mid-transaction | Kill PG, concurrent `transaction(select_for_update)` | Transaction `QueryError::PoolClosed` / retryable error, second transaction not left dangling | `rustasea-orm` |
| `schedule:pause` during tick | Issue `pause` while tick loop in `sleep(60s)` window | Running job completes, next tick suppressed, no duplicate `SchedulePaused` | `rustasea-schedule` |
| SSE/WebSocket lag | Bounded `mpsc(64)` with slow consumer | `Lagged` / backpressure signal, no process OOM | `rustasea-broadcast`, `rustasea-ai` streaming |
| AI streaming truncated | Drop WS mid-`event: token` stream | Client sees close frame, no partial `AiResponse` deserialized as success | `rustasea-ai` |
| Vector index dropped mid-search | `dropVectorIndex` while `whereVectorSimilarTo` query active | Query falls back to seq scan, still returns (TC-M6-25) | `rustasea-search` |

---

## 6. Contract & Migration Test Design

### 6.1 API / Payload Contracts

| Contract | Spec / Schema | How validated | Stub |
|----------|---------------|---------------|------|
| HTTP extractors `Json<T>`+`Query<T>`+`Path<T>` responses | `schemars` JSON-Schema of `T` + OpenAPI snippet via `utoipa` | Validate `422 ErrorBag` schema, unknown-field rejection | `integration/api-contract.test.rs` |
| JSON:API 1.1 document | `jsonapi` 1.1 schema | `data/included/links/meta` + media type `application/vnd.api+json` + sparse-set `fields[]` → only those attributes present | `contract/jsonapi.schema.json` |
| Queue payload `Job<T>` | `serde_json` round-trip + version tolerance | Same `T` after `to_string→from_str`; additive field with `#[serde(default)]` backwards-compat | `migration/queue-payload-compat.test.rs` |
| `route:list --json` machine output | `RouteEntry` schema + snapshot | JSON-Schema + `cargo insta` approved snapshot | `__snapshots__/route-list.json.snap` |
| Auth JWT claims | `jsonwebtoken` HS256 `Claims{sub,exp}` | `parse→claims.sub==id`, expired→`ExpiredToken` rejection | `contract/auth-claims.test.rs` |

### 6.2 Migration Tests (from `test-generation/rules/migration-test.md`)

Every migration file (`database/migrations/YYYY_MM_DD_HHMMSS_name.rs`) participating in `FS-M2-05`/`FS-M6-07` MUST have:

- **Round-trip test:** `up → assert table/column/index exists → down → assert absent → up → assert exists` (proof not just side-effect).
- **Idempotence test:** running `migrate` twice consecutively is no-op (row count / schema hash unchanged).
- **Bulk test:** `migrate:fresh --seed` followed by `get` of each seeded row's PK — confirms `Seeder::run` + migrations together.
- **Irreversible handling:** marking a migration irreversible MUST cause `down` to emit `MigrationError::Irreversible{name}` (tested).

Corpus lives in `crates/rustasea-orm/tests/migration_roundtrip.rs` (delegated to crate's own test tree — QA design asserts the harness shape; see stub `application/testing/stubs/orm-migration.ts`).

### 6.3 Property & Snapshot Tests

- Property: `TC-PROP-01` (`serde` round-trip arbitrary 0..20 relations) uses `proptest` (`arbitrary` derive for `User{posts}`).
- Snapshots: `cargo insta` for `toSql`/`toRawSql` (TC-M2-11), `route:list --json` (TC-CTR-01), ModelInspector metadata, paginator `bootstrap-3` HTML (TC-M5-15), AI JSON-Schema fixtures. Policy: snapshots reviewed `cargo insta review` before merge; `trybuild` companion for macro `#[derive(Model)]` diagnostics.

---

## 7. Test Data, Fakes, and Fixtures

| Construct | Spec | Isolation |
|-----------|------|-----------|
| `Factory::create(n)` + traits `states` + `sequences` | Per-model `Factory` impl (`definition()→T`, `sequence: AtomicU64`), `state("admin")` sub-factory | Reset `Str` sequences per test via `TestCase` hook; parallel tests not leaking |
| `TestCase` harness | `.env.testing` overlay (`dotenvy` + process env > file), isolated Postgres/Redis via `testcontainers`, `migrate` once per binary, 30 s container timeout | Distinct random ports per binary, teardown kills `rustasea-test-*` |
| Mocks/Fakes taxonomy | Prefer **fakes**: `object_store` in-mem (`InMemory`), `reqwest` `wiremock`/`httpmock`, Redis `FakeRedis`, `pgvector` fake distance for unit; use **mocks** only for `AiProvider` per-provider adapter boundary; **stubs** for `throw(predicate)` policies; **spies** for `SchedulePaused`/`QueueBusy` event capture | Fakes share the 4-concern boundary — not re-asserting real store semantics |
| `object_store` disk fixtures | Primary `s3` + fallback `local` as `InMemory` with path-confinement prefix enforcement | No real S3 required for red |

---

## 8. Traceability and Verdict

### Coverage Trace

Every story `US-M0-01..M6-07` → ≥1 row in `test-cases.md` → ≥1 QA bucket + ≥1 boundary row + ≥1 Smoke suite where applicable. Full map in `bdd-scenarios.md` §3 (BDD features) and `prd.md` §8 (Laravel 13 trace). The matrix above is exhaustive for current PRD FRs; adding any FR requires a QA row and a `test-cases.md` `TC-*` in the same commit.

### Verdict Criteria

QA verdict `PASS` requires: P0 smoke green, all P0 cases green per scoped crate, coverage gates met (`test-plan.md` §6), security triage Sec-01..09 verified (or N/A justified), no pending snapshot approvals. Verdict `BLOCKED` when env not ready (see `test-execution/rules/qa-execution.md` pre-execution checks). Verdict `FAIL` with `BUG-{ID}` when any P0 case red with `REAL_BUG` classification. Flaky quarantined tests do NOT imply PASS without a `DEBT-` task on file.

---

*Next: `application/testing/*` stubs per module (executable harnesses tracing to every row above) → `TASK-010` close.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
