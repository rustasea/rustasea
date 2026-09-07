# Skenario Pengujian: HttpRouting (HTTP)

> Skenario pengujian untuk fitur HttpRouting — Routing, Middleware, Http Client.
> Bundles `test-planning/qa-design`, `test-generation/bdd-gherkin` + `api-contract-test`, `security-audit`, `non-functional-testing/chaos`.

## Header & Navigation

- [Module Overview](../../modules/http-routing/overview.md)
- [API Specification](../../api/http-routing/api-routing.md) · [Http Client](../../api/http-routing/api-http-client.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| HTTP-POS-001 | US-M1-01 | Resource 7 routes | `Route::resource("users", UserController)` | `route:list --json` | 7 routes including `index` + `show` | High |
| HTTP-POS-002 | US-M1-01 | Group prefix applied | group `prefix("/api/v1")` with member `get("/users")` | `route:list` path | `/api/v1/users` | High |
| HTTP-POS-003 | US-M1-02 | Tenant catch-all wins | `*.example.com/*` + `/docs` both match `docs.example.com/docs` | request host `docs.example.com` | tenant handler, `tenant="docs"` | High |
| HTTP-POS-004 | US-M1-03 | Route list binding_fields | route `get("/users/{user:slug}")` with slug | `route:list --json` entry for that path | `binding_fields=["slug"]` | High |
| HTTP-POS-005 | US-M1-04 | Throttling 60 inclusive | route `throttle(per_minute:60).by_ip()` | 61 requests same `1.2.3.4` in 60s | 61st `429` + `Retry-After` | High |
| HTTP-POS-006 | US-M1-04 | CORS allow-list lets pass | `Cors::allow_origins(["https://app.example.com"])` | preflight with that Origin | `Access-Control-Allow-Origin` present | High |
| HTTP-POS-007 | US-M1-05 | Valid payload passes | handler `create(Json(CreateUser{name,email}))` | `{name:" Ada ",email:"ada@example.com"}` | `201` serialized user | High |
| HTTP-POS-008 | US-M1-06 | throw on 5xx fires | upstream `500` mocked | `Http::get(url).throw(\|r\| r.status().is_server_error())` | `HttpError::Status{500}` | High |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| HTTP-NEG-001 | US-M1-01 | Duplicate named route rejected | two routes both `name("users.index")` | router `build()` | `RouteError::Conflict{ name: "users.index"}` | High |
| HTTP-NEG-002 | US-M1-05 | Unknown field strict rejects | handler expects `CreateUser` | `{name:"Ada",email:"ada@example.com",age:30}` | `422` errors `age` unexpected | High |
| HTTP-NEG-003 | US-M1-05 | Invalid email is ErrorBag per field | handler with email validator | `{email:"not-an-email"}` | `422` + `errors.email[]` | High |
| HTTP-NEG-004 | US-M1-05 | Strict role type mismatch | handler `contains_strict="admin"` | `role="Admin"` or `role=1` | `422` validation fail | High |
| HTTP-NEG-005 | US-M1-04 | Disallowed origin CORS no header | `Cors::allow_origins([...])` same | `Origin: https://evil.com` | no `Access-Control-Allow-Origin` (or `403` strict) | Medium |
| HTTP-NEG-006 | US-M1-02 | Non-domain fallback serves | domain + non-domain `/dashboard` | `example.com/dashboard` (no subdomain) | non-domain handler responds | Medium |
| HTTP-NEG-007 | US-M1-06 | throw callback that itself errors | predicate returns `Err` | `Http::get.throw(...)` | `HttpError::ThrowCallback` | Medium |
| HTTP-NEG-008 | US-M1-06 | idle vs total timeout distinguished | upstream 6s silence vs 6s wall | `idle_timeout=5s` | `Timeout{kind: Idle}` not `Total` | High |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| HTTP-MNK-001 | Burst after throttle window | burst 61→ window reset → 60 pass (boundary N) | throttle 60/min | 61st 429 then window reset allows rest |
| HTTP-MNK-002 | Domain catch-all under load | 10k `docs.example.com/docs` contenders vs `*.example.com/*` | domain split | deterministic precedence (no flip) |
| HTTP-MNK-003 | Http upstream 500 burst | `docker pause` upstream → 500 burst via `throw` → not-false success | throw predicate `500 always` vs `404 server_error→success` | correct classification per `bdd-scenarios §2.2` table 500/404/422 |
| HTTP-MNK-004 | route:list snapshot drift | `cargo insta` snapshot vs JSON-Schema | `contracts/route-list.schema.json` + `__snapshots__` | `cargo insta review` passes |
| HTTP-MNK-005 | Idle timeout vs response complete | `wiremock` 6s silence then 200 | `timeout 5s Idle` | `Timeout{Idle}` not success |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| HTTP-SEC-001 | Attacker | `X-Forwarded-For: 9.9.9.9` spoof via `1.2.3.4` peer | rate limit check when `trusted_proxies` unset | checked identity `1.2.3.4` not spoofed (see `bdd-scenarios §2.4` forwarded identity) |
| HTTP-SEC-002 | Attacker | CORS suffix trick `https://evil-app.example.com` vs `https://app.example.com` | preflight with suffix evil | not allowed (exact-match allow-list) |
| HTTP-SEC-003 | Auditor | Contract `RouteEntry` binding_fields injection | path `/users/{user:slug OR 1=1}` | `InvalidPattern` via `RouteError::InvalidPattern` |
| HTTP-SEC-004 | Client | Empty prefix normalization | `prefix("/api/v1")` empty variation + trailing slash | slash-normalized canonical path invariant |

