# Sprint 05 — M4 Queue, Cache, Scheduling & Events

> **Milestone:** M4 · **Window:** 2027-04-01 → 2027-06-30 · **Status:** Planned
> **Parents:** `../roadmap.md` · `prd.md` FR-400–FR-410 · `fsd.md` FS-M4-01–FS-M4-06 · `design/architecture.md` + `design/database.md`
> **Depends On:** M0 (S01), M2 (S03), M3 (S04)
> **Crates:** `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule`

---

## 1. Goal

Async workloads, caching, scheduling, and event dispatch with observable queue metrics — so jobs, caches, and schedules are typed, routable, and measurable.

## 2. Scope (In / Out)

**In:**
- Typed `Job` trait (`handle(self)`) + `ShouldRetry`/`ShouldRetryUntil` + declarative `#[tries]`/`#[backoff]`/`#[timeout]`/`#[failOnTimeout]`
- Central queue routing `Queue::route::<Job>(connection:, queue:)` registry (`OnceLock` post-`boot`) + per-dispatch `onQueue`/`onConnection` override
- Drivers `sync` + `database` + `redis` (`deadpool-redis`) + `dispatch`/`dispatchSync`/`chain`/`delay`/`onQueue`/`onConnection` + batch + `queue:failed`/`queue:retry` + `failed_jobs`/`job_batches` tables
- Cache `Store` + `Repository` traits + `get`/`put`/`add`/`remember`/`forever`/`forget`/`flush`/`increment`/`decrement`/`pull`/`has` + `touch()` (extend TTL without `get`/`set`) + `withContext`/`store(name)`; stores `memory` (`moka`) + `redis`; `Lock` atomic `get`/`block`/`release`
- `Event` trait + `Listener` with `Queue { enable: true }` async + `dispatch` + `dispatchAfterResponse` + `JobAttempted { exception }` / `QueueBusy { connectionName }` renames
- Schedule `schedule:list`/`schedule:run` + frequencies `daily`/`cron`/`everyMinute`/`skipIfStillRunning`/`onOneServer` + `schedule:pause`/`resume` + `SchedulePaused`/`ScheduleResumed` events + `withScheduling` deferred semantics
- Cloud queue metrics `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` via `Queue` trait
- `app/jobs/`, `app/events/`, `app/listeners/`

**Out:**
- CLI `make:job`/`make:event`/`make:listener` wiring — M5 (Sprint 06) provides the CLI surface; S05 provides the runtime traits and drivers. AI/broadcast — M6.

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S05-T01 | `Job` trait + retry contracts + `#[tries]`/`#[backoff]`/`#[timeout]` | FR-400, FR-506 (M4 attrs) | FS-M4-01 | `crates/rustasea-queue/src/{job,retry,macros}.rs` + `rustasea-macros` attrs | M | `#[tries(3)] #[backoff(10)]` job fails twice → retried with 10s backoff, 3rd → `failed_jobs`; `ShouldRetryUntil` date respected; `#[failOnTimeout]` forces failure |
| S05-T02 | Queue routing `Queue::route::<Job>` registry + `onQueue`/`onConnection` | FR-401, FR-409 (metrics trait shape) | FS-M4-02 | `crates/rustasea-queue/src/route.rs` | S | `Queue::route::<ProcessPodcast>(queue:"podcasts")` → `dispatch(payload)` lands on `podcasts` without explicit `onQueue`; duplicate `route` → error; registry is `OnceLock` post-`boot` |
| S05-T03 | Queue drivers `sync`/`database`/`redis` + `chain`/`batch`/`failed_jobs` | FR-402, FR-409 | FS-M4-02/03 | `crates/rustasea-queue/src/drivers/{sync,database,redis}.rs` + `failed_jobs` table migration | L | `sync` executes inline; `database`/`redis` enqueue + worker dequeues; `chain [A,B,C]` where B fails → C not run + failure recorded; `batch` dispatch + `queue:failed`/`queue:retry`; `pendingSize` returns depth; `job-payload.schema.json` validated |
| S05-T04 | Cache `Store`/`Repository` + `touch()` + `memory`/`redis` + `Lock` | FR-403, FR-404, FR-405 | FS-M4-04 | `crates/rustasea-cache/src/{store,repository,lock,memory,redis}.rs` | M | `put("k","v",60s)` → `touch("k",120s)` extends TTL without `get`; stores isolated (`redis` vs `memory`); `Lock("billing")` contention: holder blocks waiter; `Lock::block(5s)` timeout respected; `CacheTouchFailed` only on store errors |
| S05-T05 | Events (`Event`/`Listener` + `Queue { enable: true }` + `dispatchAfterResponse` + renames) | FR-406, FR-010 deferred (withScheduling) | FS-M4-05 | `crates/rustasea-events/src/{event,listener,dispatcher}.rs` | M | `Listener { queue: Queue { enable: true } }` enqueued as job not inline; `dispatchAfterResponse` fires after HTTP response flush; `JobAttempted { exception }` and `QueueBusy { connectionName }` field renames asserted |
| S05-T06 | Schedule (`schedule:run`/`list`/`pause`/`resume` + frequencies + `onOneServer` + deferred `withScheduling`) | FR-407, FR-408, FR-410 | FS-M4-06 | `crates/rustasea-schedule/src/{schedule,runner,pause}.rs` + `schedule_state` table/Redis key | M | `everyMinute` + `skipIfStillRunning` skips when previous active; `onOneServer` distributed lock via Redis `SET NX`; `schedule:pause` halts ticker + emits `SchedulePaused`; `withScheduling` deferred until first tick (not at `boot`); `schedule:list` shows next run |

## 4. Dependencies

- **Upstream:** S01 (`Application`/`Runner`/`Shutdown`), S03 (`jobs`/`failed_jobs`/`cache` tables), S04 (cache/session hardening, auth context for queue workers). *Note:* M4 depends on M0+M2+M3 per `README.md`; S05 therefore cannot close before S04 tags `v0.4.0`.
- **Downstream:** Blocks S06 (M5 — `make:job` etc. need runtime) and S07 (M6 — notification queueing `#[deleteWhenMissingModels]`).

## 5. Deliverables

- Crates `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule`.
- Tables `jobs`, `failed_jobs`, `job_batches`, `cache`, `schedule_state`.
- Tag `v0.5.0`; `failed_jobs` schema frozen; `Store::touch` default impl documented (`Unsupported` fallback for non-Redis stores).

## 6. Acceptance (Sprint Done)

- [ ] Typed job routes to `Queue::route` queue; chain/batch + `failed_jobs` + `queue:retry` green.
- [ ] `Cache::touch` extends TTL without `get`/`set`; `Lock::block` respects timeout; NFR-Rel-03 (lock liveness) + NFR-Per-03 (cache p95 <5ms memory / <20ms redis) green.
- [ ] `Listener` with `Queue { enable: true }` enqueues; `dispatchAfterResponse` fires after response.
- [ ] `schedule:pause` halts ticker + emits `SchedulePaused`; `onOneServer` prevents duplicate run across workers.
- [ ] Cloud metrics `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` return correct RFC3339/depth; NFR-Sca-01 (100 workers + 1k HTTP concurrency) bench passes.
- [ ] `xtask check-cycles` still acyclic; `cargo check -p rustasea-cache` does not pull `async-openai`.

## 7. Risks

- R-04 Redis Cluster `touch` diverges — driver abstracts `EXPIRE` fallback; matrix tests standalone + cluster.
- R-05 Wrong-queue delivery — `OnceLock` registry + duplicate-route error + E2E per-job test.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
