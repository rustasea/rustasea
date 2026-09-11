# ADR-0007 — Facades Replaced by AppState Arc

> **Status:** Accepted
> **Date:** 2026-09-07
> **Deciders:** Tech Lead, Architecture
> **Milestone:** M0 (applies M0–M6)
> **Related:** BR-08/C-03 · FR-002/FR-008 · FS-M0-02 · architecture.md §3 · domain.md §2/BC-0

## Context

Laravel and Goravel expose global facades (`Cache::get`, `facades.*`) backed by `static mut` or container singletons resolved via global access. In Rust this would require `static mut` or `lazy_static` with interior mutability, violating thread-safety and test isolation. RustaSea's containers must support `axum::extract::State`, graceful shutdown, and per-test isolated `AppState` (parallel `testcontainers`). A decision is needed on how framework state is shared without global facades, while preserving Laravel-like ergonomics.

Constraints: no `static mut` (C-03); `AppState: Send+Sync`; per-test isolation with random ports; `OnceLock` initialization for `QueueRegistry`.

## Decision

**No global facades. Shared state is `AppState: Arc<AppState>` injected via `axum::extract::State` and provider `boot(&AppState)`. Registry singletons use `OnceLock`/`Arc`, not `static mut`.**

- `Application::boot()` returns `Arc<AppState>` containing `config`, `container`, `db`, `cache`, `queue`, etc.
- Handlers declare `State(state): State<AppState>`; providers receive `&AppState` in `boot`.
- `Manager::extend` closures are bound to the manager instance (FSD FS-M0-02) rather than a global.
- Testing: `TestCase::setup()` returns an isolated `AppState` per test binary with random `testcontainers` ports.
- Ergonomics preserved via `rustasea` umbrella re-exports and `State` extractor, not via `Cache::` global.

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **AppState Arc + State extractor (chosen)** | Thread-safe; test-isolated; `axum`-native; no `unsafe`; aligns with `tokio` ownership | Callers thread `State` through handlers (eliminated by extractor ergonomics) | **Chosen** |
| Global `static OnceLock<AppState>` facade | Familiar `Cache::get` API | Global state prevents per-test isolation; `static mut` migration risk; hard to reset between tests | Rejected — violates C-03 and test isolation |
| `thread_local!` facades | Avoids global sharing | Breaks `tokio` work-stealing (tasks migrate threads); `spawn_blocking` confusion | Rejected |
| `facades` crate with `Arc` internally | Middle ground | Still global; hides `AppState` lifecycle; complicates shutdown ownership | Rejected — same global drawback |

## Consequences

- Every handler and listener explicitly depends on `AppState`; framework never holds hidden global.
- `Container::Make<T>` resolves from `AppState::container`; `Singleton` stability is via `Arc` pointer equality per `AppState`, not process-global.
- Positive: deterministic shutdown ownership (single `Arc` count); `cargo test` parallel isolation without `#[serial]`.
- Negative: breaking change from Goravel `facades.*` import style — mitigated by documented migration and `cargo rustasea new` scaffold using `State` everywhere.
- Neutral: `OnceLock` registries (`QueueRegistry`) are per-`AppState`, initialized after `boot` — no process-wide races.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
