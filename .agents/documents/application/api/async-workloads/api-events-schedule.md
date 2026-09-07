# API: AsyncWorkloads — Events (`dispatch`/`dispatchAfterResponse`) + Schedule (`pause`/`resume`/`metrics`)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §4` · `requirements/prd FR-406..410` · `requirements/fsd FS-M4-03..05` · `requirements/tdd BC-4 Event/Dispatcher/Schedule` · `design/domain BC-4 + §4 Events E-01..04`
> **Crates:** `rustavel-events` · `rustavel-schedule` (+ `rustavel-queue` adjacency for `QUEUE=true`)

## 1. Standar Global

- **Events:** `Event` trait (`Serialize+DeserializeOwned+Send+Sync+'static`); `Listener<E>` `const QUEUE: bool` (`true` enqueues as Job). Not HTTP wire unless an app exposes `/events` push helper (e.g., `/page` handler deferred dispatch).
- **Schedule:** `Schedule::command(...).daily().cron().everyMinute().skipIfStillRunning().onOneServer()` + CLI `schedule:list`/`run`/`pause`/`resume`. Emits `SchedulePaused`/`ScheduleResumed` domain events `E-01`.
- **Metrics:** see [api-queue](./api-queue.md) §2.3 (also documented here for grouping completeness).

## 2. Endpoints

### 2.1 `Dispatcher::dispatch` / `dispatchAfterResponse` (handler-level)

- **Descriptor:** `Dispatcher::dispatch(UserCreated{ id: 1 }).await?` immediate; `dispatchAfterResponse(AnalyticsFlushed).await?` buffered until `Response` sent then flushed (observable via test spy `Vec<Event>` ordered assertion).
- **Field renames (contract):** `JobAttempted{ exception }` not `exceptionOccurred`; `QueueBusy{ connectionName }` not `connection`.

#### Response (trait — no HTTP wire)

**Sukses (QUEUE=false inline):**

```rust
Dispatcher::dispatch(UserCreated{ id: 1 }).await?; // listener inlined
```

**Sukses (QUEUE=true enqueued as Job):**

```rust
// impl Listener<UserCreated> for SendMail { const QUEUE: bool = true; }
Dispatcher::dispatch(UserCreated{ id: 1 }).await?; // → JobId in queue (observed as queue length +1)
```

**Sukses (dispatchAfterResponse):**

```rust
async fn page_handler(State(state): State<AppState>) -> impl IntoResponse {
    Dispatcher::dispatch_after_response(AnalyticsFlushed).await?;
    Json(json!({"ok": true}))
}
// client sees 200 before spy sees AnalyticsFlushed
```

### 2.2 Schedule frequencies + `schedule:run` tick

- **Descriptor:** `Schedule::command("emails:send").daily().at("08:00").skip_if_still_running().on_one_server().register();`
- **Frequencies:** `daily`/`cron("0 * * * *")`/`everyMinute()`/… (FSD FS-M4-05).

**Trait call, not HTTP wire** — exposed via CLI `schedule:list`/`run` (§2.3).

### 2.3 CLI — `schedule:list`/`run`/`pause`/`resume`

#### `schedule:list` / `schedule:run`

```bash
cargo rustavel schedule:list
# emails:send  daily at 08:00  onOneServer

cargo rustavel schedule:run        # tick every 60s; respects schedule_paused flag
# when schedule_paused=true -> no dispatch; when paused during running job -> job completes, next tick suppressed
```

#### `schedule:pause` / `schedule:resume`

```bash
cargo rustavel schedule:pause
# -> sets schedule_paused=true (cache or DB singleton row id=1); emits SchedulePaused

cargo rustavel schedule:resume
# -> clears flag; emits ScheduleResumed; subsequent tick dispatches again
```

**Response (CLI JSON when with `--json`):**

```json
{ "event": "SchedulePaused" }
```

```json
{ "event": "ScheduleResumed" }
```

**Error (already paused — idempotent resume edge):**

