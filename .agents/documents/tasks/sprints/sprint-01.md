# Sprint 01 — M0 Bootstrap & Core

> **Milestone:** M0 · **Window:** 2026-10-01 → 2026-12-31 · **Status:** Planned
> **Parents:** `../roadmap.md` · `prd.md` FR-000–FR-008 · `fsd.md` FS-M0-01–FS-M0-04 · `design/architecture.md`
> **Crates:** `rustasea` (umbrella), `rustasea-foundation`, `rustasea-config`, `rustasea-macros` (scaffold)

---

## 1. Goal

Bootable application skeleton with typed config, service container, provider DAG, and graceful shutdown — so every later milestone has a foundation to build on.

## 2. Scope (In / Out)

**In:**
- `foundation::Application::configure()` + provider `register` → `boot` DAG (with cycle detection)
- `Manager::extend` closure binding semantics (#20)
- Container `Bind`/`Singleton`/`Instance` + `Make<T>` (`Option<T>` for nullable-class semantics)
- Layered config `config/*.toml` (+ `.yaml`) + env overlay + `.env` via `dotenvy` + typed `serde`
- `AppState: Arc` injection (no `static mut`)
- `Runner` lifecycle (HTTP/Queue/Schedule)
- `SIGTERM`/`SIGINT` graceful drain (`tokio::signal`, configurable timeout)
- `cargo rustasea new <app>` scaffold + `config/` + `bootstrap/app.rs` + `.env.example` + workspace `Cargo.toml`
- `CHANGELOG.md` creation at tag `v0.1.0`

**Out (deferred to later sprints):**
- Routing, ORM, auth, queue/cache (M1–M4); CLI generators beyond `new` (M5); AI/broadcast (M6).

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S01-T01 | `rustasea-foundation` Application & Provider lifecycle (DAG + Runner) | FR-000, FR-003, FR-008 | FS-M0-01 | `crates/rustasea-foundation/src/{application,provider,runner}.rs` | M | `register`→`boot` fires in DAG order; cycle error `BootError::Cycle`; duplicate provider warns; `AppState` resolvable in `boot` |
| S01-T02 | Service Container `Bind`/`Singleton`/`Instance`/`Make<T>` | FR-002, FR-006 | FS-M0-02 | `crates/rustasea-foundation/src/container.rs` | M | Singleton `Arc::ptr_eq` stable; `Make::<Option<T>>` None when unbound; `Manager::extend` closure `self` resolves to manager |
| S01-T03 | Layered config loader + typed diagnostics | FR-001, FR-007 | FS-M0-03 | `crates/rustasea-config/src/{loader,error}.rs` + `config/*.toml` examples | S | `.env` wins over file; invalid TOML → `ConfigError::Parse { file, line }`; missing file non-fatal; `AppState::config::<T>()` typed |
| S01-T04 | Graceful shutdown (`SIGTERM`/`SIGINT`, drain) | FR-004 | FS-M0-04 | `crates/rustasea-foundation/src/shutdown.rs` | S | In-flight 5s request completes on `SIGTERM`; timeout → diagnostic + exit `1`; no data loss |
| S01-T05 | `cargo rustasea new <app>` scaffold & umbrella crate | FR-005 | FS-M0-03/01 | `crates/rustasea/` + `cargo xtask` `new` | M | `cargo rustasea new demo` → `cargo check` passes; `demo/bootstrap/app.rs` + `config/` + `.env.example` present |
| S01-T06 | CI gates + `CHANGELOG.md` + tag `v0.1.0` | NFR-Per-01, NFR-Usa-02, NFR-Sca-02 | — | `.github/workflows/ci.yml`, `CHANGELOG.md` | S | `cargo xtask ci` (fmt+clippy+test) green; cold boot <2s on CI 2 vCPU; `cargo check -p rustasea-foundation` no transitive ORM/AI; tag `v0.1.0` |

## 4. Dependencies

- **Upstream:** None (M0 is root).
- **Downstream:** Blocks S02 (M1), S03 (M2), S05 (M4), S06 (M5). No sprint may close before S01 is tagged — but S02/S03 may start scaffolding once `rustasea-foundation` publishes internally.

## 5. Deliverables

- Crates `rustasea`, `rustasea-foundation`, `rustasea-config` publishable (workspace `Cargo.toml` is source of truth).
- Scaffold output + `CHANGELOG.md` at `v0.1.0`.
- `architecture.md` §3 DAG still acyclic (verified by `xtask check-cycles`).

## 6. Acceptance (Sprint Done)

- [ ] `cargo run` boots, loads `config/*.toml` + `.env`, resolves bound singleton via `Make` (`Arc::ptr_eq`), shuts down gracefully on `SIGTERM`.
- [ ] `cargo check -p rustasea-foundation` succeeds without pulling `sqlx`/`async-openai` (`cargo tree --depth 1` audit).
- [ ] `cargo rustasea new demo && cargo check` passes in generated workspace.
- [ ] CI `xtask ci` green; `CHANGELOG.md` has `v0.1.0` entry; tag `v0.1.0` pushed.

## 7. Risks

- R-01 spike: container `Bind`/`Singleton` trait-object + `Arc` design must be finalized before M2 (see roadmap §7).

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
