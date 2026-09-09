# Skenario Pengujian: AsyncWorkloads — Queue/Cache/Events/Schedule (ASYNC)

> Skenario pengujian untuk fitur AsyncWorkloads.
> Per `qa-design §1–2`, `bdd-gherkin` + `api-contract-test` + `security-triage` + `chaos-engineering`.

## Header & Navigation

- [Module Overview](../../modules/async-workloads/overview.md)
- API specs: [queue](../../api/async-workloads/api-queue.md) · [cache](../../api/async-workloads/api-cache.md) · [events/schedule](../../api/async-workloads/api-events-schedule.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| ASYNC-POS-001 | US-M4-01 | Dispatch uses central routing | `Queue::route::<ProcessPodcast>("redis","podcasts")` | `ProcessPodcast{id:42}.dispatch()` without `onQueue` | placed on `podcasts` | High |
| ASYNC-POS-002 | US-M4-01 | Per-dispatch override `onQueue` | same `route` podcasts | `ProcessPodcast{id:42}.dispatch().on_queue("urgent")` | placed on `urgent` not `podcasts` | High |
| ASYNC-POS-003 | US-M4-01 | Retry with backoff succeeds | `#[tries(3)] #[backoff(1)] Flaky` fails 2x | `dispatch` | retried twice ~1s backoff, 3rd succeeds | High |
| ASYNC-POS-004 | US-M4-02 | Batch returns BatchId | `batch([Job{1},Job{2},Job{3}])` | dispatch | `BatchId` + 3 enqueued | High |
| ASYNC-POS-005 | US-M4-02 | Failed job retries clears entry | failed `abc` | `queue:retry abc` | re-queued + `failed_jobs` cleared after success | High |
| ASYNC-POS-006 | US-M4-03 | touch extends TTL | `Cache::put("k","v",60s)` + 30s | `touch("k",120s)` | `get("k")` after 90s from touch still `Some("v")` | High |
| ASYNC-POS-007 | US-M4-03 | Lock free path acquires | no holder `billing` | `Lock("billing",10s).get()` | `Some(LockGuard)` RAII | High |
| ASYNC-POS-008 | US-M4-04 | Async listener enqueued as Job | `Listener SendMail QUEUE=true` on `UserCreated` | `Dispatcher::dispatch(UserCreated{id:1})` | enqueued as Job not inline | High |
| ASYNC-POS-009 | US-M4-04 | dispatchAfterResponse after Response | handler defers `AnalyticsFlushed` | `GET /page` returns 200 | event dispatched after Response (spy ordered assertion) | High |
| ASYNC-POS-010 | US-M4-05 | Pause halts + emits SchedulePaused | `email:send` every minute | `schedule:pause` | `SchedulePaused` + next tick no dispatch | High |
| ASYNC-POS-011 | US-M4-05 | Resume emits + subsequent tick dispatches | paused | `schedule:resume` | `ScheduleResumed` + next tick dispatches | High |
| ASYNC-POS-012 | US-M4-05 | skipIfStillRunning suppresses | `NightlyImport` 80s with `skipIfStillRunning` | 60s tick while still running | tick skipped | High |
| ASYNC-POS-013 | US-M4-05 | onOneServer distributed lock wins once | two schedulers both `daily` with `onOneServer` | daily tick | only one acquires `Cache::lock` | High |
| ASYNC-POS-014 | US-M4-06 | Pending size 42 probes | `redis` `podcasts` 42 pending | `pending_size("redis","podcasts")` | `42` | High |
| ASYNC-POS-015 | US-M4-06 | Oldest pending RFC3339 | oldest at `2026-09-07T10:00:00Z` | `creation_time_of_oldest_pending_job` | that RFC3339 string | High |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| ASYNC-NEG-001 | US-M4-01 | Duplicate route rejected | `Queue::route::<ProcessPodcast>("a")` already | `Queue::route::<ProcessPodcast>("b")` again during boot | `DuplicateRoute{ProcessPodcast}` | High |
| ASYNC-NEG-002 | US-M4-02 | Chain stops at first failure | `chain([JobA,JobB,JobC])` where JobB fails | dispatch | `JobC` not executed; `failed_jobs` has JobB + exception | High |
| ASYNC-NEG-003 | US-M4-03 | touch missing key false not error | no entry `k` | `touch("k",60s)` | `false` (not error) | High |
| ASYNC-NEG-004 | US-M4-04 | Contract field renames invariant (JobAttempted/QueueBusy) | handlers for both | inspect fields | `JobAttempted{exception}` not `exceptionOccurred`; `QueueBusy{connectionName}` not `connection` | High |
| ASYNC-NEG-005 | US-M4-06 | Empty queue oldest is None | `podcasts` empty | `creation_time_of_oldest_pending_job` | `None` not `1970-01-01` | High |
| ASYNC-NEG-006 | US-M4-06 | Metrics pending/delayed/reserved outline | queue 10/2/1 and 0/5/0 rows | each metric requested | reported pending/delayed/reserved match (BDD outline §2.5) | Medium |
| ASYNC-NEG-007 | US-M4-05 | Pause during execution lets running finish | job executing then `pause` | `schedule:pause` while running | running job completes, next tick suppressed | Medium |
| ASYNC-NEG-008 | US-M4-03 | Cache stores isolation | `redis` put "v" under "k" | lookup in `memory` | `None` — not found | High |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| ASYNC-MNK-001 | Concurrency | duplicate `Queue::route` contenders | `OnceLock` race during boot | exactly one succeeds, other `DuplicateRoute` |
| ASYNC-MNK-002 | Redis unavailable mid-queue | `docker pause redis` / SIGKILL | queue dispatch or `pendingSize` | `CacheError::StoreUnavailable` / `StoreUnavailable` not panic — Re-queued on retry |
| ASYNC-MNK-003 | Postgres lost mid-tx | kill PG container during `transaction(select_for_update)` | concurrent tx | `PoolClosed` retryable, not dangling |
| ASYNC-MNK-004 | schedule:pause during tick `sleep(60s)` window | pause issued while ticker sleeps | tick window | running job completes, exactly-one `SchedulePaused` |
| ASYNC-MNK-005 | SSE/WebSocket lag bounded mpsc overflow | slow consumer `mpsc(64)` | broadcast/queue stream | `Lagged` backpressure signal, no OOM |
| ASYNC-MNK-006 | Vector index dropped mid-search seq-scan | `dropVectorIndex` while `whereVectorSimilarTo` active | query | fallback seq scan still returns (TC-M6-25 adjacency) |
| ASYNC-MNK-007 | StoreUnavailable metric typed | Redis/DB down + `pendingSize` | unreachable store | typed `QueueError::StoreUnavailable` not panic |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| ASYNC-SEC-001 | Worker | Job payload allow-list | enqueue `Job<T>` where `T` not in `serializable_classes` | deserialization `NotAllowed` before `serde` instantiation (RegisterSec02) |
| ASYNC-SEC-002 | Operator | `schedule:pause` without auth `Authorize` | attempt pause without privileged role | typed auth check in `api-events-schedule` §3 (bearer) |
| ASYNC-SEC-003 | Auditor | `Lock` lease-expiry deadlock liveness | `Lock::block` contender never releasing | lease expiry not stuck (NFR-Rel-03) |
| ASYNC-SEC-004 | Auditor | `X-Forwarded-For` adjacent to schedule Gated `onOneServer` | worker behind proxy spoof | `Lock` not bypassed |
| ASYNC-SEC-005 | Auditor | `touch` not uniform Redis Cluster | cluster shard fails `EXPIRE` | `EXPIRE` fallback via driver abstraction tested |


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
