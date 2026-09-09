# ADR-003 — Async Stack: Single tokio Runtime

> **Status:** Accepted  
> **Date:** 2026-09-07  
> **Deciders:** Tech Lead, Platform  
> **Milestone:** M0 (Foundation; spans all milestones)  
> **Related:** FR-004 · FS-M0-04 · BR-08/C-01 · NFR-Per-01/NFR-Rel-01

## Context

Every RustaSea workload is async: `axum` HTTP serving, `sqlx` queries, `deadpool`/`deadpool-redis` pools, queue workers (Redis BRPOP + DB polling), `tokio-cron-scheduler` schedule ticker, WebSocket/SSE streams, `reqwest` HTTP client, and `async-openai` AI adapters. The runtime choice must unify these under one executor, support graceful shutdown (`SIGTERM` drain), and not fragment the crate graph. Rust's async runtimes are `tokio` (de-facto), `async-std`, and `smol`.

## Decision

**One `tokio` 1.x multi-threaded runtime across the entire workspace. No mixed runtimes. No facade over `async-std`/`smol`.**

- `tokio = { version = "1", features = ["full"] }` in `[workspace.dependencies]`.
- The single runtime powers: `axum` server, `sqlx`/`deadpool` pools, queue workers, scheduler ticker, WebSocket, `reqwest`, `object_store`, `async-openai`, `testcontainers`.
- Concurrency uses `tokio::spawn` / `tokio::select!` / `tokio::signal`; CPU-bound `Agent`/`Tool` handlers use `tokio::task::spawn_blocking`.
- Shutdown: `tokio::signal::unix::signal(SignalKind::terminate/interrupt)` → stop `axum::Server` accepting → drain in-flight tasks up to `shutdown_timeout` (default 10s) → stop ticker → drain queue workers (ack/nack) → flush `dispatchAfterResponse` buffer → `exit 0` (timeout → log outstanding + `exit 1`).

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **tokio single** | De-facto ecosystem (axum/sqlx/deadpool/reqwest all tokio-native); `select!` for shutdown; work-stealing scheduler; largest talent pool | Pinning contracts must be respected by `async fn` in handlers/Tools | **Chosen** |
| async-std primary | Simpler `async` syntax in some crates | Requires bridging every tokio-native crate (`axum`, `sqlx`, `deadpool-redis`) via compatibility layers | Rejected |
| Dual runtimes | Could pick best-of for niche crate | Two executors, signal/shutdown split, pool incompatibility, `cargo tree` fragmentation | Rejected |
| actix-rt | Matches if actix-web chosen | Coupled to actor model; incompatible with `axum`/`sqlx`/`deadpool` graph per ADR-001 | Rejected with actix-web |

## Consequences

- `rustasea`, `rustasea-foundation`, `rustasea-router` all `#[tokio::main]` or crate-level `async` without re-exporting a runtime shim.
- All `Store`/`Queue`/`Guard`/`AiProvider` traits are `async` via `async-trait` / native `async fn in trait` (Rust 1.75+).
- Deployment is one binary with three `tokio::spawn` task groups sharing one `PgPool`/`RedisPool` (see `architecture.md §7`).
- MSRV 1.80+ and `tokio` 1.x are linked constraints per `prd.md C-01`.
- IPC between workers and HTTP is in-process channels/mpsc, not inter-runtime messaging.

## Validation

- `cargo tree` contains exactly one async runtime (`tokio`); no `async-std` node.
- `/health` (liveness) and `/ready` (pool probes) plus `SIGTERM` drain E2E covered by `bdd-scenarios.md @foundation` / `TestCase` harness isolation.

## References

- README Tech Stack — `tokio` row: "De-facto async runtime; powers axum, sqlx, deadpool…"
- ADR-001 — axum choice depends on tokio; ADR-002 — sqlx/deadpool are tokio-native.
- `tdd.md §4–5` — budgets and deployment topology assuming single runtime.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
