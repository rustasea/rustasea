# ADR-0002 — RustaSea starter-kit architecture (shared core + presentation variants)

> **Status:** Proposed
> **Date:** 2026-09-11
> **Deciders:** Tech Lead, Platform
> **Milestone:** M3 (auth core) · M5 (scaffolder) · M6 (view engine, Inertia, real-time)
> **Related:** TASK-004 · TASK-001 finding C8 · TASK-002 variant mapping · `README.md` §Milestones · `docs/laravel-parity.md` §8 · `docs/milestones.md` · blueprint `.agents/documents/design/starter-kit-architecture.md`

## Context

RustaSea has no presentation layer. `resources/views/` contains a single static `welcome.html` served with `include_str!` (`routes/web.rs:16`); no template engine is declared (`README.md:199` proposes askama/minijinja, `docs/laravel-parity.md:80` marks `Illuminate\View` **Planned**, `docs/milestones.md:280` records template rendering as missing). There is no `cargo rustasea new` scaffolder (`docs/milestones.md:74,247`), the provider/command registries are empty (`bootstrap/providers.rs:11`, `bootstrap/commands.rs:11`), the session guard is a placeholder whose `store` is `#[cfg(test)]` and whose `logout` is a no-op (`crates/rustasea-auth/src/session.rs:117,147,202-210`), and the config loader reads only `config/app.toml` (`crates/rustasea-config/src/lib.rs:16`).

Laravel's starter kits solve the same problem with a **shared Fortify auth core** and presentation-only variants (Blade `resources/views`; React/Vue `resources/js` + Inertia + Vite; Livewire `resources/views` + Flux). RustaSea needs the Rust equivalent: one auditable auth/domain core, four presentation strategies, and a generator that emits the correct layout.

The design must satisfy: (a) a single security-critical auth path shared by all variants; (b) zero-cost ergonomics — a Blade app must not link WASM/Inertia code (`architecture.md:14`); (c) Laravel layout parity (`app/`, `routes/`, `resources/`, `config/`, `database/`, `tests/`); (d) Rust compile-time safety as a first-class feature (`README.md:20`).

## Decision

1. **Shared core, variant presentation.** Generate one core — `app/actions/auth`, `app/concerns`, `app/http/requests/settings`, `app/models`, `routes/{web,auth,settings,console}.rs`, `database/{migrations,factories,seeders}`, `tests/{feature,unit}` — and vary only `resources/` plus the generated `Cargo.toml` feature set. The scaffolder parameterizes presentation, never auth logic.
2. **View engine: `rustasea-view` with askama as default and minijinja behind feature `runtime-templates`.** A `ViewEngine` trait abstracts rendering; askama gives compile-time-checked templates in the binary; minijinja covers dev hot-reload and user-authored templates.
3. **Inertia analogue: `rustasea-inertia`.** A `Page<T>` envelope (`component`, `props`, `url`, `version`), the full `X-Inertia*` header contract, partial reloads filtered on the wire prop map, per-request shared props, and `409` + `X-Inertia-Location` on version mismatch. A WASM `rustasea-inertia-client` mounts components via a generated registry.
4. **WASM mapping: `react` → Dioxus, `vue` → Leptos**, both on the shared Inertia contract.
5. **Livewire analogue: askama + HTMX + `rustasea-broadcast`** (WS/SSE, already real at `crates/rustasea-broadcast/src/lib.rs:36`).
6. **Scaffolder: `rustasea-scaffold` library behind a `cargo-rustasea` subcommand** — `cargo rustasea new <app> --variant {blade|react|vue|livewire}`. `cargo artisan new` is retained only as an in-app alias.
7. **Auth: complete the existing `rustasea-auth` session guard on `tower-sessions`** (declared at `Cargo.toml:28`) — real `store`, real `logout` (destroy + rotate), session-id rotation on login, and Fortify-like generated `app/actions/auth`.
8. **Config: auto-discover all `config/*.toml`** in `rustasea-config`, preserving the layered merge.

Full detail, the generated directory tree, per-variant deltas, the request flow, and the phased SK-0…SK-6 plan are in the blueprint (`.agents/documents/design/starter-kit-architecture.md`).

## Consequences

