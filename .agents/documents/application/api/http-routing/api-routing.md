# API: HttpRouting — Routes, Middleware, Introspection

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §1` · `requirements/prd FR-100..106, FR-102..103, FR-109` · `requirements/fsd FS-M1-01..05` · `requirements/tdd BC-1` · `architecture.md BC-1` · ADR-001
> **Crates:** `rustavel-router` · `rustavel-http` · `rustavel-macros`
> **Module:** [modules/http-routing/overview.md](../../modules/http-routing/overview.md) · [routing](../../modules/http-routing/routing.md) · [middleware](../../modules/http-routing/middleware.md) · **Testing:** [testing/http-routing/overview.md](../../testing/http-routing/overview.md)

## 1. Standar Global

- **Base URL:** `http://localhost:3000` — version prefix `/api/v1` only when the route is versioned (see §2.1 resource examples). Unversioned `/users` routes are documented without `/api/v1` to preserve realistic paths from `api-contracts.md §1`.
- **Content-Type:** `application/vnd.api+json` for collection resources when served via `Json<T>` over a Paginated envelope; `application/json` for `ErrorBag`.
- **Accept:** `application/vnd.api+json` or `application/json`.
- **Format Tanggal:** RFC3339 `YYYY-MM-DDTHH:mm:ssZ`.
- **Auth default:** Optional per route — `Bearer JWT` on protected routes; `401` on miss. Throttle is per-middleware.

## 2. Endpoints

### 2.1 `GET /users` — List (resource `index`)

- **URL:** `GET /users` (when `Route::resource("users", UserController)`; group `prefix("/api/v1")` yields `GET /api/v1/users`)
- **Deskripsi:** Paginated user listing. Demonstrates group prefix + `Paginated<T>` envelope (shared with ORM/JSON:API).
- **Kontrol Akses:** `Bearer JWT` optional (when `#[middleware("auth:jwt")]` on resource).

#### Request

**Headers:**

```http
Accept: application/vnd.api+json
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIn0.xxx
```

**Query Params:**

| Name | Type | Req | Default | Desc | Example |
|------|------|-----|---------|------|---------|
| `page` | integer 1..1000 | no | 1 | current page | `2` |
| `per_page` | integer 1..100 | no | 15 | items per page | `15` |
| `include` | string | no | — | eager relationships (future) | `posts` |

#### Response

**Sukses (200):**

```json
{
  "data": [{ "type": "users", "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "attributes": { "name": "Ada", "email": "ada@example.com" } }],
  "meta": { "current_page": 1, "per_page": 15, "total": 42, "last_page": 3 },
  "links": { "first": "/users?page=1", "prev": null, "next": "/users?page=2", "last": "/users?page=3" }
}
```

**Error (401 — auth middleware when protected):**

```json
{ "errors": [{ "status": "401", "code": "AuthError::InvalidToken", "title": "Unauthorized", "detail": "Missing or invalid bearer token for guard jwt." }] }
```

**Error (429 — throttle middleware):**

```json
{ "errors": [{ "status": "429", "code": "Throttle", "title": "Too Many Requests", "detail": "Rate limit per_minute:60 exceeded for ip 1.2.3.4." }], "meta": { "retry_after": 37 } }
```

#### Usage

```bash
curl -s http://localhost:3000/users?page=1 \
  -H 'Accept: application/vnd.api+json' \
  -H 'Authorization: Bearer '"$JWT" | jq .
```

#### Notes

- `resource("users", UserController)` expands to 7 routes (`index/create/store/show/edit/update/destroy`). This endpoint is `index`.
- Duplicate `name("users.index")` → `RouteError::Conflict` at boot (not an HTTP status — boot fails).

### 2.2 `GET /users/{user:slug}` — Show with binding field

- **URL:** `GET /users/{user:slug}` — note `binding_fields: ["slug"]` in `route:list --json`.
- **Deskripsi:** Single user by `slug` via implicit binding.
- **Kontrol Akses:** Optional throttle `throttle:60,1`.

