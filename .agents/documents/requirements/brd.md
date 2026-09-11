# RustaSea — Business Requirements Document (BRD)

> **Status:** Final — P6 Planning Docs Finalization (verified from P2; traceability appended)
> **Date:** 2026-09-07 · **Finalized:** 2026-09-07  
> **Source research:** `docs/laravel-13-research.md` (Laravel 13.0.0, 2026-03-17) + `README.md` + Goravel v1.18 reference  
> **Milestone coverage:** M0 – M6 (dependency-ordered, no circular deps)  
> **Laravel 13 parity:** 20 headline features traced → requirements → milestones
> **Planning vs as-built:** This document records planning intent, not implementation status. Live status: [`docs/milestones.md`](../../../docs/milestones.md) — the authoritative as-built status source (TASK-003).

---

## 1. Executive Summary

RustaSea is a Rust framework that delivers Laravel-grade developer ergonomics — expressive routing, Eloquent-fluent ORM, Artisan-style code generation, and batteries-included DX — while leveraging Rust's ownership model, compile-time safety, and `tokio`-native concurrency for production-grade performance.

**Core thesis:** _Laravel proves ergonomics and velocity win hearts; Rust proves safety and performance win production. RustaSea proves you can have both._

**Business problem:** Teams that outgrow PHP/Ruby/Node on throughput, correctness, or concurrency face a productivity cliff when moving to Rust or Go: idiomatic Rust frameworks (axum, actix) are minimal and unopinionated; Go ports (Goravel) still leak `any`/`interface{}` and global facades. No Rust framework offers Laravel parity — routing ergonomics, typed Eloquent, Artisan CLI, queue/schedule/event orchestration, and AI-native primitives — behind a single `rustasea` umbrella crate.

**Why now:** Laravel 13 (2026-03-17) shipped AI SDK + vector search + expanded attributes + queue routing — the innovation frontier is now AI-native + vector + declarative DX. Rust (1.88+, edition 2021) + `tokio` + `axum` + `sqlx`/`pgvector` have stabilized enough to port these idioms natively. Goravel v1.18 proves the Laravel→compiled-language thesis commercially.

---

## 2. Business Goals and Success Metrics

