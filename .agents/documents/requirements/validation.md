# RustaSea — Idea Validation Report (P1)

> **Owner:** vheins/rustasea | **Phase:** Discovery — P1 | **Task:** TASK-006 | **Date:** 2026-09-07
> **Inputs:** `README.md` (vision, milestones M0–M6, tech stack, Laravel 13 feature map), `docs/laravel-13-research.md` (20 Laravel 13 features, breaking changes, RustaSea implications)
> **Skill:** `idea-validation` (S0→S1→S2→G0) — all four rules applied in sequence

---

## 1. Problem Analysis

### Problem Statement (solution-free)

Teams that built their product on a batteries-included dynamic framework (Laravel, Rails, Django) hit a ceiling on **performance, correctness, and concurrency** as traffic and team size grow. Rewriting in Rust/Go restores runtime guarantees but destroys the velocity, convention, and ergonomics that made the original framework productive. The consequence is a forced trade-off: ship fast **or** run fast — never both.

- Current workarounds: (a) stay on PHP/Go and patch with caching, queue workers, and horizontal scaling (rising infra + ops cost); (b) rewrite hot paths in Rust/Go as microservices while keeping the monolith (polyglot tax, deployment complexity, knowledge split); (c) adopt a raw Rust web stack (`axum`/`actix` + `sqlx`) and hand-roll the framework layer (weeks of glue per project, repeated across teams).
- Quantified consequence: Goravel exists precisely because Go teams wanted Laravel DX without PHP — 1.8k+ stars, v1.18, proves demand for "Laravel ergonomics on a compiled language." Rust teams report the same gap but lack an equivalent. Raw `axum`/`actix` require assembling 8–12 crates before feature parity with Laravel's M0–M2 alone.

### Affected Users

| Segment | Frequency of Pain | Current Workaround |
|---------|-------------------|--------------------|
| Backend teams (5–30 eng) outgrowing Laravel/Rails on p95 latency or background-job throughput | Every deploy / every incident | Microservice extraction of hot path; Redis-heavy queue tuning |
| Platform/infra teams standardizing on Rust for new services | Every new service kickoff | Copy-paste `axum` + `sqlx` + `tokio` boilerplate; internal "starter kit" |
| Solo / small-team founders who chose Rust early | Daily DX friction | Hand-rolled CLI (`clap` + `xtask`), custom ORM wrappers, ad-hoc validation |

### Root Cause (5-Why)

1. No batteries-included Rust web framework with Laravel-level DX → 2. Rust web ecosystem is intentionally modular (`axum`, `sqlx`, `tokio` are building blocks, not a framework) → 3. Framework ergonomics require proc-macros, convention, codegen, and opinionated defaults — culturally at odds with Rust's explicitness bias → 4. Prior attempts (Rocket, Actix) optimized for performance or novelty, not for porting a proven DX model → 5. No team has systematically mapped Laravel/Goravel idioms to Rust's type system (container, providers, ORM, Artisan) as a product rather than a demo.

### Pain Points (verifiable behaviors, not opinions)