#### Params

| Name | Type | Req | Default | Desc | Example |
|------|------|-----|---------|------|---------|
| `user` | string slug `[a-z0-9-]+` length 3..50 | yes | — | binding field `slug` | `ada-lovelace` |

#### Response

**Sukses (200):**

```json
{ "data": { "type": "users", "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "attributes": { "name": "Ada", "email": "ada@example.com" } } }
```

**Error (404):**

```json
{ "errors": [{ "status": "404", "code": "QueryError::NotFound", "title": "Not Found", "detail": "No user matches the given id for slug ada-lovelace." }] }
```

#### Usage

```bash
curl -s http://localhost:3000/users/ada-lovelace -H 'Accept: application/vnd.api+json' | jq .
```

### 2.3 `POST /users` — Create (requires validation → `ErrorBag` 422)

- **URL:** `POST /users`
- **Deskripsi:** Creates a user via `#[validate]` strict rules (`email`, `length(min=3)`) → `ErrorBag` keyed by field.
- **Kontrol Akses:** `Bearer JWT` + `throttle:60,1` when declared on handler.

#### Body

**Content-Type:** `application/json`

| Field | Type | Req | Desc | Example |
|-------|------|-----|------|---------|
| `name` | string min 3 | yes | trimmed by middleware | `"Ada"` |
| `email` | string email | yes | validated strictly | `"ada@example.com"` |
| `role` | string `contains_strict="admin"` | no | strict `contains_strict` — `"Admin" != "admin"` | `"admin"` |

```json
{ "data": { "type": "users", "attributes": { "name": "Ada", "email": "ada@example.com" } } }
```

#### Response

**Sukses (201):**

```json
{ "data": { "type": "users", "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "attributes": { "name": "Ada", "email": "ada@example.com" } } }
```

**Error (422 — ErrorBag):**

```json
{
  "message": "The given data was invalid.",
  "errors": {
    "email": ["The email must be a valid email address."],
    "password": ["The password must be at least 8 characters."]
  }
}
```

#### Usage

```bash
curl -s -X POST http://localhost:3000/users \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'Authorization: Bearer '"$JWT" \
  -d '{"data":{"type":"users","attributes":{"name":"Ada","email":"not-an-email"}}}' | jq .
# -> 422 with errors.email
```

### 2.4 Introspection — `cargo rustavel route:list [--json]` (not a wire endpoint)

This spec documents the **machine output contract** (snapshot-guarded) — `api-contracts.md §1` defines the shape.

**Machine output (`--json`):**

```json
[
  {
    "method": "GET",
    "path": "/users/{user:slug}",
    "name": "users.show",
    "middleware": ["throttle:60,1", "auth:jwt"],
    "binding_fields": ["slug"]
  }
]
```

Validated via `contracts/route-list.schema.json` + `cargo insta` snapshot `__snapshots__/route-list.json.snap`.

## 3. OpenAPI 3.0 Snippet (valid YAML)