| # | Business Goal | Metric (Measurable) | Target | Source / Rationale |
|---|---------------|----------------------|--------|---------------------|
| BG-01 | Ship M0 bootable skeleton with config + container + providers | `cargo run` cold-boot → ready in <2s on CI runner; `cargo test` pass rate 100% on M0 suite | M0 release tagged, docs published | Validates thesis foundation; blocks all later milestones |
| BG-02 | Achieve Laravel 13 routing + HTTP ergonomics parity (M1) | Route definition DX survey (internal dog-food): ≥4/5 Likert for "feels like Laravel" among 5+ Rust developers; `route:list` introspection 100% coverage | M1 tagged | Ergonomics is the differentiator vs axum/actix |
| BG-03 | Ship type-safe ORM with vector search from day one (M2) | `whereVectorSimilarTo` integration test (pgvector) passing; collection `serde` round-trip preserves eager relations; `cargo rustasea make:model` generates `rustfmt`-clean code | M2 tagged | #6 + #13 + #15 Laravel features; AI workloads require vectors on day one |
| BG-04 | Harden security defaults at parity with Laravel 13 (M3) | CSRF `Sec-Fetch-Site` rejection test; session JSON serialization default; guard-mismatch typed error; strict validation suite green | M3 tagged | #11 + #12 + #19 improved features |
| BG-05 | Deliver async workloads at production operability (M4) | Queue routed job execution E2E; `Cache::touch` TTL extension; `schedule:pause`/`resume` event emission; queue metrics observable via trait | M4 tagged | #4 + #5 + #8 + #10 + #16 |
| BG-06 | Achieve Laravel-like DX loop (M5) — generate, test, inspect | `cargo rustasea make:*` for 10 generators; `cargo test` isolated PG via testcontainers; `ModelInspector` / route introspection parity (#20) | M5 tagged | #7 + #20 attributes/observability |
| BG-07 | Differentiate on AI-native + real-time + JSON:API completeness (M6) | Agent+Tool streaming over WebSocket E2E; 12-provider trait compilable; `JsonApiResource` sparse fieldset conformance; read-through Storage with path confinement | M6 tagged | #1 + #2 + #3 + #9 + #6/full + SSE |
| BG-08 | Prove incremental adoption (pay-for-crates-you-use) | Workspace example builds with only `rustasea-router` (no ORM/queue) and passes `cargo check` | From M1 onward | Design principle: zero-cost ergonomics |
| BG-09 | Establish contribution funnel | ≥10 external stars + ≥3 RFC discussions within 30d of M0 tag | M0+30d | Community flywheel for multi-crate workspace |

**Non-goals (explicit exclusions from BRD scope):**

- Full Blade compatibility engine (only `askama`/`minijinja` templating via resources/views).
- Hosting PaaS / deployment orchestration (unlike Laravel Cloud/Fly — RustaSea is a framework, not a platform).
- GUI admin panel (analogous to Nova/Filament) — deferred post-M6.
- PHP interop / transpilation layer.

---

## 3. Stakeholders

| # | Stakeholder | Role | Interest / Need | Influence | Requirement Source Tag |
|---|-------------|------|-----------------|-----------|------------------------|
| SH-01 | Rust backend team lead (primary persona) | Decision maker / Evaluator | Wants Laravel velocity without runtime `any`, `unwrap` panics, or GC pauses; needs hiring-friendly DX for ex-Laravel/Rails developers | High | VP Eng quote analogue |
| SH-02 | Individual Rust developer (ex-Laravel/Go, polyglot) | End user / Builder | Wants `Route::get` prose routes, `#[derive(Model)]`, `cargo rustasea make:*`, typed jobs/events without boilerplate | High | Contributor interviews (Goravel community) |
| SH-03 | Platform / SRE engineer | Operator | Needs graceful shutdown, observable queue depth/age metrics (#8), health checks, minimal runtime, container-friendly config | Medium | DevOps requirements |
| SH-04 | AI application builder | Differentiator user | Needs provider-agnostic AI trait, agentic tools, pgvector `whereVectorSimilarTo`, SSE streaming, MCP | Medium-High | Laravel 13 AI SDK headline (#1/#2) |
| SH-05 | Goravel maintainers / prior-art community | Reference / potential contributor | Proves Laravel→compiled thesis; API familiarity expectations (facades→AppState, Register→Boot, etc.) | Low-Med | TASK-002 Goravel mapping |
| SH-06 | Laravel framework team (indirect) | Innovation signal | Defines feature frontier (20 features) — not a dependency, but parity benchmark | Low | docs/laravel-13-research.md |
| SH-07 | Security auditor | Gate | Requires `Sec-Fetch-Site` CSRF, JSON session, allow-list deserialization, hyphenated prefixes (#11/#12) | Medium | Breaking-change analysis |

### Stakeholder Requirements (Extracted)

| Req Source | Verbatim Need (paraphrased) | Interpreted Requirement |
|------------|------------------------------|------------------------|
| SH-01 | "I want Laravel routing ergonomics but `cargo check` catches it before runtime." | FR set in PRD §M1: axum-backed typed routing with proc-macro attributes |
| SH-02 | "I don't want `any` in my jobs/events; the compiler should tell me I broke the handler." | FR set in PRD §M4: `Job<T>` / `Event<T>` generics, no `Any` |
| SH-03 | "If the queue backs up I need `pendingSize`/`oldestPendingJob` now, not just `hlens`." | FR-304b: Cloud queue metrics trait (#8) |
| SH-04 | "Let me define an Agent with a Tool, stream its output over WebSocket, and pgvector-search my embeddings — without wiring 4 SDKs." | FR set in PRD §M6: `rustasea-ai` + `rustasea-search` + broadcast |
| SH-07 | "Session cookies must be JSON by default and deserialization must be allow-listed." | NFR-Sec + FR-3xx: session hardening (#12) + CSRF origin-aware |

---

## 4. Scope

### In Scope (by Milestone)

| Milestone | In Scope | Deliverables |
|-----------|----------|--------------|
| **M0 Bootstrap & Core** | `foundation::Application`, layered config (TOML/YAML + env + `.env`), container `Bind`/`Singleton`/`Instance`, provider `register`→`boot` DAG, graceful shutdown, `bootstrap/app.rs` | `rustasea`, `rustasea-foundation`, `rustasea-config` crates; `cargo rustasea new` scaffold; `config/` directory |
| **M1 Routing & HTTP** | `axum` router with group/prefix/naming/resource helper, domain-route prioritization, `route:list` introspection, middleware stack (throttle/cors), typed extractors, `Json`/`View` responses, HTTP client (`reqwest` with `throw` callbacks) | `rustasea-router`, `rustasea-http` crates; `routes/web.rs`; `#[route]` macro; `cargo rustasea route:list` |
| **M2 ORM & Database** | Query builder (`sqlx`/`sea-orm`), Postgres/MySQL/SQLite drivers, `#[derive(Model)]`, migrations/seeders/factories, `pgvector` `whereVectorSimilarTo`/`vector` column, collection `serde` round-trip, `insertOrIgnoreReturning` etc. | `rustasea-orm` crate; `database/migrations/`; `make:model` generator |
| **M3 Auth, Middleware & Validation** | JWT + session guards, `login`/`parse`/`refresh`/`logout`/`user`, `Auth::extend`, `#[authorize]`/`#[middleware]`, CSRF origin-aware (`Sec-Fetch-Site`), rate limiter, strict validation + `ErrorBag`, `#[validate]` macro, JSON session default | `rustasea-auth`, `rustasea-validation` crates; `app/http/middleware/`; `make:middleware`/`make:request` |
| **M4 Queue, Cache, Scheduling & Events** | `Queue::route::<Job>(connection:, queue:)` registry, typed `Job`/`ShouldRetry`/`#[tries]`/`#[backoff]`/`#[timeout]`, drivers `sync`/`database`/`redis`, `dispatch`/`chain`/`batch`, `Cache::touch`, `Lock`, `dispatchAfterResponse`, `schedule:pause`/`resume`, Cloud metrics | `rustasea-queue`, `rustasea-cache`, `rustasea-events`, `rustasea-schedule`; `make:job`/`make:event`/`make:listener` |
| **M5 DX, CLI & Testing** | `cargo rustasea` CLI (clap+xtask), `make:*` (10+ generators), prompts/table/progress/spinner, `Shutdownable`, declarative attributes `#[middleware]`/`#[tries]`/etc., `TestCase` harness, testcontainers isolated DB, `Str` factory resets, paginator views | `rustasea-cli`, `rustasea-macros`, `rustasea-testing`; `bootstrap/commands.rs`; `cargo rustasea make:test` |
| **M6 Advanced** | WebSocket (`tokio-tungstenite`) + SSE `eventStream`, channel auth, `Storage` read-through (primary+fallback), `JsonApiResource` (sparse fieldsets/links/headers), queued `DeleteWhenMissingModels`, AI SDK provider-agnostic trait (12 providers), `Agent`/`Tool`/`make:agent`/`make:tool`, streaming/broadcast/queue/MCP, sub-agents/middleware, `Str::toEmbeddings`/`dropVectorIndex` | `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai`; `app/ai/agents/` + `app/ai/tools/` |

### Out of Scope (and Why)

| Excluded | Rationale | Revisit When |
|----------|-----------|--------------|
| Laravel Nova / Filament-style admin | Separate product surface; blocks focus on framework core | Post-M6 |
| PaaS hosting (Laravel Cloud analogue) | Framework ≠ platform; infrastructure is operator choice | Never in framework crate |
| Full Blade engine parity | Rust prefers compile-time `askama`; runtime parity is low-value | If demand emerges for Blade compat |
| PHP transpilation/bridge | No interop goal; greenfield Rust projects | Never |
| gRPC application layer (beyond `app/grpc/` stub) | Goravel has it; RustaSea leaves it optional behind a feature flag | Community RFC if requested |

### Constraints

- **C-01** Rust 1.88+ stable, edition 2021, `tokio` async throughout — no sync facade.
- **C-02** Workspace per-crate feature gating — incremental adoption must stay `cargo check`-clean with a single crate.
- **C-03** No global `static mut` facades — `AppState` via `axum::extract::State` (`OnceLock`/`Arc`), unlike Goravel `facades.*`.
- **C-04** Generated code via `make:*` must be `rustfmt`-clean and `clippy -- -D warnings` clean.
- **C-05** No runtime `any`/`Box<dyn Any>` for domain payloads (jobs/events) — use generics `Job<T>`/`Event<T>`.

### Assumptions

- **A-01** Laravel 13 feature surface (20 items) is the parity baseline; Laravel 13.x minors beyond v13.30.1 are not retroactively required unless RFC-approved.
- **A-02** `pgvector` (Postgres) is the primary vector store; MariaDB vector is best-effort behind a feature flag.
- **A-03** `sqlx` is primary ORM driver; `sea-orm` is optional ActiveRecord-style alternative — not both required simultaneously.
- **A-04** AI provider SDKs are additive behind `rustasea-ai` feature flags; core framework does not pull AI deps.

### Open Questions (Tracked)

| # | Question | Owner | Blocks | Target Close |
|---|----------|-------|--------|--------------|
| OQ-01 | `sea-orm` vs `sqlx` only — support both or commit to `sqlx`? | Architecture RFC after M0 | M2 | M1 end |
| OQ-02 | `askama` (compile-time) vs `minijinja` (runtime) default — or both behind flags? | M6 design | M6 | M5 end |
| OQ-03 | `object_store` vs per-provider FS crates (aws-sdk-s3 / google-cloud-storage) | M6 Storage design | M6 | M5 end |

---

## 5. Business Requirements

| # | Business Requirement | Priority (MoSCoW) | Mapped To PRD FRs | Laravel 13 Feature(s) | Milestone |
|---|---------------------|-------------------|-------------------|-----------------------|-----------|
| BR-01 | The system shall provide a bootable application skeleton with layered config, typed container, and service-provider lifecycle so that a `cargo run` boots and shuts down gracefully. | Must | FR-000 – FR-009 | Container `call` semantics; `Manager::extend` closure binding (#20) | M0 |
| BR-02 | The system shall provide Laravel-ergonomic HTTP routing and middleware with typed extractors and introspection so that route definitions read like prose and are introspectable. | Must | FR-100 – FR-109 | #18 HTTP Client & Process; #19 domain-route priority; #20 `route:list` (#20) | M1 |
| BR-03 | The system shall provide a fluent, type-safe ORM with migrations/seeders/factories and first-class vector search so that AI workloads and classic CRUD coexist. | Must | FR-200 – FR-214 | #6 vector search; #13 collection serialization; #14 upsert/delete; #15 builder additions | M2 |
| BR-04 | The system shall protect authentication, CSRF, and validation with Laravel 13 security defaults so that hardened session/validation semantics are matched. | Must | FR-300 – FR-313 | #11 origin-aware CSRF; #12 cache/session hardening; #19 strict validation + `ErrorBag` | M3 |
| BR-05 | The system shall support async workloads — typed queues, caching, scheduling, and event dispatch — with observability and pause/resume operations. | Must | FR-400 – FR-420 | #4 queue routing; #5 `touch`; #8 Cloud metrics; #10 pause/resume; #16 contracts | M4 |
| BR-06 | The system shall deliver Laravel-like DX — code generation, declarative attributes, and isolated test harness — so that the develop→test loop is <5s per iteration. | Must | FR-500 – FR-518 | #7 expanded attributes; #20 ModelInspector/Str resets | M5 |
| BR-07 | The system shall differentiate with AI-native (12 providers), JSON:API, real-time broadcasting, read-through storage, and semantic search integrated end-to-end. | Should* | FR-600 – FR-622 | #1 AI SDK; #2 Agents; #3 JSON:API; #6/full; #9 read-through; #17 mail defaults; SSE `eventStream` | M6 |
| BR-08 | The system shall support incremental adoption — any single crate usable standalone — so that projects pay only for crates they include. | Must (architectural) | NFR-Arch-01, FR-000b | — (Rust adaptation of Goravel workspace model) | M0–M6 |
| BR-09 | The system shall adhere to zero-cost ergonomics — proc-macros generating `rustfmt`-clean code, compile-time safety over runtime `any`. | Must (architectural) | NFR-DX-01, NFR-Per-01 | — (Rust adaptation) | M0–M6 |

> *M6 is **Should** in MoSCoW because M0–M5 deliver a shippable Laravel-parity framework; M6 is the differentiator milestone.

---

## 6. Market and Competitive Context

| Competitor | Strengths to Borrow | Gaps RustaSea Exploits |
|------------|---------------------|------------------------|
| **Laravel 13** (PHP) | Gold-standard DX, community, 20-feature velocity | GC pauses, dynamic typing — RustaSea keeps ergonomics with static safety |
| **Goravel** (Go, v1.18) | Closest port; validated Register→Boot, facades, ORM, Artisan mapping | `any`/`interface{}` payloads, global facades, runtime reflection — RustaSea uses generics + `AppState` + proc-macros |
| **axum / actix-web** (Rust) | Minimal, fast, `tokio`-native | Unopinionated; no ORM, queue, schedule, AI SDK, or `make:*` — high assembly cost |
| **Loco / Shuttle** (Rust, Rails-like) | Convention over configuration, batteries-included | Narrower Laravel mapping; no vector/AI-native parity explicitly |

**Market signal:** Laravel 13 deliberately shipped AI-native + vector as headline features after incremental 12-week QOL cadence — this is the parity bar. Teams choosing Rust for perf now also expect AI primitives.

---

## 7. Risks (Summary — Detail in PRD §7)

| # | Risk | L×I | Category | Early Warning | Mitigation Owner |
|---|------|-----|----------|---------------|------------------|
| R-01 | `sqlx`/`sea-orm` duality doubles maintenance; proc-macro complexity spikes | 3×4=12 | Technical | M2 scope creep >30% | Tech Lead |
| R-02 | 12-provider AI trait drifts as providers version APIs | 3×3=9 | Technical | Provider changelog breakage | M6 lead |
| R-03 | Proc-macro `#[middleware]`/`#[validate]` DX inconsistent vs attribute specs in Laravel 13 | 2×4=8 | Technical | Dog-food survey <4/5 | DX lead |
| (Full register: see `prd.md` §7 — 8+ risks with L×I scoring) | | | | | |

---

## 8. Traceability Summary

Every BR → FR → Feature → Milestone linkage is catalogued in `prd.md` §8 *Traceability Matrix*. Each of the 20 Laravel 13 features maps to ≥1 FR; each milestone M0–M6 has ≥1 user story (see `user-stories.md`).

> **Read order:** `brd.md` (this file) → `prd.md` (functional/NFR spec) → `fsd.md` (feature specs) → `user-stories.md` (stories + EARS AC) → `bdd-scenarios.md` (Gherkin).

---

## 9. Approvals and Change Control

| Role | Approver | Signed |
|------|----------|--------|
| Product | PM (author) | — |
| Engineering | Tech Lead | — |
| Security | Security reviewer (for BR-04 / NFR-Sec) | — |

**Change control:** Any BR scope change (add/remove milestone feature, MoSCoW reprioritization, M6 deferral) requires a `docs/adr/ADR-*.md` (repository-root-relative) and `TASK-007` comment update.


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
