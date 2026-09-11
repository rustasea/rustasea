# RustaSea — Capacity & SLA Design

> **Status:** Draft — P3 (TASK-008)  
> **Date:** 2026-09-07  
> **Parents:** `brd.md` · `prd.md` (NFRs §4, Risks §7) · `fsd.md` · `architecture.md`  
> **Scope:** Capacity model, SLOs/SLIs, pooling, queue concurrency, storage quotas, AI provider budgets, degradation modes, scaling guidance for operators deploying the single-binary `tokio` app (HTTP + queue workers + scheduler) per `architecture.md §7`.
> **Lineage (P3 → P6):** Authored as a P3 draft and adopted as the design parent by the P6 blueprint (`fsd.md` §8, `tdd.md` §8; `blueprint-audit.md` D4 PASS). The "P3" label records provenance, not unfinished status.
> **Planning vs as-built:** This document records planning intent, not implementation status. Live status: [`docs/milestones.md`](../../../docs/milestones.md) — the authoritative as-built status source (TASK-003).

---

## 1. Workload Model

| Workload | Arrival | Payload | Latency Budget | Concurrency |
|----------|---------|---------|----------------|-------------|
| HTTP `GET/POST` (no DB) | 1k RPS sustained target (load test) | ~1KB JSON | p95 <50ms | 1k concurrent `tokio` tasks |
| HTTP `GET` with DB (ORM) | 500 RPS (estimated bench before M2 profiling) | 1–10KB | p95 <150ms | pool-limited |
| Queue `Job<T>` | Spiky (bulk up to 10k/batch) | 1–100KB serialized `serde_json` | worker p95 <2s per job (app-dependent) | 100 workers default |
| Cache `get`/`put` | 5k ops/s mixed | <10KB | p95 <5ms (memory) / <20ms (redis localhost) | same pool |
| Vector `whereVectorSimilarTo` | 50 RPS (embedding call dominates) | 6KB embedding (1536×f32) | p95 <300ms w/ HNSW | PG `max_connections` limited |
| AI `text` streaming | 10 concurrent streams | 1k–4k tokens streamed | token p95 <500ms inter-arrival | provider rate-limited |
| Schedule ticker | 1/min (cron eval) | — | evaluation <10ms | single task |

*Numbers are operator guidance, not guarantees — calibrate per deployment with `criterion`/`oha`/`k6` after M1/M2 profiling. M6 AI numbers are provisional until provider benchmarks.*

---

## 2. SLOs & SLIs

| # | SLI | SLO | Window | Alert | Source NFR |
|---|-----|-----|--------|-------|------------|
| S-01 | Boot to ready (listening) | p50 <2s on CI (2 vCPU, 5 providers) | per deploy | deploy fails if >5s | NFR-Per-01 |
| S-02 | HTTP p95 (no DB, `GET /health`) | <50ms at 1k RPS | 5m rolling | page if p95 >100ms 2× | NFR-Per-02 |
| S-03 | HTTP availability (5xx) | <0.1% | 30d | page if 5xx >0.5% in 5m | Operational |
| S-04 | Cache `get` p95 | <5ms (memory) / <20ms (redis) | 5m | warn if 2× target for 5m | NFR-Per-03 |
| S-05 | Queue job success rate | >99.9% (excluding poison jobs hitting `failed_jobs`) | 1h | page if success <99% | BR-05 |
| S-06 | Queue oldest-pending age | <5m under normal load | 5m | warn if >10m; page if >30m | FR-409 metrics |
| S-07 | Schedule pause/resume latency | <1s to take effect | per op | log anomaly if >5s | FR-408 |
| S-08 | Graceful shutdown drain | ≤ `shutdown_timeout` (default 10s); 0 data loss on HTTP/queue ack | per deploy | page if forced exit (code 1) | NFR-Rel-01 |

Degraded-mode SLOs: with AI providers throttled (429 from provider), HTTP+queue SLOs unchanged; AI streaming SLI suspended with `AiError::RateLimited` surfaced as 429 to caller.

---

## 3. Resource Budgets

### 3.1 Connection Pools (`deadpool` / `sqlx`)

```
config.database.pool:
  max: 20          # PgPool; covers HTTP + queue + schedule sharing one pool
  min: 2
  idle_timeout: 10m
  connect_timeout: 5s
  acquire_timeout: 3s   # -> PoolError::Timeout if exhausted; surfaced as 503

config.redis.pool (deadpool-redis):
  max: 30          # queue (BRPOP workers) + cache + Lock + schedule onOneServer
  acquire_timeout: 2s

Sizing rule: pool_max = peak_concurrency × avg_query_time / target_utilization(0.7)
  e.g., 500 RPS × 20ms avg / 0.7 = ~15 needed → 20 configured with headroom
```

### 3.2 Tokio Runtime

*One multi-threaded runtime.* Thread count = `num_cpus` (tokio default). CPU-bound `Agent`/`Tool` handlers must use `spawn_blocking` to avoid starving I/O. No separate runtime for queues — same runtime, separate `spawn` task groups per `architecture.md §7`.

### 3.3 Queue Workers

