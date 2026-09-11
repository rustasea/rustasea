# RustaSea — Sprint Manifest

> **Date:** 2026-09-07 | **Horizon:** Q4 2026 → Q4 2027 (7 sprints = 7 milestones)
> **Parents:** `../roadmap.md` + `prd.md` + `fsd.md` + `design/architecture.md`
> **Task:** TASK-012

---

## 1. Sprint Index

| Sprint | Milestone | Goal (one-liner) | Window | FRs | Crates | Status |
|--------|-----------|------------------|--------|-----|--------|--------|
| **Sprint 01** | **M0** Bootstrap & Core | Bootable skeleton with config, container, providers, shutdown. | 2026-10-01 → 2026-12-31 | FR-000–FR-008 (9) | `rustasea`, `rustasea-foundation`, `rustasea-config`, `rustasea-macros` (scaffold) | Planned |
| **Sprint 02** | **M1** Routing & HTTP | Expressive HTTP layer with routing, middleware, introspection. | 2026-11-15 → 2027-02-15 | FR-100–FR-109 (10) | `rustasea-router`, `rustasea-http` | Planned |
| **Sprint 03** | **M2** ORM & Database | Fluent type-safe DB layer with migrations/factories/vector. | 2027-01-01 → 2027-03-31 | FR-200–FR-210 (11) | `rustasea-orm`, `rustasea-macros` (Model) | Planned |
| **Sprint 04** | **M3** Auth, Middleware & Validation | Hardened auth/validation at Laravel 13 defaults. | 2027-02-15 → 2027-05-15 | FR-300–FR-311 (12) | `rustasea-auth`, `rustasea-validation` | Planned |
| **Sprint 05** | **M4** Queue, Cache, Scheduling & Events | Observable async workloads. | 2027-04-01 → 2027-06-30 | FR-400–FR-410 (11) | `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule` | Planned |
| **Sprint 06** | **M5** DX, CLI & Testing | Laravel-like DX loop: CLI, generators, testing. | 2027-05-15 → 2027-08-31 | FR-500–FR-509 (10) | `rustasea-cli`, `rustasea-macros`, `rustasea-testing` | Planned |
| **Sprint 07** | **M6** Advanced (AI/Broadcast/FS/JSON:API) | Differentiate: AI SDK, broadcast/SSE, storage, JSON:API. | 2027-07-01 → 2027-12-31 | FR-600–FR-612 (13) | `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai` | Planned |

**Total FRs:** 76 (M0 9 + M1 10 + M2 11 + M3 12 + M4 11 + M5 10 + M6 13) per `prd.md` §8.

## 2. Derivation Rationale

- **N = 7** (recommended 6–7, one sprint per milestone) — 1:1 mapping keeps milestone success criteria intact as sprint acceptance; avoids splitting a milestone across sprints which would require partial success criteria.
- Each sprint is sized to its milestone's dev-week estimate (see `../roadmap.md` §6); M6 as a single sprint is the largest (13 FRs, 4 crates, 12-provider trait) — acceptable because its `rustasea-ai` crate is feature-flagged `optional` and can incrementally land provider adapters behind flags without blocking the sprint's core deliverable (Storage/JSON:API/Broadcast).
- **Overlap windows** mirror `README.md` Roadmap: M1 starts 6 weeks into M0, M3 starts 6 weeks into M2, etc. Sprints therefore overlap in calendar time but are dependency-ordered: Sprint N+1 may start scaffolding once its dependency's `foundation` sub-crate is stable, but cannot **close** before its dependencies are tagged.

## 3. Capacity Assumptions (per roadmap §6)

- Team: **2 developers**, Rust 1.88+ stable, edition 2021, `tokio` everywhere.
- Wall-clock: **~13–18 weeks** at 2 devs (26–36 dev-weeks) +20% contingency (30–40% on M6).
- Sprint velocity is calibrated after Sprint 01; `NFR-Per-04` (`cargo check` after `make:*` <10s incremental) and `NFR-Per-01` (cold boot <2s) are CI gates each sprint.

## 4. Sprint Boundaries & Dependencies

```
S01 (M0) ─┬─► S02 (M1) ─┬─► S04 (M3) ─┬─► S05 (M4) ─► S06 (M5) ─► S07 (M6)
          │             │             │      ▲               ▲
          │             └─► S03 (M2) ─┘      │               │
          └──────────────────► S03 ──────────┘               │
                         (vector M2 initial)                  │
                                   M6 full vector ────────────┘
```

- **Gate per sprint:** All deliverables present, acceptance criteria green, downstream dependency unblocked. See each `sprint-0N.md` §Acceptance.
- **Global gate GF:** Blocked if any `sprint-0N.md` missing or FR coverage gap (proven in `allocation-audit.md`).

## 5. File Map (Outputs of TASK-012)

| File | Content | Guard |
|------|---------|-------|
| `../roadmap.md` | Milestone timeline, Gantt, release strategy | M0–M6 sequenced |
| `manifest.md` | This file — sprint index + derivation | 7 sprints indexed |
| `sprint-01.md` | M0 bootstrap & core | FR-000–FR-008 |
| `sprint-02.md` | M1 routing & HTTP | FR-100–FR-109 |
| `sprint-03.md` | M2 ORM & database | FR-200–FR-210 |
| `sprint-04.md` | M3 auth & validation | FR-300–FR-311 |
| `sprint-05.md` | M4 queue/cache/schedule/events | FR-400–FR-410 |
| `sprint-06.md` | M5 CLI & testing | FR-500–FR-509 |
| `sprint-07.md` | M6 advanced | FR-600–FR-612 |
| `allocation-audit.md` | FR → sprint allocation proof, gaps/overlaps/duplicates check | 76/76 allocated |

## 6. How to Use This Manifest

1. **Sprint kickoff:** Read the sprint file's Goal + Scope + Tasks + Dependencies — DoR is that milestone's FRs + FSD specs are reviewed.
2. **Daily:** Track Tasks table acceptance column; update status per skill rules (`rules/technical-planning.md` GF/V1).
3. **Sprint review:** Run Acceptance section as a checklist; tag the milestone version (`v0.1.0` … `v0.7.0`) only when all acceptance items are green.
4. **Retro input:** Allocation audit flags any scope drift that entered the sprint without an FR.

---

*Next: `sprint-01.md` through `sprint-07.md` for task-level detail.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
