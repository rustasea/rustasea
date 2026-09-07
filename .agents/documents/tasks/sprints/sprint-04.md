# Sprint 04 — M3 Auth, Middleware & Validation

> **Milestone:** M3 · **Window:** 2027-02-15 → 2027-05-15 · **Status:** Planned
> **Parents:** `../roadmap.md` · `prd.md` FR-300–FR-311 · `fsd.md` FS-M3-01–FS-M3-06 · `design/architecture.md`
> **Depends On:** M1 (S02), M2 (S03)
> **Crates:** `rustavel-auth`, `rustavel-validation`

---

## 1. Goal

Complete auth, authorization, and validation with hardened security defaults — so the framework ships with Laravel 13's CSRF/session hardening and strict validation out of the box.

## 2. Scope (In / Out)

**In:**
- JWT guard (`jsonwebtoken`) + session guard (`tower-sessions`): `login`/`loginUsingId`/`parse`/`refresh`/`logout`/`user`/`id`; `Auth::extend` for custom guards; typed `Error::GuardMismatch`; `markEmailAsUnverified`
- Origin-aware CSRF `PreventRequestForgery` (`Sec-Fetch-Site` + token + origin allow-list `config.app.csrf_origins`; `GET`/`HEAD`/`OPTIONS` exempt; missing header degrades to token-only)
- `#[middleware]` + `#[authorize]` attributes
- Rate limiter `limit.perMinute(n).by(ip)` → `Throttle` middleware (keyed buckets)
- CORS via `tower-http`
- Validation: strict `in_array`/`contains`/`doesnt_contain` (no loose equality), `ErrorBag` per form request, `#[validate]` proc-macro wiring `validator` derive rules; `validator` + custom `rustavel-validation`
- Session store JSON default + hyphenated prefixes (`-cache-` / `-session-`) + `serializable_classes` allow-list for cache/session deserialization
- `app/http/middleware/`, `make:middleware`/`make:request` (runtime in S04; CLI wiring in S06)

**Out:**
- Queue/cache/events/schedule specifics — M4 (Sprint 05). AI/broadcast — M6.

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S04-T01 | Guards (JWT + session) + `Auth::extend` + `GuardMismatch` | FR-300, FR-301, FR-311 | FS-M3-01 | `crates/rustavel-auth/src/{guard,jwt,session}.rs` | M | `login(&creds)` → token; `parse(token)` → `user.id`; wrong guard → `GuardMismatch { expected, actual }`; `markEmailAsUnverified` clears `email_verified_at` |
| S04-T02 | Origin-aware CSRF `PreventRequestForgery` | FR-302 | FS-M3-02 | `crates/rustavel-auth/src/csrf.rs` | S | `POST /form` with `Sec-Fetch-Site: cross-site` + invalid origin → `403 CsrfError::UntrustedOrigin`; token-only bypass rejected; `GET` exempt; `csrf-matrix.json` fixture covers spec branches |
| S04-T03 | `#[middleware]` / `#[authorize]` + rate limiter + CORS | FR-305, FR-306, FR-310 | FS-M3-03 | `crates/rustavel-macros/src/{middleware,authorize}.rs` + `crates/rustavel-auth/src/throttle.rs` | M | `#[middleware("auth:jwt")]` unauthenticated → `401`; `limit.per_minute(10).by_ip()` 11th → `429` + `Retry-After`; CORS allow-list enforced; `by_key(fn)` custom key |
| S04-T04 | Validation strict rules + `ErrorBag` + `#[validate]` | FR-307, FR-308, FR-309 | FS-M3-04 | `crates/rustavel-validation/src/{rules,error_bag,validate_macro}.rs` | M | `in_array: [1,"1"]` strict — `1` (int) ≠ `"1"` (str); multi-field errors both in `ErrorBag`; `#[validate] name: length(min=3)` with `"ab"` → validation fails before handler body; 422 JSON shape matches `jsonapi.schema.json` contract |
| S04-T05 | Session hardening (JSON default, hyphenated prefix, allow-list) | FR-303, FR-304 | FS-M3-05 | `crates/rustavel-auth/src/session.rs` + `crates/rustavel-cache` config | S | Cookie `serialization = "json"`; cache prefix contains `-cache-` (not `_cache_`); cached `AdminDto` not in `serializable_classes` → error on deserialize; `allowlist-corpus.json` fuzz passes |

## 4. Dependencies

- **Upstream:** S02 (routing/middleware plumbing) + S03 (users table, ORM) — S04 cannot close until `users` + `personal_access_tokens` migrations are stable.
- **Downstream:** Blocks S05 (M4 — queue `onConnection` auth context, cache hardening relies on session config).

## 5. Deliverables

- Crates `rustavel-auth`, `rustavel-validation`; `app/http/middleware/` + `make:middleware`/`make:request` scaffolds.
- Tag `v0.4.0` with security advisory notes if CSRF/session behavior changed vs `v0.3.0`.

## 6. Acceptance (Sprint Done)

- [ ] JWT `login`→`parse`→`refresh`→`logout` round-trip green; `GuardMismatch` typed error on wrong guard.
- [ ] Cross-site `POST` without valid `Sec-Fetch-Site` → `403`; strict validation rejects loose equality; `ErrorBag` holds multi-field errors.
- [ ] Session cookie uses JSON + hyphenated prefix; allow-list rejects unlisted types (`allowlist-corpus.json` green).
- [ ] `#[middleware]`/`#[authorize]` + `Throttle`/`CORS` tests green; `tower-http` layers verified.
- [ ] Security NFRs NFR-Sec-01 – NFR-Sec-04 green; no `unwrap` in framework crates (`#[deny(clippy::unwrap_used)]`).

## 7. Risks

- R-03 `Sec-Fetch-Site` spec divergence — gate tests on spec version; keep token as primary gate; missing header path tested.
