# ADR-001 — HTTP Framework: axum over actix-web

> **Status:** Accepted  
> **Date:** 2026-09-07  
> **Deciders:** Tech Lead, Platform  
> **Milestone:** M1 (Routing & HTTP)  
> **Related:** FR-100–109 · FS-M1-01–06 · BR-02/C-01 · Laravel 13 #19/#20

## Context

Rustavel needs an HTTP layer that feels like Laravel routing (prose-like `Route::get`, groups, `resource` helpers, domain-aware precedence) while staying `tokio`-native and compatible with `sqlx`/`deadpool` pools, tower middleware (throttle/cors), and proc-macro attributes (`#[route]`, `#[middleware]`). Two mature Rust web frameworks satisfy the performance baseline: `axum` (tower-native) and `actix-web` (actor-based). A choice determines crate graph, middleware model, and ergonomics for all later milestones.

## Decision

**Use `axum` + `tower` + `tower-http` as the HTTP stack.** `actix-web` is not used.

- Runtime: `tokio` 1.x (`axum` is `tokio`-first; powers `sqlx`/`deadpool` pools and queue workers on the same executor).
- Routing: `axum::Router` with typed extractors (`Json<T>`, `Query<T>`, `Path<T>`, `State<AppState>`).
- Middleware: `tower::Layer` / `tower-http` (CORS, compression, tracing). Custom `Throttle` and `PreventRequestForgery` implemented as `tower::Layer`.
- State: `AppState: Arc<AppState>` via `axum::extract::State`, `OnceLock` for registry init — no `static mut`.

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **axum + tower** | De-facto `tokio` alignment; `tower` composes all middleware uniformly; simplest ownership (no actor `Addr`); best `sqlx`/`deadpool` integration; smaller conceptual gap from Laravel request lifecycle | Slightly lower raw bench than actix on synthetic benchmarks | **Chosen** |
| actix-web | Benchmark leader; mature; actor isolation can help heavy CPU per-request | Actor model (`Actor`/`Addr`/`Handler`) diverges from Laravel mental model; separate `actix-rt` runtime shims add glue with `tokio` pools; `tower` ecosystem gap (must bridge) | Rejected — mental-model mismatch and runtime bridging cost outweigh bench delta |
| rocket | Closest Laravel-like attribute ergonomics (`#[get("/users")]`) | Historically nightly-coupled; slower `tokio` alignment at decision time; smaller middleware ecosystem than tower | Rejected — same ergonomics achievable via `axum` + `rustavel-macros` |

## Consequences

- `rustavel-router` and `rustavel-http` depend only on `axum`/`tower`/`tower-http` + `rustavel-foundation`; no `actix` in `Cargo.lock`.
- Single `tokio` runtime covers HTTP, queue workers, scheduler ticker, and `tokio::signal` shutdown (ADR-003).
- NFR-Per-02 (`<50ms p95` at 1k RPS, no DB) is met on `axum` in bench; bench delta vs actix is not user-visible at this envelope.
- Testing uses `axum::Router` directly with `tower::ServiceExt::oneshot` in `cargo test` — no test server needed.

## Validation

- `cargo tree` on `rustavel-router` shows `axum` + `tower` only; no `actix` node.
- Domain-route precedence and `route:list` introspection covered by `bdd-scenarios.md` `@routing-validation` / `@observability-tooling`.

## References

- README Tech Stack table (HTTP row) — `axum + tower + tower-http` rationale.
- ADR-003 (tokio stack) — `axum` choice reinforces single-runtime decision.
