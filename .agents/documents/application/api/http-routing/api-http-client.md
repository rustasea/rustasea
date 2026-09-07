# API: HttpRouting — Http Client (reqwest wrapper)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §1` · `requirements/prd FR-107..108` · `requirements/fsd FS-M1-06`
> **Crates:** `rustavel-http` · **Module:** [modules/http-routing/http-client.md](../../modules/http-routing/http-client.md)

> **Note:** Client-side contract — not an HTTP endpoint served by Rustavel. Documented here for `test-generation` contract trace.

## 1. Standar Global

- **Library:** `reqwest` under hood; `Http::get(url).header(k,v).timeout(Duration::from_secs(5)).throw(|resp| resp.status().is_server_error()).send().await -> Result<Response, HttpError>`.
- **Timeout kinds:** `Timeout{ kind: Connect|Total|Idle }` distinct (#18). `throw` that itself errors → `HttpError::ThrowCallback`.
- **Process idle-timeout:** `cargo rustavel` internal tooling `FakeInvokedProcess::stop`/`ensureNotTimedOut` adjacency (FS-M1-06).

## 2. Endpoints (client builder)

### 2.1 `Http::get(...).throw(...).timeout(...).send()`

- **URL pattern:** any `url: String`.
- **Deskripsi:** Outbound request with predicate-based throw and timeout classification.
- **Kontrol Akses:** n/a.

#### Params

| Name | Type | Req | Default | Desc | Example |
|------|------|-----|---------|------|---------|
| `url` | string uri | yes | — | target | `https://example.com/api` |
| `header(k,v)` | header pair | no | — | per-request header | `Authorization: Bearer x` |
| `timeout` | Duration 0..60s | no | 30s | `CarbonInterval`-style timeout | `5s` |
| `throw(fn)` | predicate `Fn(Response)->bool` | no | none | throws `HttpError` when predicate true | `\|r\| r.status().is_server_error()` |

#### Response (client)

**Sukses (Ok(Response)):**

```json
{ "status": 200, "headers": { "Content-Type": "application/json" }, "body": { "ok": true } }
```

**Error (HttpError::Status):**

```json
{ "errors": [{ "status": "500", "code": "HttpError::Status", "title": "Server error via throw policy", "detail": "Upstream 500 matched throw(|r| r.status().is_server_error())" }] }
```

**Error (HttpError::Timeout Idle):**

```json
{ "errors": [{ "status": "408", "code": "HttpError::Timeout", "title": "Idle timeout", "detail": "No bytes for 6s (idle_timeout=5s)", "meta": { "kind": "Idle" } }] }
```

**Error (HttpError::ThrowCallback):**

```json
{ "errors": [{ "status": "500", "code": "HttpError::ThrowCallback", "title": "Throw callback failed", "detail": "Predicate itself returned Err." }] }
```

#### Usage (Rust handler calling outbound)

```rust
let resp = Http::get("https://example.com/api")
    .header("Accept", "application/json")
    .timeout(Duration::from_secs(5))
    .throw(|r| r.status().is_server_error())
    .send().await?;
```

#### OpenAPI (client-contract style)

```yaml
openapi: 3.0.3
info:
  title: Rustavel Http Client — outbound contract
  version: 0.1.0
  description: Client builder contract wrapping reqwest; throw predicate + timeout kinds are contract.
x-inferred: true
paths: {}
components:
  schemas:
    HttpError:
      type: object
      properties:
        code: { type: string, example: "HttpError::Timeout" }
        kind: { type: string, enum: [Connect, Total, Idle], example: Idle }
  securitySchemes:
    none: { type: http, scheme: bearer, description: "Not a served API — outbound only" }
security: []
```

## 3. Verification

Contract exercised via `wiremock`/`httpmock` predicates in [testing/http-routing/test-routing.md](../../testing/http-routing/test-routing.md) § 4 (Timeout kinds + throw matrix).

## 4. A-Gate

- [x] Error responses have realistic examples (Status/Timeout Idle/ThrowCallback differentiated).
- [x] YAML valid with `security`.
- [x] Parameter constraints captured (timeout `Duration`, predicate correctness).
