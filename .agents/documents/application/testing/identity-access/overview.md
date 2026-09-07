# Testing: IdentityAccess (M3 — Auth, CSRF, Validation)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/identity-access/overview.md](../../modules/identity-access/overview.md)
> **BDD:** `@auth`, `@csrf-origin`, `@cache-session-hardening`, `@attributes`, `@throttle` · **FSD:** FS-M3-01..06

## 1. Scope

Covers JWT+session `Auth::extend` + `GuardMismatch`, origin-aware CSRF `Sec-Fetch-Site` matrix (6 rows) + JSON serialization allow-list + hyphenated `-cache-` prefix + `argon2`, strict `#[validate]` + `ErrorBag` 422, rate limiter `limit.perMinute(n).by(ip)` + `trusted_proxies`, and declarative `#[middleware]`/`#[authorize]`/`#[validate]`.

## 2. Trace

- Stubs: `testing/stubs/m3-auth-validation.stub.rs`
- Contracts: `testing/contracts/jwt-claims.schema.json`, `testing/fixtures/csrf-matrix.json` (6-row decision table), `testing/fixtures/allowlist-corpus.json`.
- BDD: `bdd-scenarios.md §2.4` (4 Features).

## 3. Links

- Specs: [test-auth-validation.md](test-auth-validation.md)
- API: [api-auth](../../api/identity-access/api-auth.md) · [api-validation](../../api/identity-access/api-validation.md)
- Modules: [auth](../../modules/identity-access/auth.md) · [csrf](../../modules/identity-access/csrf.md) · [validation](../../modules/identity-access/validation.md)
