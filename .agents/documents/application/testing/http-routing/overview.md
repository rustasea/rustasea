# Testing: HttpRouting (M1 — Routing & HTTP)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/http-routing/overview.md](../../modules/http-routing/overview.md)
> **BDD:** `@routing`, `@routing-validation`, `@observability-tooling`, `@http-client-process` · **FSD:** FS-M1-01..06 · **FR:** FR-100..109

## 1. Scope

Covers `RouteRegistry` 7-route `resource`, domain precedence, `route:list --json` binding_fields, `Throttle`/`Cors` middleware ( `429` + `trusted_proxies`), `ErrorBag` 422 typed path, and `Http` client `throw`/`Timeout{Idle}` classification. Snapshot `contracts/route-list.schema.json` + `__snapshots__/route-list.json.snap` live here.

## 2. Trace

- Stubs: `testing/stubs/m1-router-http.stub.rs`
- BDD Feature groups: `bdd-scenarios.md §2.2` (4 Features: routing, domain, introspection, Http client).
- QA rows: `qa-design §1.2` M1 positive+negative+decision/boundary.

## 3. Links

- Specs: [test-routing.md](test-routing.md)
- API: [api-routing](../../api/http-routing/api-routing.md) · [api-http-client](../../api/http-routing/api-http-client.md)
- Module docs: [routing](../../modules/http-routing/routing.md) · [middleware](../../modules/http-routing/middleware.md) · [http-client](../../modules/http-routing/http-client.md)
