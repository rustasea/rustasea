# RustaSea — Project Brief

> A Rust framework with Laravel ergonomics — expressive syntax, convention over configuration, and Rust-grade safety and performance.

## Vision

RustaSea brings the developer experience that made Laravel the most loved PHP framework to Rust — without sacrificing what makes Rust great. Route definitions that read like prose, Eloquent-inspired query builders with compile-time safety, Artisan-like code generation via proc-macros, and a service container that leverages Rust's type system.

**Core thesis:** Laravel proves ergonomics and velocity win hearts; Rust proves safety and performance win production. RustaSea proves you can have both.

## Design Principles

- **Convention over configuration** — sensible defaults, explicit opt-out.
- **Type safety as a feature** — generics, lifetimes, and traits replace runtime `any` blobs.
- **Zero-cost ergonomics** — expressive APIs that compile away where possible.
- **Incremental adoption** — use one crate or the full stack.

## Target Audience

Teams that outgrew dynamic-language frameworks on performance, correctness, or concurrency — but do not want to outgrow the productivity that made them ship fast.

## Synthesis: Laravel × Rust

| Laravel strength | Rust strength | RustaSea synthesis |
|---|---|---|
| Expressive routing, middleware, validation | Ownership, lifetimes, fearless concurrency | `axum` routing with typed extractors + proc-macro attributes |
| Eloquent ORM — fluent, chainable | Compile-time query checking | `sqlx`/`sea-orm` with derive macros; vector search from day one |
| Artisan code generation | `cargo` + proc-macros + `clap` | `cargo rustasea make:*` with `xtask` |
| Queue / Schedule / Events | `tokio` async runtime | Typed jobs/events, backpressure-aware queues |
| Blade / JSON:API resources | `serde` / `askama` | `JsonApiResource` via `serde` |
| Batteries-included DX | Minimal runtime, no GC | Pay only for crates you include |

## Prior Art

**Goravel** (Go port of Laravel, v1.18) is the closest reference. Validated patterns: service providers with Register→Boot lifecycle, container Bind/Singleton/Instance, fluent ORM, Artisan CLI, Queue/Event/Schedule/Cache/Auth. RustaSea differs: explicit `AppState` via `axum::extract::State` (no global facades), strongly-typed generics over `any`, proc-macros over reflection, `axum` over `gin`, `testcontainers` over Docker helpers, typed `config`+`serde` over stringly-typed config.

## Milestones (Dependency-Ordered)

| Milestone | Focus | Depends On |
|---|---|---|
| **M0** Bootstrap & Core | `foundation::Application`, typed config, container, providers, graceful shutdown | — |
| **M1** Routing & HTTP | `axum` router, groups, middleware, typed extractors, HTTP client | M0 |
| **M2** ORM & Database | `sqlx`/`sea-orm`, `#[derive(Model)]`, migrations, seeders, factories, `pgvector` | M0, M1 |
| **M3** Auth, Middleware & Validation | JWT + session guards, CSRF, `#[validate]`, `ErrorBag` | M1, M2 |
| **M4** Queue, Cache, Scheduling & Events | `Queue::route`, `Cache::touch`, `schedule:pause`, event dispatch | M0, M2, M3 |
| **M5** DX, CLI & Testing | `cargo rustasea` CLI, `make:*` generators, `TestCase` harness | M0–M4 |
| **M6** Advanced | Broadcasting, search, filesystem, AI SDK (12 providers), real-time | M1–M5 |

> All 20 Laravel 13 features (2026-03-17, PHP 8.3+) mapped to milestones — see `docs/laravel-13-research.md` and `README.md` §Laravel 13 Feature Map.

## Tech Stack (Selected)

`tokio` · `axum`+`tower` · `sqlx`/`sea-orm` · `deadpool` · `validator` · `jsonwebtoken`+`argon2` · `serde` · `config`+`dotenvy` · `clap`+`cargo xtask` · `syn`/`quote` · `moka`+`deadpool-redis` · `askama` · `reqwest` · `object_store` · `pgvector`+`async-openai` · `testcontainers`

## Document Skeleton

```
.agents/documents/
├── requirements/brief.md   # this file
├── design/
├── tasks/
└── application/
```

## Status

Discovery phase — no code. This brief is the S0 context for blueprint planning (idea-to-blueprint S1/S2).

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
