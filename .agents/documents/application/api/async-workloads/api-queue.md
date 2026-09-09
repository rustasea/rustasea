# API: AsyncWorkloads — Queue (dispatch, chain, batch + CLI + metrics)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §4` · `requirements/prd FR-400..402, FR-409` · `requirements/fsd FS-M4-01..03` · `requirements/tdd BC-4 QueueRegistry/Job` · `design/domain BC-4` · `design/database §2 jobs/failed_jobs/job_batches`
> **Crates:** `rustasea-queue` · `rustasea-cache` (pendingSize via Queue) · **BDD:** `@queue-routing`, `@queue`, `@queue-metrics`

## 1. Standar Global

- **Producer URL:** `POST /jobs` is an app-level dispatch endpoint if the app exposes it; most jobs are dispatched from handlers/schedule/event listeners, not direct HTTP. CLI is the primary operative surface: `queue:work`/`queue:failed`/`queue:retry`. Metrics are Rust traits: `Queue::pendingSize(...)`.
- **Content-Type:** `application/json` (job payload `serde_json` via `testing/contracts/job-payload.schema.json`).
- **Base URL:** `http://localhost:3000` for operative HTTP (schedule dispatch), CLI `cargo rustasea ...` otherwise.
- **Format Tanggal:** RFC3339 `2026-09-07T10:00:00Z` for `creationTimeOfOldestPendingJob`.

## 2. Endpoints

### 2.1 `POST /jobs` — dispatch with central routing + per-dispatch override (app-level)

- **URL:** `POST /jobs` — example app-level endpoint dispatching typed `Job<T>` via central routing.
- **Deskripsi:** Demonstrates `Queue::route::<ProcessPodcast>(connection:"redis", queue:"podcasts")?` at boot (`DuplicateRoute` on duplicate) + `ProcessPodcast{id:42}.dispatch().await?` routed without explicit `onQueue`, + `on_queue("urgent")` override, `chain`/`batch`. Drivers `sync`/`database`/`redis` are boot-selected.
- **Kontrol Akses:** `Bearer JWT` when behind `#[middleware("auth:jwt")]`.

#### Body

**Content-Type:** `application/json`

| Field | Type | Req | Desc | Example |
|-------|------|-----|------|---------|
| `job` | string enum `[ProcessPodcast]` | yes | Job type name (for typed routing) | `ProcessPodcast` |
| `data` | object | yes | `Job<T>` payload `T` (`Serialize`) | `{ "id": 42 }` |
| `connection` | string `[sync,database,redis]` | no | override `route` connection | `redis` |
| `queue` | string | no | override `route` queue | `urgent` |
| `delay` | integer seconds | no | `available_at` delay | `60` |

```json
{ "job": "ProcessPodcast", "data": { "id": 42 } }
```

With override:

```json
{ "job": "ProcessPodcast", "data": { "id": 42 }, "queue": "urgent" }
```

**Chain (stop on first failure):**

```json
{ "chain": [{ "job": "JobA" }, { "job": "JobB" }, { "job": "JobC" }] }
```

**Batch:**

```json
{ "batch": [{ "job": "Job", "data": { "id": 1 } }, { "job": "Job", "data": { "id": 2 } }] }
```

#### Response

**Sukses (202 dispatched / 200 for override demo):**

```json
{ "job_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890", "queue": "podcasts", "connection": "redis" }
```

Chained stop on failure returns failure list not HTTP error:

```json
{ "chain": { "executed": ["JobA"], "failed": "JobB", "not_executed": ["JobC"], "exception": "JobError::Exception" } }
```

**Batch (200):**

```json
{ "batch_id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1" }
```

**Error (409 DuplicateRoute — boot-time, surfaced as 500/503 if hit at dispatch):**

```json
{ "errors": [{ "status": "409", "code": "QueueError::DuplicateRoute", "title": "Duplicate queue route", "detail": "Queue::route::<ProcessPodcast> already registered for queue a; attempt for b rejected." }] }
```

**Error (422 payload validation):**

```json
{ "errors": [{ "status": "422", "code": "JobError::InvalidPayload", "title": "Invalid job payload", "detail": "data.id: required field missing" }] }
```

#### Usage

```bash
# via app endpoint
curl -s -X POST http://localhost:3000/jobs \
  -H 'Content-Type: application/json' -H 'Authorization: Bearer '"$JWT" \
  -d '{"job":"ProcessPodcast","data":{"id":42}}' | jq .

# override queue
curl -s -X POST http://localhost:3000/jobs \
  -H 'Content-Type: application/json' -H 'Authorization: Bearer '"$JWT" \
  -d '{"job":"ProcessPodcast","data":{"id":42},"queue":"urgent"}' | jq .
```

#### But primary `Queue` producer is handler-internal

```rust
Queue::route::<ProcessPodcast>("redis","podcasts")?; // boot
ProcessPodcast{ id: 42 }.dispatch().await?;                     // routed to podcasts
ProcessPodcast{ id: 42 }.dispatch().on_queue("urgent").await?;  // override
Job::chain([JobA, JobB, JobC]).dispatch().await?;                // stops on first failure
Job::batch([Job{1}, Job{2}]).dispatch().await?;                  // -> BatchId
```

### 2.2 CLI — `queue:work` / `queue:failed` / `queue:retry`

#### `cargo rustasea queue:work`