```json
{ "errors": [{ "status": "200", "code": "ScheduleIdle", "title": "Already paused", "detail": "pause is idempotent — no duplicate SchedulePaused emitted on next tick." }] }
```

## 3. OpenAPI 3.0 Snippet (schedule + metrics when app exposes `/schedule` + `/metrics` helpers)

```yaml
openapi: 3.0.3
info:
  title: Rustavel Schedule + Events
  version: 0.1.0
  description: Schedule pause/resume + event dispatch; inferred from api-contracts.md §4
servers:
  - url: http://localhost:3000
paths:
  /schedule/pause:
    post:
      summary: Pause schedule (idempotent)
      security:
        - bearerAuth: []
      responses:
        '200':
          description: SchedulePaused emitted
          content:
            application/json:
              example: { event: SchedulePaused, schedule_paused: true }
        '401':
          description: Unauthorized
          content:
            application/json:
              example:
                errors:
                  - status: '401'
                    code: AuthError::InvalidToken
  /schedule/resume:
    post:
      summary: Resume schedule (idempotent)
      security:
        - bearerAuth: []
      responses:
        '200':
          description: ScheduleResumed emitted
          content:
            application/json:
              example: { event: ScheduleResumed, schedule_paused: false }
  /schedule/list:
    get:
      summary: List scheduled commands
      responses:
        '200':
          description: Schedule entries
          content:
            application/json:
              example:
                - command: emails:send
                  frequency: daily
                  at: "08:00"
                  modifiers: [skipIfStillRunning, onOneServer]
  /metrics/queue:
    get:
      summary: Queue metrics (pending/delayed/reserved/oldest)
      parameters:
        - name: connection
          in: query
          required: true
          schema: { type: string, example: redis }
        - name: queue
          in: query
          required: true
          schema: { type: string, example: podcasts }
      responses:
        '200':
          description: Metrics
          content:
            application/json:
              example: { pending: 42, delayed: 2, reserved: 1, oldest: "2026-09-07T10:00:00Z" }
        '500':
          description: Store unavailable (metrics typed error not panic)
          content:
            application/json:
              example:
                errors:
                  - status: '500'
                    code: QueueError::StoreUnavailable
                    title: Store unavailable
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    QueuedEvent:
      type: object
      properties:
        event: { type: string, example: SchedulePaused }
        fields:
          type: object
          properties:
            exception: { type: string, example: "JobError::Timeout" }
            connectionName: { type: string, example: my-redis }
```

## 4. Error Catalogue

| HTTP / CLI | Typed error | When |
|------------|-------------|------|
| 200 + flag | `SchedulePaused`/`ScheduleResumed` | pause/resume idempotent |
| 500 | `QueueError::StoreUnavailable` | metrics when Redis/DB down (typed) |
| — | `JobAttempted{exception}` / `QueueBusy{connectionName}` | field-rename invariant verified via probe |

## 5. Cross-References

- Module: [events.md](../../modules/async-workloads/events.md) · [schedule.md](../../modules/async-workloads/schedule.md) · [queue.md](../../modules/async-workloads/queue.md) (metrics)
- Design: `api-contracts.md §4` · `tdd.md BC-4` · `domain.md §4 Events E-01..04` · `database.md §2 schedule_state`
- Testing: [test-queue-cache](../../testing/async-workloads/test-queue-cache.md) · BDD `@contracts-expansion`, `@schedule`, `@schedule-pauseresume`, `@queue-metrics` · `testing/stubs/m4-queue-cache-schedule.stub.rs` · `design/capacity.md §3`

## 6. A-Gate

- [x] 200/500 examples (metrics StoreUnavailable typed).
- [x] YAML valid with `security`, examples.
- [x] curl valid (schedule:pause/resume via CLI).

## 7. Chaos & Resilience Note

Per `chaos-engineering`: `schedule:pause` during tick `sleep(60s)` window — running job completes, next tick suppressed, exactly-one `SchedulePaused` emission. See [testing/async-workloads/overview.md](../../testing/async-workloads/overview.md) § Chaos.

