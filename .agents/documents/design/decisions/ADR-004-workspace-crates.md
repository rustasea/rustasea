# ADR-004 — Workspace Crate Boundaries per FR Domain

> **Status:** Accepted
> **Date:** 2026-09-07
> **Deciders:** Tech Lead, Architecture
> **Milestone:** M0 (applies M0–M6)
> **Related:** BR-08/C-02/C-05 · FR-000–FR-612 · FS-M0-01–FS-M6-07 · architecture.md §3 · domain.md §2 · component-inventory.md

## Context

RustaSea must satisfy incremental adoption (BR-08, NFR-Sca-02): `cargo check -p rustasea-router` must not pull ORM, queue, or AI dependencies. A single `laravel/framework`-style monolithic crate would violate pay-for-crates-you-use and couple compilation across milestones. Conversely, over-fragmentation (one crate per FR) would inflate publish and CI cost. The bounded contexts from `domain.md` (BC-0–BC-6) and milestones M0–M6 suggest a natural crate grouping, but the boundary must be documented and enforced as a DAG with no cycles.

Constraints: workspace `resolver = "2"` with `[workspace.dependencies]` as single source of truth; each crate `cargo check`-clean standalone (C-05); proc-macros isolated to `rustasea-macros`; re-export via umbrella `rustasea`.

## Decision

**One crate per milestone domain, plus shared foundation and umbrella — 18 domain crates + `xtask` helper — with a strict DAG and umbrella re-export.**

Structure (see `architecture.md §3`):

- `rustasea` (umbrella, re-exports only, no logic)
- M0: `rustasea-foundation`, `rustasea-config`, `rustasea-container` (optional split), `rustasea-macros` (proc-macro)
- M1: `rustasea-router`, `rustasea-http`
- M2: `rustasea-orm`
- M3: `rustasea-auth`, `rustasea-validation`
- M4: `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule`
- M5: `rustasea-cli`, `rustasea-testing`
- M6: `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai`, `rustasea-jsonapi` (optional split from `rustasea-search`)

Edges are compile-time `depends on`; acyclicity validated by `xtask check-cycles` over `cargo metadata`. Cross-context communication is via domain events (`Dispatcher`) or shared kernel types from `rustasea-foundation`.

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **Per-domain crates + umbrella (chosen)** | Matches BC boundaries; incremental adoption via feature flags; clear ownership per milestone; `cargo tree` audit proves isolation | 18 crates increase publish/CI matrix | **Chosen** — balances modularity with maintainability; single workspace manifest keeps versioning coherent |
| Monolithic `rustasea` crate | Simplest publish, single version | Violates BR-08; every consumer pulls AI/pgvector/redis; compile times scale with full feature surface | Rejected — directly contradicts NFR-Sca-02 |
| One crate per FR (≈70 crates) | Maximal pay-per-feature granularity | Explosive publish overhead; dependency diamond risk; proc-macro per FR is wasteful | Rejected — fragmentation cost exceeds benefit |
| Feature-flagged single crate with modules | No crate sprawl; still feature-gated | Isolation is not enforced by Cargo — a consumer can accidentally depend on non-enabled module types; DAG not visible | Rejected — crate boundary is stronger than feature boundary |

## Consequences

- `rustasea` umbrella re-exports domain crates with matching feature flags; consumer selects `rustasea = { features = ["router","orm"] }` or individual crates.
- `cargo check -p rustasea-router` has no `sqlx`/`async-openai` in `cargo tree` — enforced in CI.
- Each domain crate owns its typed errors (`ContainerError`, `QueryError`, …) and traits; new cross-crate decisions require an ADR and an update to `architecture.md §3`.
- Negative: publish requires 18 crate versions bumped in lockstep; mitigated by `cargo xtask release --dry-run` and `release-plz`.
- Neutral: `xtask` is not a framework crate but ships in the same repo for `check-cycles` and generation tooling.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
