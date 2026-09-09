# Sprint 02 — M1 Routing & HTTP

> **Milestone:** M1 · **Window:** 2026-11-15 → 2027-02-15 · **Status:** Planned
> **Parents:** `../roadmap.md` · `prd.md` FR-100–FR-109 · `fsd.md` FS-M1-01–FS-M1-06 · `design/architecture.md`
> **Depends On:** M0 (Sprint 01)
> **Crates:** `rustasea-router`, `rustasea-http` (uses `rustasea-foundation`, `rustasea-macros`)

---

## 1. Goal

Expressive HTTP layer with routing, middleware, typed extractors, and introspection — so handlers read like Laravel routes and are observable via `route:list`.

## 2. Scope (In / Out)

**In:**
- `axum`-backed method helpers `get`/`post`/`put`/`delete`/`patch`/`options`/`any` + `#[route]` proc-macro
- Groups (prefix, name prefix, middleware stacking) + `resource` helper (`index/create/store/show/edit/update/destroy`)
- Domain-aware routing — domain routes prioritized, catch-all does not shadow non-domain
- `cargo rustasea route:list` (table + `--json` with `binding_fields`)
- `ModelInspector`-compatible metadata for `show:model`
- Middleware stack `throttle`/`cors`/`TrimStrings` via `tower`/`tower-http` (keyed buckets `by_ip`/`by_user`)
- Typed extractors `Json<T>`/`Query<T>`/`Path<T>`/`State<AppState>` with `validator` → `ErrorBag` 422
- Typed responses `Json`/`View` (`askama`/`minijinja`) + correct `Content-Type`
- HTTP client `reqwest` wrapper (`throw` callbacks, retry, timeouts including idle timeout) + `FakeInvokedProcess` testing helpers
- Routes file `routes/web.rs`, `app/http/middleware/`

**Out:**
- ORM, auth/JWT/CSRF hardening (M2/M3); queue/cache (M4); generators `make:controller` etc. are M5 (Sprint 06) — S02 provides the runtime only.

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S02-T01 | Router core: methods, groups, `resource`, slash normalization, conflict detection | FR-100, FR-101 | FS-M1-01 | `crates/rustasea-router/src/{route,group,resource}.rs` + `routes/web.rs` | M | `Route::get` → 200; group prefix concatenated; `resource("users", UserController)` expands 7 routes; duplicate named route → `RouteError::Conflict`; `OPTIONS` auto-added for `any` |
| S02-T02 | Domain-aware routing (host extraction, precedence) | FR-102 | FS-M1-02 | `crates/rustasea-router/src/domain.rs` | S | `*.example.com/*` vs `/docs` — domain route wins; `Host`/`Forwarded` parsed; ambiguous domain → `RoutingError::AmbiguousDomain` |
| S02-T03 | `route:list` + `show:model` inspector metadata | FR-103, FR-109 | FS-M1-03 | `crates/rustasea-router/src/inspector.rs` + `cargo xtask route:list` | S | `route:list --json` emits `method/path/name/middleware[]/binding_fields[]`; `{user:slug}` shows `["slug"]`; `show:model User` lists attributes/relations/casts (FR-109) |
| S02-T04 | Middleware stack (`throttle`, `cors`, etc.) via `tower-http` | FR-104 | FS-M1-04 | `crates/rustasea-http/src/middleware/{throttle,cors}.rs` | M | `throttle(60, per_minute).by(ip)` → 61st → 429 + `Retry-After`; CORS allow-list emits `Access-Control-Allow-Origin` only for listed origins; trusted-proxy `X-Forwarded-For` documented |
| S02-T05 | Typed extractors & responses (`Json`/`Query`/`Path`/`View`) | FR-105, FR-106 | FS-M1-05 | `crates/rustasea-http/src/{extract,response}.rs` | S | `Json(CreateUser)` invalid → 422 `ErrorBag` JSON with field keys; `Json(user)` → `Content-Type: application/json`; `View` renders askama/minijinja |
| S02-T06 | HTTP client (`reqwest` wrapper: `throw`, timeouts, idle timeout) | FR-107, FR-108 | FS-M1-06 | `crates/rustasea-http/src/client.rs` + `FakeInvokedProcess` | M | `.throw(\|r\| r.status().is_server_error())` → `HttpError::Status`; idle timeout >5s → `Timeout { kind: Idle }`; `stop`/`ensureNotTimedOut` on fake |

## 4. Dependencies

- **Upstream:** S01 (M0) — `Application`, `AppState`, config — must be tagged `v0.1.0` before close; S02 may start scaffolding in parallel once `rustasea-foundation` is stable.
- **Downstream:** Blocks S03 (M2), S04 (M3), S07 (M6). `axum` choice locked by ADR-001.

## 5. Deliverables

- Crates `rustasea-router`, `rustasea-http` publishable; `#[route]` macro in `rustasea-macros`.
- `routes/web.rs`, `route:list` CLI, `app/http/middleware/` directory shape.
- Tag `v0.2.0` with migration notes.

## 6. Acceptance (Sprint Done)

- [ ] `Route::get("/users", [UserController, "index"])` equivalent returns `200` over wire (`cargo test` HTTP assertions).
- [ ] `cargo rustasea route:list --json` shows `binding_fields` for `{user:slug}`; human table renders without `--json`.
- [ ] Domain catch-all does not shadow non-domain routes (deterministic precedence test green).
- [ ] `throttle` + `cors` middleware tests green; extractors return `422 ErrorBag` on validation failure.
- [ ] HTTP client `throw` + `timeout` (including idle) tests green.
- [ ] CI + incremental adoption: `cargo check -p rustasea-router` does not pull `sqlx`/`async-openai`.

## 7. Risks & Notes

- Overlap with S01 is intentional — if M0 shutdown semantics slip, S02 router core is still unblockable; coordinate `AppState` shape early.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