```bash
cargo rustasea queue:work --connection=redis --queue=podcasts --max-jobs=100
# implements Shutdownable — SIGTERM drains
```

- **Success:** exits `0` after draining `max-jobs` or `SIGTERM`.
- **Error:** `UnknownConnection` → non-zero with `code QueueError::UnknownConnection`.

#### `cargo rustasea queue:failed` / `queue:retry`

```bash
cargo rustasea queue:failed
# [{"id":"abc","queue":"podcasts","payload":{ ... },"exception":"JobError::Exception","failed_at":"2026-09-07T10:00:00Z"}]

cargo rustasea queue:retry abc
# -> Job a1b2c3d4... re-queued; failed_jobs entry abc cleared after retry succeeds
```

### 2.3 Queue metrics (`Queue` trait) — `pendingSize` etc.

Not HTTP wire unless the app exposes `/metrics` — documented as trait calls per `api-contracts.md §4`:

```rust
let n: usize = queue.pending_size("redis","podcasts").await?; // 42
let t: Option<DateTime<Utc>> = queue.creation_time_of_oldest_pending_job("redis","podcasts").await?; // Some(RFC3339)
queue.delayed_size("redis","podcasts").await?;
queue.reserved_size("redis","podcasts").await?;
```

- When `LLEN` unreachable (Redis down) → `QueueError::StoreUnavailable`, not panic.
- Empty queue → `creationTimeOfOldestPendingJob` returns `None`, not `1970-01-01`.

**Example metric response (if exposed via HTTP by app):**

```json
{ "connection": "redis", "queue": "podcasts", "pending": 42, "delayed": 2, "reserved": 1, "oldest": "2026-09-07T10:00:00Z" }
```

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: RustaSea Queue — dispatch + CLI + metrics
  version: 0.1.0
  description: Typed Job + Queue::route + drivers + metrics per api-contracts.md §4
servers:
  - url: http://localhost:3000
paths:
  /jobs:
    post:
      summary: Dispatch a typed job
      security:
        - bearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              oneOf:
                - type: object
                  required: [job, data]
                  properties:
                    job: { type: string, example: ProcessPodcast }
                    data: { type: object, example: { id: 42 } }
                    queue: { type: string, example: urgent }
                    connection: { type: string, enum: [sync, database, redis], example: redis }
                    delay: { type: integer, minimum: 0, example: 60 }
                - type: object
                  required: [chain]
                  properties:
                    chain: { type: array, items: { type: object } }
                - type: object
                  required: [batch]
                  properties:
                    batch: { type: array, items: { type: object } }
      responses:
        '202':
          description: Job(s) enqueued
          content:
            application/json:
              example:
                job_id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
                queue: podcasts
                connection: redis
        '409':
          description: Duplicate queue route
          content:
            application/json:
              example:
                errors:
                  - status: '409'
                    code: QueueError::DuplicateRoute
                    title: Duplicate queue route
        '422':
          description: Invalid payload
          content:
            application/json:
              example:
                errors:
                  - status: '422'
                    code: JobError::InvalidPayload
                    title: Invalid job payload
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    JobPayload:
      type: object
      properties:
        job: { type: string, example: ProcessPodcast }
        data: { type: object, example: { id: 42 } }
        queue: { type: string, example: podcasts }
        connection: { type: string, enum: [sync, database, redis] }
    BatchId:
      type: object
      properties:
        batch_id: { type: string, format: uuid, example: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1 }
    QueueMetrics:
      type: object
      properties:
        connection: { type: string, example: redis }
        queue: { type: string, example: podcasts }
        pending: { type: integer, example: 42 }
        delayed: { type: integer, example: 2 }
        reserved: { type: integer, example: 1 }
        oldest: { type: string, format: date-time, nullable: true, example: "2026-09-07T10:00:00Z" }
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 202 | — | enqueued |
| 409 | `DuplicateRoute{type_name}` | second `Queue::route::<J>` during boot |
| 422 | `InvalidPayload` | malformed `Job<T>` data |
| 500 | `StoreUnavailable` | Redis/DB down |
| 404 | `UnknownConnection` | connection name unknown |

## 5. Cross-References

- Module: [queue.md](../../modules/async-workloads/queue.md)
- Design: `api-contracts.md §4 queue`; `tdd.md BC-4 QueueRegistry`; `database.md §2 jobs/failed_jobs/job_batches`; `domain.md BC-4 Job<T>` state `Pending→Reserved→Processing→Retrying→DeadLetter`
- Testing: [test-queue-cache](../../testing/async-workloads/test-queue-cache.md) · `testing/contracts/job-payload.schema.json` · `testing/stubs/m4-queue-cache-schedule.stub.rs` · BDD `@queue-routing`, `@queue`, `@queue-metrics`

## 6. A-Gate

- [x] 202/409/422/500 examples with realistic payloads (`job_id`, `batch_id`, `DuplicateRoute`).
- [x] YAML valid OpenAPI 3.0 with `security`, param constraints (uuid, integer min).
- [x] curl valid.

## 7. Chaos Note

`docker pause redis` / `kill -KILL postgres` mid-queue is nightly chaos in [testing/async-workloads/overview.md](../../testing/async-workloads/overview.md) § Chaos — must return typed `StoreUnavailable`, not panic; jobs re-queued on retry.


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