```yaml
openapi: 3.0.3
info:
  title: Rustavel HttpRouting
  version: 0.1.0
  description: M1 routes — inferred from design/api-contracts.md §1
servers:
  - url: http://localhost:3000
    description: dev
paths:
  /users:
    get:
      summary: List users (Paginated)
      security:
        - bearerAuth: []
      parameters:
        - name: page
          in: query
          schema: { type: integer, minimum: 1, example: 1 }
        - name: per_page
          in: query
          schema: { type: integer, minimum: 1, maximum: 100, example: 15 }
      responses:
        '200':
          description: Paginated users
          content:
            application/vnd.api+json:
              example:
                data:
                  - type: users
                    id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1
                    attributes:
                      name: Ada
                      email: ada@example.com
                meta: { current_page: 1, per_page: 15, total: 42, last_page: 3 }
                links: { first: "/users?page=1", next: "/users?page=2", last: "/users?page=3" }
        '401':
          description: Unauthorized
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '401'
                    code: AuthError::InvalidToken
                    title: Unauthorized
        '429':
          description: Too Many Requests
          headers:
            Retry-After:
              schema: { type: integer, example: 37 }
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '429'
                    code: Throttle
                    title: Too Many Requests
        '422':
          description: Validation ErrorBag (see §2.3)
          content:
            application/json:
              example:
                message: The given data was invalid.
                errors:
                  email: ["must be a valid email"]
    post:
      summary: Create user
      security:
        - bearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [data]
              properties:
                data:
                  type: object
                  properties:
                    type: { type: string, example: users }
                    attributes:
                      type: object
                      required: [name, email]
                      properties:
                        name: { type: string, minLength: 3, example: Ada }
                        email: { type: string, format: email, example: ada@example.com }
      responses:
        '201':
          description: Created
          content:
            application/vnd.api+json:
              example:
                data:
                  type: users
                  id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1
                  attributes:
                    name: Ada
                    email: ada@example.com
        '422':
          $ref: '#/paths/~1users/get/responses/422'
  /users/{user}:
    get:
      summary: Show user by slug binding
      parameters:
        - name: user
          in: path
          required: true
          schema: { type: string, pattern: "^[a-z0-9-]+$", minLength: 3, maxLength: 50, example: ada-lovelace }
      responses:
        '200':
          description: User
          content:
            application/vnd.api+json:
              example:
                data:
                  type: users
                  id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1
                  attributes: { name: Ada, email: ada@example.com }
        '404':
          description: Not Found
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '404'
                    code: QueryError::NotFound
                    title: Not Found
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    RouteEntry:
      type: object
      properties:
        method: { type: string, example: GET }
        path: { type: string, example: "/users/{user:slug}" }
        name: { type: string, example: users.show }
        middleware: { type: array, items: { type: string }, example: ["throttle:60,1", "auth:jwt"] }
        binding_fields: { type: array, items: { type: string }, example: ["slug"] }
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 201 | — | `store` (`POST /users`) |
| 401 | `InvalidToken` / `ExpiredToken` / `GuardMismatch` | `#[middleware("auth:jwt")]` unauthenticated |
| 404 | `QueryError::NotFound` | slug not found |
| 409 | `RouteError::Conflict` | duplicate `{method,path}` or `name` at boot (not wire) |
| 422 | `ErrorBag` | `#[validate]` strict failure |
| 429 | `Throttle` | `per_minute(60)` exceeded; header `Retry-After` |

## 5. Cross-References

- Module: [routing.md](../../modules/http-routing/routing.md) · [middleware.md](../../modules/http-routing/middleware.md) · [http-client.md](../../modules/http-routing/http-client.md)
- Design: `api-contracts.md §1`, `tdd.md BC-1`, ADR-001
- Testing: [testing/http-routing/test-routing.md](../../testing/http-routing/test-routing.md) · `contracts/route-list.schema.json`

## 6. Skill Reference

| Layer | Skill |
|-------|-------|
| API | `technical-documentation` Part A |
| Contract | `test-generation` — `route-list.schema.json` + `cargo insta` |
| Security | `security-audit` — CORS suffix trick, `X-Forwarded-For` spoof |
| Chaos | `non-functional-testing` — catch-all under `oha` burst |

## 7. A-Gate

- [x] Error responses have realistic examples.
- [x] YAML valid OpenAPI 3.0 with `security`, constraints (min/max/pattern), examples.
- [x] curl uses valid CLI with headers.

## 8. Chaos & Resilience Note

`throttle` row deleted mid-429 window → window reset (NFR-Rel) is tested via `cargo test` burst; SSE adjacency is in [api-broadcast](../../api/intelligence-delivery/api-broadcast.md).

