# API: IdentityAccess — Validation (`422 ErrorBag`)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §3` · `requirements/prd FR-307..309` · `requirements/fsd FS-M3-05..06` · `requirements/tdd BC-3 Validatable/ErrorBag`
> **Crates:** `rustavel-validation` (`#[validate]`) · `rustavel-macros` · **BDD:** `@attributes`, `@routing-validation` / `@cache-session-hardening` adjacency

## 1. Standar Global

- **Sits on:** every `POST`/`PUT`/`PATCH` handler with `#[validate]` vs `Validatable` trait — e.g., `POST /users` (see [api-routing](../../api/http-routing/api-routing.md) §2.3) returns `201` on valid else `422`.
- **Content-Type:** `application/json` for `ErrorBag`.
- **Validation engine:** `validator` derive + `rustavel-validation` strict helpers (`in_array`/`contains`/`doesnt_contain` are type+value).

## 2. Endpoint Contract (via `POST /users` as example)

- **URL:** `POST /users` (and any `POST/PATCH ...` that declares `#[validate]`).
- **Deskripsi:** See `api-routing` §2.3 Create + `modules/identity-access/validation.md`. This doc focuses on the `422 ErrorBag` contract shape used across all validating endpoints.

### 2.1 Request Body — `CreateUser` (strict rules)

| Field | Type | Req | Validation | Desc | Example |
|-------|------|-----|------------|------|---------|
| `name` | string length≥3 | yes | `length(min=3)` | trimmed | `"Ada"` |
| `email` | string email | yes | `email` | strict | `"ada@example.com"` |
| `role` | string | no | `contains_strict="admin"` | type+value (`"1"!=1`) | `"admin"` |

```json
{ "data": { "type": "users", "attributes": { "name": "ab", "email": "not-an-email", "role": "Admin" } } }
```

### 2.2 Response

**Sukses (201 / 200 depending on resource):**

```json
{ "data": { "type": "users", "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "attributes": { "name": "Ada", "email": "ada@example.com" } } }
```

**Error (422 — ErrorBag — multi-field keys, multiple errors per field coexist):**

```json
{
  "message": "The given data was invalid.",
  "errors": {
    "name": ["The name must be at least 3 characters."],
    "email": ["The email must be a valid email address."],
    "role": ["The role must contain admin strictly."]
  }
}
```

### 2.3 `#[validate]` failure as 422 matrix (strict)

| Input | Rule | Result |
|-------|------|--------|
| `role="Admin"` | `contains_strict="admin"` | invalid (case mismatch) |
| `identifier="1"` (string) | `in_array:[1,2,3]` (int) | invalid (type mismatch) |
| `identifier=1` (int) | `in_array:[1,2,3]` | valid |

## 3. OpenAPI 3.0 Snippet (ErrorBag schema)

```yaml
openapi: 3.0.3
info:
  title: Rustavel Validation — ErrorBag 422
  version: 0.1.0
  description: ErrorBag contract shared by every #[validate] endpoint
paths:
  /users:
    post:
      summary: Create user (validation probe)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
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
                        role: { type: string, example: "admin" }
      responses:
        '201':
          description: Created
          content:
            application/json:
              example:
                data:
                  type: users
                  id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1
                  attributes: { name: Ada, email: ada@example.com }
        '422':
          description: Validation ErrorBag
          content:
            application/json:
              schema:
                type: object
                required: [message, errors]
                properties:
                  message: { type: string, example: The given data was invalid. }
                  errors:
                    type: object
                    additionalProperties:
                      type: array
                      items: { type: string }
                    example:
                      email: ["must be a valid email address."]
                      password: ["must be at least 8 characters."]
              example:
                message: The given data was invalid.
                errors:
                  email: ["The email must be a valid email address."]
                  password: ["The password must be at least 8 characters."]
                  role: ["The role must contain admin strictly."]
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
security: []
```

## 4. Error Catalogue

| HTTP | Code | When |
|------|------|------|
| 422 | `ErrorBag` | strict `contains_strict`/`in_array` fail, `validator` length/email fail |
| 401 | `AuthError::*` | when behind `#[middleware("auth:jwt")]` similarly (see `api-auth`) |
| 429 | `Throttle` | adjacency when rate-limited (pair with `api-routing` throttle) |

## 5. Usage

```bash
# should fail: name too short + email invalid → 422 ErrorBag with both keys
curl -s -X POST http://localhost:3000/users \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer '"$JWT" \
  -d '{"data":{"type":"users","attributes":{"name":"ab","email":"not-an-email"}}}' | jq .

# strict contains_strict
curl -s -X POST http://localhost:3000/users \
  -H 'Content-Type: application/json' -H 'Authorization: Bearer '"$JWT" \
  -d '{"data":{"type":"users","attributes":{"name":"Ada","email":"ada@example.com","role":"Admin"}}}' | jq .
# -> 422 role contains_strict mismatch
```

## 6. Cross-References

- Module: [validation.md](../../modules/identity-access/validation.md)
- HTTP: [api-routing](../../api/http-routing/api-routing.md) §2.3 (429/201 surface uses same handler)
- Auth: [api-auth](../../api/identity-access/api-auth.md) (CSRF + guard `401`)

## 7. A-Gate

- [x] 422 body with `message` + `errors[field]` multi-field keys.
- [x] YAML valid OpenAPI 3.0, constraints (`minLength`, `format: email`), example.
- [x] curl valid.