- `cargo new` + `axum` still requires manual wiring of config, DI/container, migrations, auth, and validation — 3–5 days before "hello authenticated user" vs. `laravel new` in minutes.
- Typed job/event payloads in Rust today are `any`/`serde_json::Value` unless the team invents its own — no first-class `Job<T>` / `Event<T>` convention exists.
- Vector/semantic search (`whereVectorSimilarTo`, `pgvector`) is bolted on per-project, not a framework primitive — despite Laravel 13 making it headline (#6).

### Gate Check — Problem Analysis

- [x] Problem statement contains no solutions
- [x] Root cause reaches systemic level (≥3 whys)
- [x] Pain points are verifiable behaviors
- [x] Consequences quantified (time, infra cost, adoption signal)
- [x] Assumptions extracted and falsifiable (see §5)
- [x] Risk levels assigned consistently

---

## 2. Hypothesis & Target Users

### Primary Persona — "Mira, Platform Lead"

| Attribute | Value |
|-----------|-------|
| Name / Role | Mira, Platform Lead at 40-person B2B SaaS (series A) |
| Stack | Laravel monolith + 3 Go microservices; evaluating Rust for next service |
| Goals | Cut p95 latency <100ms; keep onboarding <1 day for Laravel-trained hires; avoid polyglot sprawl |
| Frustrations | Rebuilding auth/validation/queue per Rust service; Go facades reintroduce `any`; docs are crate-scoped, not workflow-scoped |
| Tech Savviness | High — comfortable with `tokio`, `serde`, `sqlx`; wants conventions, not magic |
| Quote | "I don't need another router. I need `laravel new` for Rust." |

### Secondary Persona — "Ken, Solo Founder"

| Attribute | Value |
|-----------|-------|
| Role | Solo founder, Rust-first product |
| Goals | Ship full-stack features alone at Laravel velocity without hiring PHP devs |
| Frustrations | `cargo make:*` doesn't exist; proc-macro ergonomics are DIY |
| Quote | "If I have to write my own `make:model` again I'm going back to Rails." |

### Core Hypothesis (falsifiable)

> **If** we ship a Laravel-idiomatic Rust framework with convention-over-configuration, typed container/providers, `axum`-backed routing, `sqlx`/`sea-orm` ORM with `#[derive(Model)]`, `cargo rustasea make:*` generators, and typed `Job<T>`/`Event<T>`, **then** teams like Mira's will adopt it for at least one production service within 90 days **because** it eliminates 3–5 days of per-service boilerplate while preserving Rust's safety/performance.

### Success Metrics (M0–M2 window)

| Metric | Target | Threshold (pivot trigger) | Method |
|--------|--------|---------------------------|--------|
| M0 boot success | `cargo run` boots, loads `config/*.toml` + `.env`, resolves singleton, graceful `SIGTERM` | No boot in <5 min from `cargo rustasea new` → UX failure | `cargo test` + manual QA |
| M1 route parity | `Route::get` equivalent + `route:list` + domain-route precedence correct | Domain catch-all shadows non-domain → correctness bug | Integration tests |
| M2 ORM round-trip | `User` model → migrate → `Factory::create` → `whereVectorSimilarTo` + `serde` collection round-trip | Relations lost on `serde` serialize → M2 fails (cf. Laravel 13 #13) | `sqlx::test` + `testcontainers` |
| Adoption signal | 50 stars / 5 external contributors OR 1 production user by M2 | <10 stars and zero external PRs after 60 days of public M0–M1 → demand risk | GitHub analytics |
| DX satisfaction | Generated code is `rustfmt`-clean and compiles on first try | >20% of `make:*` outputs require manual fix → generator quality failure | CI `cargo check` on generated app |

### Pivot Triggers

- **Persevere if:** M0/M1 success criteria met and ≥1 external team opens an issue/PR or reports running a service.
- **Investigate if:** Stars grow but no production usage — DX is attractive, onboarding or docs are the bottleneck.
- **Pivot if:** M0 feasibility spike fails (container/proc-macro ergonomics untenable) or M1 routing cannot match Laravel semantics without `unsafe` / unreasonable trait complexity → narrow scope to library crates rather than framework.

---

## 3. Market Research

### 3.1 Market Sizing — Rust Web Frameworks

> Confidence: **Medium** for directional sizing; **Low** for absolute TAM dollars. Rust web framework market lacks standalone revenue data — estimates are proxy-based from developer population × framework adoption × willingness to invest in DX. Flagged as assumption.

| Level | Definition | Estimate | Basis |
|-------|------------|----------|-------|
| **TAM** | All web backend development where Rust is a viable choice (performance-sensitive APIs, platforms, infra services) | ~1.5M active Rust developers (2025–2026 surveys) × ~30% doing web/backend = **~450k addressable devs** | Rust Survey, GitHub language stats, Stack Overflow — proxy; no direct TAM revenue |
| **SAM** | Subset that values both performance **and** framework ergonomics (would choose a batteries-included framework over raw crates) | ~25–35% of TAM = **~110k–160k devs**; teams of 2–50 eng that previously used Laravel/Rails/Django or Goravel | Goravel traction + Laravel's "most loved framework" signal + `axum`/`actix` download ratios |
| **SOM (3yr)** | RustaSea's realistic capture if M0–M2 ship with strong DX | **0.5–1.5% of SAM = ~600–2,000 active projects** | Analog: Goravel reached ~1.8k stars in Go (larger TAM) without Rust's safety advantage; Loco/Rocket are <5k stars each |

**Monetization note:** Framework is MIT/Apache-2.0 (README license). Direct revenue is not the near-term metric — adoption, ecosystem, and hiring signal are. Future options (paid Cloud, support, hosted vector/AI) mirror Laravel Cloud (research #8) but are explicitly post-M6.

### 3.2 Competitor Matrix (7)

| Competitor | Language | Positioning | Core Strengths | Pricing / License | Weakness vs. RustaSea Thesis |
|------------|----------|-------------|---------------|-------------------|------------------------------|
| **Laravel 13** | PHP 8.3+ | Batteries-included DX king; 20 features in 13.0.0 (AI SDK, vector, JSON:API, queue routing, declarative attributes) | Unmatched ergonomics, ecosystem, hiring pool; AI-native headline (research #1–#10) | MIT | Dynamic typing, runtime errors, GC, concurrency limits — the ceiling RustaSea escapes |
| **Goravel** (v1.18) | Go | Laravel port for Go; closest prior art | Proves service providers, container, ORM facades, Artisan CLI translate to compiled language; 1.8k+ stars | MIT | `any`/`interface{}` payloads, global facades, `gin` router, stringly-typed config — all fixed by RustaSea's typed design (README §Goravel Inspiration) |
| **Axum** | Rust | Modular HTTP framework on `tokio`/`tower` | `tower` middleware, extractor ergonomics, `tokio` alignment; RustaSea's chosen base (README Tech Stack) | MIT/Apache-2.0 | Deliberately unopinionated — no ORM, auth, queue, CLI, or conventions; requires assembly |
| **Actix Web** | Rust | High-performance actor-based HTTP | Benchmark leader, mature | MIT/Apache-2.0 | Actor model diverges from Laravel mental model; steeper learning curve; Tower ecosystem gap |
| **Rocket** | Rust | Ergonomic, codegen-heavy web framework | Attribute macros, batteries-included feel (closest to Laravel DX in Rust) | MIT/Apache-2.0 | Historically tied to nightly, slower `tokio` alignment; smaller ecosystem than `axum`; no ORM/queue story |
| **Loco** | Rust | Rails-like Rust framework (ActiveRecord-inspired) | Most direct "Rails for Rust" attempt; conventions, background jobs, ORM via `sea-orm` | MIT | Opinionated Rails mapping (not Laravel/Goravel); younger, smaller community; no AI/vector headline |
| **Dioxus** | Rust | Fullstack / frontend-first (CSR/SSR) | Excellent for UI-heavy Rust apps | MIT/Apache-2.0 | Frontend-centric; not a backend framework competitor — included as **indirect** (validates Rust DX appetite) |

> Gate satisfied: ≥4 competitors compared (7 total, direct + indirect + cross-language reference).

### 3.3 Feature Gap Analysis

| Gap | Who Leaves It Underserved | RustaSea Opportunity |
|-----|---------------------------|----------------------|
| Laravel-grade DX on a compiled, memory-safe runtime | `axum`/`actix` leave DX to the user; Loco targets Rails idioms | Typed Laravel idioms: container, providers (`Register→Boot` + DAG), `make:*`, `route:list`, `Job<T>` |
| Strongly-typed jobs/events/queue routing | Goravel uses `any`; raw Rust uses `serde_json::Value` | `Queue::route::<Job>(queue:)` + typed `Job`/`Event` traits (README M4, Laravel 13 #4) |
| Vector/semantic search as framework primitive | Bolt-on per project; Laravel 13 made it headline #6 | `whereVectorSimilarTo` + `vector` column + `toEmbeddings` from M2 via `pgvector` |
| Declarative attributes over config | Laravel 13 expanded to `#[Middleware]`/`#[Tries]`/`#[Authorize]` etc. (#7) | Rust proc-macros are idiomatic: `#[middleware]`, `#[tries]`, `#[validate]`, `#[authorize]` |
| JSON:API + real-time + AI SDK in one stack | Fragmented across crates; no unified story | M6: `JsonApiResource` (sparse fieldsets), WebSocket/SSE, `rustasea-ai` over 12 providers |

### 3.4 Value Proposition & Positioning

**Positioning statement:**

> For backend teams that outgrew Laravel/Rails on performance but not on productivity, **RustaSea** is the Rust framework with Laravel ergonomics that boots a production-ready service in minutes — unlike raw `axum`/`actix` (which require weeks of assembly) and unlike Goravel/Go (which reintroduces `any` and global facades), RustaSea leverages Rust's type system to make ergonomics safer.

**Primary value prop:** Zero-cost ergonomics — expressive, Laravel-familiar APIs that compile away where possible.

**Supporting benefits:**

- Convention over configuration (sensible defaults, explicit opt-out).
- Incremental adoption — use one crate or the full stack (workspace-gated features).
- Compile-time safety for queries, validation, and job payloads.
- AI/vector-native from M2/M6 (differentiation vs. Loco/Rocket).

**Elevator pitch (1 sentence):**

> RustaSea proves you can have Laravel's velocity and Rust's safety in the same framework.

---

## 4. Feasibility & Prioritization

### 4.1 Feasibility

| Dimension | Score (1–10) | Key Risk | Mitigation |
|-----------|-------------|----------|------------|
| **Technical** | 6.5 | Ergonomic Rust APIs (container DI, provider DAG, `#[derive(Model)]`, route macros) risk trait-complexity or lifetime friction that erodes DX | M0 spike: prototype `Container::bind/singleton/instance` + `ServiceProvider` lifecycle before committing to full milestone; prefer `OnceLock`/`Arc<AppState>` over global `static mut` |
| **Financial** | 9 | Low infra cost (OSS, no paid dependencies required for M0–M2); risk is contributor time | Workspace crates are MIT/Apache-2.0; CI uses `testcontainers` (ephemeral) not hosted infra |
| **Timeline** | 6 | M0–M2 in ~6 months (README roadmap: M0 Q4 2026 → M2 Q1 2027) with small team is tight but dependency-ordered (no circular deps) | Strict M0→M1→M2 sequencing; defer M6 AI/broadcast until M5 DX is solid; each milestone is a tagged release |

**Technical deep-dive flags:**

- Async runtime is settled: `tokio` is de-facto and powers `axum`/`sqlx`/`deadpool` — low risk.
- ORM choice is hedged: `sqlx` primary (compile-time checked) + `sea-orm` optional — mitigates ActiveRecord vs. query-builder trade-off.
- Proc-macros (`syn`/`quote`/`proc-macro2`) are well-trodden but increase compile times — monitor with `cargo build --timings`.
- `pgvector` behind feature flag avoids forcing Postgres on all users.
- Laravel 13 research cautions on contract churn (`Cache::touch`, `Queue::pendingSize`, `Dispatcher::dispatchAfterResponse`) — RustaSea traits should version-gate additive methods or use default impls.

### 4.2 Risk Matrix (Top 5)

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|------------|--------|------------|
| 1 | Trait/lifetime ergonomics make container or routing feel worse than raw `axum` | M | H | M0 spike + dogfood `cargo rustasea new` internally; abort to library crates if DX regresses |
| 2 | ORM parity with Laravel 13 #13–#15 (collection `serde`, `upsert` strict `uniqueBy`, `whereBinary` etc.) is underestimated | M | M | Scope M2 to `sqlx` subset first; `insertOrIgnoreReturning` etc. behind follow-up tickets |
| 3 | No external adoption despite good DX (demand risk) | M | H | Ship M0 publicly early; measure stars/PRs; invest in docs/examples over more features |
| 4 | Compile-time cost of proc-macros hurts iteration speed | L | M | Feature-gate macros; keep `rustasea-macros` crate isolated; benchmark `cargo check` |
| 5 | Vector/AI scope creep pulls focus from M0–M2 core | M | M | Enforce milestone gates: M6 work cannot start before M2 success criteria pass |

### 4.3 MoSCoW — MVP (M0–M2) Scope

| Feature | Category | Effort | Rationale |
|---------|----------|--------|-----------|
| `foundation::Application`, typed config (`config/*.toml` + `.env` + env overlay), container `Bind`/`Singleton`/`Instance`, provider `register→boot`, graceful shutdown | **Must** | M | M0 success criteria; nothing else boots without it |
| `axum` router + `tower` middleware, route groups/prefix/naming/resource, domain-route precedence, `route:list` | **Must** | M | M1 success criteria; Laravel 13 #19 domain priority is a correctness requirement |
| Typed request extractors, `Json`/`View` responses, HTTP client (`reqwest` wrapper) | **Must** | S | Minimal HTTP ergonomics to be useful |
| Query builder (`sqlx`/`sea-orm`), `where`/`orWhere`, `find`/`first`/`firstOrFail`, `create`/`save`/`update`/`delete`, `paginate`/`cursor`, transactions, `#[derive(Model)]` with `deleted_at` | **Must** | L | M2 core; collection `serde` round-trip (Laravel 13 #13) included |
| Migrations + `cargo rustasea make:migration` / `migrate` | **Must** | M | Without migrations the ORM is not shippable |
| `whereVectorSimilarTo` + `vector` column via `pgvector` (feature-flag) | **Should** | M | Laravel 13 #6 headline; defer full embedding pipeline to M6, ship column + query primitive in M2 |
| Seeders / factories / `Factory::create` | **Should** | S | Testing story starts in M2, completes in M5 |
| Auth / validation / CSRF (M3) | **Won't** (in MVP) | — | Explicitly deferred to M3; MVP is M0–M2 only |
| Queue / Cache / Schedule / Events (M4) | **Won't** (in MVP) | — | Deferred to M4 |
| CLI `make:*` generators beyond migration/model, `TestCase` harness, AI SDK (M5–M6) | **Won't** (in MVP) | — | Deferred; `cargo rustasea new` scaffold is the only M0 generator |

### 4.4 Prioritization (RICE) — MVP Features

| Feature | Reach | Impact | Confidence | Effort | RICE = (R×I×C)/E | Notes |
|---------|-------|--------|------------|--------|------------------|-------|
| Container + providers (M0) | 200 | 3 | 0.8 | 3 | **160** | Unblocks everything; highest leverage |
| Routing + `route:list` + domain precedence (M1) | 200 | 3 | 0.85 | 2 | **255** | Quick win, visible DX |
| ORM core + `#[derive(Model)]` + migrations (M2) | 180 | 3 | 0.7 | 4 | **95** | Largest effort, core value |
| Vector primitive (`whereVectorSimilarTo`) (M2) | 80 | 2 | 0.6 | 2 | **48** | Differentiator, smaller reach initially |
| Config loader + `.env` | 200 | 2 | 0.9 | 1 | **360** | Tiny effort, universal need — do first |

> RICE confirms build order: config → container/providers → routing → ORM core → vector primitive.

### 4.5 MVP Scope Definition (M0–M2) — What's In / What's Out

**In (MVP = M0 + M1 + M2):** Bootable app skeleton, typed config, service container + providers, `axum` routing with domain precedence and introspection, ORM + migrations + `#[derive(Model)]` + vector query primitive.

**Out (explicitly deferred):** Auth/JWT/session/CSRF/validation (M3), queue/cache/schedule/events (M4) including `Cache::touch` and `Queue::route` (Laravel 13 #4–#5), CLI `make:*` beyond model/migration and testing harness (M5), broadcast/search/filesystem/AI SDK/JSON:API (M6).

**Boundary rationale:** M0–M2 alone delivers "hello authenticated user" → "hello persisted & searchable user" without requiring async job infrastructure or AI provider work. This minimizes time-to-first-value while keeping M6 bets optional.

### 4.6 Build Order

1. **M0** — `rustasea-foundation` + `rustasea-config` + `rustasea` umbrella + `cargo rustasea new` scaffold
2. **M1** — `rustasea-router` + `rustasea-http` + `#[route]` macro + `route:list`
3. **M2** — `rustasea-orm` (sqlx primary) + `rustasea-macros::Model` + migrations + `pgvector` feature flag
4. **M3–M6** — only after M0–M2 success criteria pass (gated)

### Gate Check — Feasibility & Prioritization

- [x] Technical score justified by specific risks (trait/complexity, ORM parity, contracts)
- [x] Financials include recurring infra (testcontainers, no hosted deps for MVP)
- [x] Timeline vs team size logic is sound (dependency-ordered, no circular deps)
- [x] MoSCoW must-haves fit within M0–M2 window
- [x] MVP definition is clear and bounded with explicit Won't list
- [x] Effort estimates reflect small team
- [x] RICE scoring is consistent; high-effort low-impact items flagged for deferral

---

## 5. Assumption Mapping

| # | Assumption | Risk | Validation Method | Falsifiable Test |
|---|------------|------|-------------------|------------------|
| 1 | Teams want Laravel ergonomics on Rust (not just raw `axum`) | **High** | 10 interviews with Laravel→Rust/Go teams + Goravel adoption as proxy | <30% of interviewees rank "framework DX" as top-3 pain → assumption fails |
| 2 | `axum` + `tower` + `sqlx` + `tokio` can support Laravel-idiomatic APIs without `unsafe` or painful lifetimes | **High** | M0 spike (2-week prototype of container + provider DAG + `#[route]`) | Spike cannot achieve `Route::get` ergonomics with typed extractors and compiles cleanly → pivot to library crates |
| 3 | Developers will accept Rust's learning curve if DX matches Laravel velocity | **Medium** | Landing page + `cargo rustasea new` try-out; measure time-to-first-route | Median time-to-first-route >30 min or >50% abandon during setup → onboarding failure |
| 4 | Vector/semantic search as a day-one primitive is a differentiator (Laravel 13 #6) | **Medium** | Feature-flag `whereVectorSimilarTo` demo with `pgvector` in M2 | Zero interest / no usage in early feedback → demote to M6-only |
| 5 | `pgvector` + Postgres is acceptable as the default vector backend | **Low** | M2 supports Postgres + MySQL + SQLite; vector is opt-in | Users demand non-Postgres vector and refuse feature flag → add abstraction |
| 6 | MIT/Apache-2.0 OSS will attract contributors without paid incentives | **Medium** | Public M0–M1 release + RFC process (README Contributing) | No external PRs/issues in 60 days → invest in docs/examples before more code |
| 7 | Laravel 13's 20 features are a stable target (not churn) | **Low** | Cross-check `docs/laravel-13-research.md` against `laravel.com/docs` + framework changelog (research §Verification) | Laravel 14 redefines AI/vector surface → M6 scope revisits |

> Highest-risk assumptions are #1 (demand) and #2 (technical ergonomics) — both validated before heavy M2 investment.

---

## 6. Go / No-Go Decision

### Decision: **GO — Conditional**

RustaSea is **approved to proceed through M0–M2 (MVP)** under the following conditions. This is not an unconditional green light to M6.

### Rationale

- **For GO:** Demand signal is credible (Goravel proves Laravel-on-compiled-language has users; Rust's safety/performance need is growing; raw Rust web stack has a clear DX gap). Technical path is plausible (`axum`/`tokio`/`sqlx` are mature; proc-macros are idiomatic for declarative attributes — Laravel 13 #7 maps cleanly). Milestones are dependency-ordered with no circular deps and each has testable success criteria. M0–M2 alone delivers standalone value. Competitive positioning is differentiated (typed `Job<T>`/`Event<T>`, vector-native, incremental crate adoption) — not "another router."
- **For caution:** TAM/SAM estimates are proxy-based (low confidence on absolute dollars); Rust web framework market is fragmented and adoption is not guaranteed. Highest risk is technical ergonomics (#2) — if Rust's type system makes Laravel idioms feel worse, the thesis collapses. Financial risk is low but contributor risk is real.

### Conditions (Gates)

1. **M0 spike gate:** 2-week prototype of container + `ServiceProvider` lifecycle + `#[route]` must demonstrate Laravel-like ergonomics without `unsafe` and with acceptable `cargo check` times. Fail → re-scope to focused crates (e.g., `rustasea-router` + `rustasea-orm` as standalone libraries).
2. **Public M0 gate:** Publish M0 as `0.1.0` with `cargo rustasea new` and measure adoption signal (stars, issues, try-outs) before committing M2 headcount.
3. **M2 exit gate:** `whereVectorSimilarTo` stays feature-flagged; full AI SDK (M6) does not start until M2 `serde` collection round-trip and migration story are green — prevents M6 scope creep from starving core.
4. **Re-validate at M2:** Re-run competitor check (Loco, Rocket, Axum releases) and Laravel 14 delta before approving M3–M6 funding.

### What Would Flip to No-Go

- M0 spike proves trait/lifetime ergonomics are untenable for the target DX.
- Zero external interest after public M0–M1 with docs and examples (demand assumption #1 falsified).
- Team cannot sustain M0–M2 sequencing and is forced to parallelize M3–M6 prematurely (timeline assumption collapses).

---

## Traceability

| README Milestone | Validation Coverage |
|------------------|---------------------|
| M0 Bootstrap & Core | §4.1–§4.6 MVP Must; RICE 160–360 |
| M1 Routing & HTTP | §4.3–§4.4; Laravel 13 #18–#20 (domain precedence, `route:list`, HTTP client) |
| M2 ORM & Database | §4.3–§4.6; Laravel 13 #6, #13–#15 (vector, `serde` collections, `upsert`/`whereBinary`) |
| M3 Auth/Middleware/Validation | Deferred from MVP; Laravel 13 #11–#12, #19 (CSRF, hardening, strict validation) |
| M4 Queue/Cache/Schedule/Events | Deferred; Laravel 13 #4–#5, #8, #10, #16 (`Queue::route`, `touch`, Cloud metrics, pause/resume) |
| M5 DX/CLI/Testing | Deferred; Laravel 13 #7, #20 (attributes, `ModelInspector`, `route:list` fields) |
| M6 Advanced (Broadcast, AI, Filesystem) | Deferred; Laravel 13 #1–#3, #6, #9, #17–#18 (AI SDK, Agents, JSON:API, read-through, SSE) |

## Sources & Confidence

- `docs/laravel-13-research.md` — cross-checked against `laravel.com/docs/releases`, `.../13.x/upgrade`, `laravel/framework` v13.0.0 changelog (§Verification).
- Market sizing flagged **Medium/Low** confidence — proxy-based, no direct Rust framework revenue data.
- Competitor claims (stars, versions) are point-in-time and should be refreshed at M2 gate.

## Handoff

- **Next step:** `@product-planning` — PRD, sprint plan, and risk register for M0 spike.
- **Requires confirmation:** M0 spike timebox (proposed: 2 weeks, 1–2 eng) and public M0 release criterion.

---

*Generated via `idea-validation` skill — problem-analysis → hypothesis-target → market-research → feasibility-prioritization. Gates checked per-rule.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
