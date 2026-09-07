# Testing: Foundation (M0 — Bootstrap & Core)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/foundation/overview.md](../../modules/foundation/overview.md)
> **BDD:** `@foundation`, `@container`, `@observability-tooling` (adjacency) · **FSD:** FS-M0-01..04 · **FR:** FR-000..008

## 1. Scope

Covers `Application::configure()` provider DAG, `Container::Make<T>` (`Bind`/`Singleton`/`Instance`/`Option<T>`), layered config merge + `ConfigError::Parse`, and graceful shutdown — all without HTTP. Fast smoke `S-03` boot <2s gate lives here.

## 2. Executable trace

- Stubs: `testing/stubs/m0-foundation.stub.rs` (DB: migration table · Service: container+M0-02 · State: DAG+shutdown · UI: scaffold) — `#[ignore = "stub: crate not yet implemented"]`, discovered via `cargo test -- --list | grep stub:`.
- QA rows: `requirements/testing/qa-design.md §1.2` M0 positive/negative rows → `TC-M0-01..08`.

## 3. Links

Specs: [test-boot-container.md](test-boot-container.md) · Module feature docs: [boot](../../modules/foundation/boot.md) · [container](../../modules/foundation/container.md) · [config](../../modules/foundation/config.md)
