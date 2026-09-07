# Testing: AsyncWorkloads (M4 — Queue, Cache, Events, Schedule)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/async-workloads/overview.md](../../modules/async-workloads/overview.md)
> **BDD:** `@queue-routing`, `@queue`, `@cache-touch`, `@contracts-expansion`, `@schedule`, `@schedule-pauseresume`, `@queue-metrics` · **FSD:** FS-M4-01..05

## 1. Scope

Covers typed `Job<T>` + `Queue::route` `OnceLock` + drivers `sync`/`database`/`redis`, `chain`/`batch`/failed→retry, `Cache` `touch`+`Lock`+store isolation, `Event`/`Listener` `QUEUE=true` + `dispatchAfterResponse` + `JobAttempted{exception}`/`QueueBusy{connectionName}` renames, scheduler frequencies + `skipIfStillRunning`/`onOneServer` (Lock) + `schedule:pause`/`resume` idempotence, and Cloud metrics `pendingSize`/`oldest`.

## 2. Trace

- Stubs: `testing/stubs/m4-queue-cache-schedule.stub.rs` · `testing/contracts/job-payload.schema.json`.
- BDD: `bdd-scenarios.md §2.5` (6 Features).
- DB: `database.md §2 jobs/failed_jobs/job_batches/cache/schedule_state`; `api-contracts.md §4`.
- QA: `qa-design §1.2` M4 positive+negative+decision/boundary.

## 3. Links

- Specs: [test-queue-cache.md](test-queue-cache.md)
- API: [api-queue](../../api/async-workloads/api-queue.md) · [api-cache](../../api/async-workloads/api-cache.md) · [api-events-schedule](../../api/async-workloads/api-events-schedule.md)
- Module: [queue](../../modules/async-workloads/queue.md) · [cache](../../modules/async-workloads/cache.md) · [events](../../modules/async-workloads/events.md) · [schedule](../../modules/async-workloads/schedule.md)