```
Default: 10 workers per queue (configurable per queue)
Max tested: 100 concurrent workers (NFR-Sca-01 bench)
Backpressure: QueueBusy { connectionName } emitted when pending > high-water-mark
Saturation signal: pendingSize / delayedSize / reservedSize + creationTimeOfOldestPendingJob
  polled via Cloud metrics (FR-409) — drive autoscaling and alerting
```

### 3.4 Storage

| Disk | Quota Guidance | Confinement |
|------|----------------|-------------|
| `local` (`tokio::fs`) | bounded by host volume; `Storage::path()` enforces `starts_with(disk_root)` | `PathTraversal` on escape; fuzz-tested `..` payloads |
| `s3`/`gcs`/`azure` (`object_store`) | provider quotas; optional `copy_back` doubles write on read-through miss | virtual `path()` only; no fs escape |

### 3.5 Vector / AI

| Resource | Quota | Note |
|----------|-------|------|
| PG `vector(1536)` HNSW | `m=16, ef_construction=64`; query `ef_search` tunable; index RAM ~ `rows × dim × 4B` + graph overhead | Provision PG with `shared_buffers` ≥ index size target |
| Embeddings API | provider `RPM`/`TPM` limits (e.g., OpenAI 3k RPM); retries with `backoff` | Circuit: `UnsupportedCapability` when provider lacks embedding |
| LLM streaming tokens | 4k context window typical; 1k-token streaming bench in SLO section | `AgentError::McpUnavailable` when `mcp` feature off |

---

## 4. Scaling Guidance

| Dimension | Scale Up | Scale Out |
|-----------|----------|-----------|
| HTTP | increase `tokio` threads (num_cpus) + `--workers` workers | horizontal replicas behind LB; stateless (AppState from config+DB) |
| Queue | increase per-queue `workers` + `deadpool-redis` max | additional binary replicas sharing same Redis/DB queue (competing consumers via BRPOP/poll) |
| Schedule | single ticker per deployment; `onOneServer` lock ensures only one replica dispatches per tick | run scheduler on exactly one replica (leader) or rely on `onOneServer` distributed `Cache::lock` |
| Cache | `moka` scales with RAM (in-proc); Redis scales with shard/cluster (note: `touch` uses `EXPIRE` — test on cluster, see risk R-04) | Redis Cluster / Sentinel |
| Vector | increase PG `max_connections` + HNSW params | read replicas for pgvector; sharded vector DB (future, post-M6) |

Graceful scaling: `SIGTERM` drain satisfies rolling deploys / K8s `preStop` with `terminationGracePeriodSeconds >= shutdown_timeout + 2s`.

---

## 5. Degradation Modes

| Degraded Condition | Behavior | Recovery |
|--------------------|----------|----------|
| PG unreachable | `PoolError::Timeout` -> HTTP 503; queue DB driver pauses polling (backoff) | probe `/ready` fails; reconnect on next `acquire` |
| Redis unreachable | cache `get` -> `StoreUnavailable`; queue Redis driver pauses; `Lock::get` fails; `onOneServer` lock unavailable -> schedule tick skips with warning | cache falls back to no-op; queue falls to `database` driver if configured |
| pgvector extension missing | migration fails with `ExtensionMissing` diagnostic incl. `hint` | operator runs `CREATE EXTENSION vector` (or managed-DB workaround per `database.md §4`) |
| AI provider 429 / 5xx | `AiError::RateLimited`/`Provider { source }` -> HTTP 429/502; streaming chunk interrupted with `event: error` | retry with `backoff`; switch provider via `Ai::provider("anthropic")` (one config change) |
| Schedule paused | ticker checks `schedule_paused` flag before dispatch; jobs suppressed; `SchedulePaused` event logged | `schedule:resume` clears flag; emits `ScheduleResumed` |
| Queue saturated (oldest pending age >30m) | S-06 page; operator triggers `queue:work` scale-up or investigates poison jobs in `failed_jobs` | `queue:retry` / dead-letter inspection |

---

## 6. Observability Hooks for Capacity

*Metrics (exposed for Prometheus / OTLP when configured):*

- `rustasea_queue_pending_size{connection,queue}` / `delayed` / `reserved` (`gauge`)
- `rustasea_queue_oldest_pending_age_seconds{connection,queue}` (`gauge`; derived from `creationTimeOfOldestPendingJob`)
- `rustasea_cache_hit_ratio{store}` / `rustasea_cache_touch_total{store}` / `rustasea_lock_contention_total`
- `rustasea_schedule_tick_total` / `rustasea_schedule_skipped_total{reason: StillRunning|Paused|OnOneServer}`
- `http_request_duration_seconds{method,path,status}` (`histogram` for S-02)
- `rustasea_db_pool_available` / `rustasea_redis_pool_available`

Health endpoints: `GET /health` (liveness, always 200 when process alive), `GET /ready` (readiness: pool `acquire` probe + `SELECT 1` + `PING` to Redis).

---

## 7. Open Items

- Load profile for `whereVectorSimilarTo` under mixed read/write must be re-benched after M2 HNSW vs IVFFLAT decision (see `database.md §4`).
- AI provider token/rate budgets require per-provider `RPM`/`TPM` config once `rustasea-ai` adapters land (M6).
- Cost model (infra $) deferred — framework is OSS MIT/Apache-2.0; operator infra is caller-owned.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