**Positive**
- One security-critical auth path; a session/CSRF fix lands once for all four variants.
- Blade/Livewire apps never link WASM or Inertia code (feature-gated), honoring zero-cost ergonomics.
- Laravel layout parity and a single `cargo rustasea new` entrypoint.
- askama makes template typos build failures, not production errors.

**Negative**
- Two WASM frameworks (Dioxus + Leptos) and two view engines widen the maintenance matrix; a framework/template-engine bump touches adapter code.
- askama recompiles on every template edit, slowing the inner dev loop — mitigated by the `runtime-templates` feature.
- Partial reloads force props to an untyped JSON map at the wire boundary, weakening type guarantees for that slice.
- Completing the session guard is security-sensitive and needs dedicated fixation/logout/CSRF tests.
- `resources/js` will hold Rust/WASM sources — a naming compromise for Laravel parity.

**Neutral**
- `rustasea-view` and `rustasea-inertia` become optional umbrella features; the crate DAG gains three leaves with no back-edges.
- The scaffolder adds a cargo subcommand binary to the workspace.
- `cargo artisan new` wording in `README.md`/`docs/milestones.md` becomes an alias rather than the primary command.

## Alternatives

| Decision | Option | Rejected because |
|---|---|---|
| Core vs variants | Four independent template repos | Auth/security code drifts four ways; quadruple patch surface |
| Core vs variants | Runtime presentation dispatch | No Rust runtime reflection; pulls every engine/WASM target into every app |
| View engine | minijinja only | No compile-time template checking; typos reach production |
| View engine | `tera` | Runtime-only, heavier, no advantage over minijinja |
| View engine | `maud` (macro HTML) | Templates in Rust source, not `resources/views/` — breaks parity/designer workflow |
| Inertia | REST + separate SPA repo | Duplicates routing/auth/CSRF/CORS; diverges from Laravel parity |
| Inertia | GraphQL / tRPC analogue | Heavier contract, no Laravel analogue, overkill |
| Inertia | Embedded V8/QuickJS SSR | Enormous dependency/ops surface, not Rust-native |
| WASM | Single framework for both variants | Breaks React/Vue mental-model fidelity the kits exist to provide |
| WASM | `yew` | No capability Dioxus/Leptos lack; a third framework adds cost without value |
| WASM | Real React/Vue via `wasm-bindgen` + Node | Reintroduces the JS/`node_modules` toolchain the Rust-native thesis avoids |
| Livewire | Stateful component protocol over WS | Rust ownership complexity, per-connection memory, bespoke protocol |
| Livewire | Alpine.js sprinkles | Adds a JS toolchain, breaks all-Rust stack |
| Scaffolder | `cargo artisan new` | Chicken-and-egg — runs inside an app that does not yet exist |
| Scaffolder | `cargo generate` (third-party) | External dep, untyped templates, no code sharing with `make:*` |
| Auth | JWT for all web variants | Browser bearer storage is XSS-exposed; mismatches Laravel session kits |
| Auth | `axum-login` crate | Couples app types; hides the session hardening policy already in `rustasea-auth` |
| Config | Hardcoded `config/app` path | Every new config file silently fails to load |

## Note — ADR-location consolidation (finding C5)

This ADR lives under `docs/adr/` using the 4-digit convention established by `docs/adr/ADR-0001-jsonwebtoken-10-msrv-bump.md`. The former **dual ADR location** (`docs/adr/` 4-digit vs `.agents/documents/design/decisions/` `ADR-001`…`ADR-006`) was finding **C5**; it is now **resolved** by TASK-005 — all ADRs live under `docs/adr/` and the legacy `ADR-001`…`ADR-006` are renumbered `ADR-0003`…`ADR-0008` (see `docs/adr/README.md`). New ADRs follow the `docs/adr/ADR-XXXX-*.md` convention.

## References

- Blueprint — `.agents/documents/design/starter-kit-architecture.md`.
- `docs/adr/ADR-0001-jsonwebtoken-10-msrv-bump.md` — 4-digit convention.
- `README.md` §Milestones · `docs/laravel-parity.md` §8 · `docs/milestones.md` (M3/M5/M6).
- `.agents/documents/design/architecture.md` — C4 model and crate DAG extended here.
- TASK-004 enrichment comment — current-state audit and Laravel core/variant mapping.
