# API: IdentityAccess — Auth + CSRF

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §3` · `requirements/prd FR-300..311` · `requirements/fsd FS-M3-01..06` · `requirements/tdd BC-3`
> **Crates:** `rustavel-auth` · `rustavel-validation` · · **BDD:** `@auth`, `@csrf-origin`, `@cache-session-hardening`, `@attributes`, `@throttle`
> **Module:** [modules/identity-access/overview.md](../../modules/identity-access/overview.md) · **Testing:** [testing/identity-access/overview.md](../../testing/identity-access/overview.md)

## 1. Standar Global

- **Base URL:** `http://localhost:3000`
- **Content-Type:** `application/json` (auth + validation `422`), JSON:API not required for wire-auth (design decision `api-contracts.md §3`).
- **Auth:** `Authorization: Bearer <JWT>` (HS256). Missing/invalid/expired → `401`typed `InvalidToken`/`ExpiredToken`.
- **Format Tanggal:** RFC3339.

## 2. Endpoints

### 2.1 `POST /login` — JWT login

- **URL:** `POST /login`
- **Deskripsi:** Verifies `argon2` password, issues `access_token`+`refresh_token`. Pairs with `tower-sessions` session guard path not duplicated here.
- **Kontrol Akses:** Public (rate-limited `throttle:60,1` recommended via `#[middleware("throttle:60,1")]`).

#### Body

**Content-Type:** `application/json`

| Field | Type | Req | Default | Desc | Example |
|-------|------|-----|---------|------|---------|
| `email` | string email | yes | — | user email | `"ada@example.com"` |
| `password` | string min 8 | yes | — | plaintext to verify | `"s3cr3tPass"` |

```json
{ "email": "ada@example.com", "password": "s3cr3tPass" }
```

#### Response

**Sukses (200):**

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJiMmU4ZjNjMC05YTFkLTRmNmItOGMyZS0xZDNmYTliN2U1YzEiLCJleHAiOjE3MjYwMzI4MDB9.xxx",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJiMmU4ZjNjMCIsImV4cCI6MTcyNjExOTIwMH0.yyy",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

**Error (401 BadCredentials):**

```json
{ "errors": [{ "status": "401", "code": "AuthError::BadCredentials", "title": "Bad credentials", "detail": "Password does not match stored argon2 hash." }] }
```

#### Usage

```bash
curl -s -X POST http://localhost:3000/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"ada@example.com","password":"s3cr3tPass"}' | jq .
JWT=$(curl -s -X POST http://localhost:3000/login -H 'Content-Type: application/json' -d '{"email":"ada@example.com","password":"s3cr3tPass"}' | jq -r .access_token)
```

### 2.2 `POST /loginUsingId` — Testing/internal helper

- **URL:** `POST /loginUsingId`
- **Deskripsi:** Issues token for `user_id: UUID` without password. Guard `allow_loginUsingId` config gates it (internal/testing only — prod should return `403`).
- **Kontrol Akses:** Testing only.

#### Body

| Field | Type | Req | Desc | Example |
|-------|------|-----|------|---------|
| `user_id` | string uuid v4 | yes | target user | `b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1` |

#### Response

**Sukses (200):** same token pair as §2.1.

**Error (403 when disabled in prod):**

```json
{ "errors": [{ "status": "403", "code": "GuardMismatch", "title": "Forbidden", "detail": "loginUsingId is disabled in production." }] }
```

### 2.3 `POST /refresh` — Rotate tokens

- **URL:** `POST /refresh`
- **Deskripsi:** Rotates a valid (even near-expired) `Bearer` into a new pair.
- **Kontrol Akses:** `Bearer JWT` required.

#### Headers

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `Authorization` | `Bearer <token>` | yes | current access token | `Bearer eyJ...` |

#### Response

**Sukses (200):** new `access_token`+`refresh_token`+`expires_in`.

**Error (401 ExpiredToken / InvalidToken):**

```json
{ "errors": [{ "status": "401", "code": "AuthError::ExpiredToken", "title": "Token expired", "detail": "Claim exp in past (1726032800)." }] }
```

#### Usage

```bash
curl -s -X POST http://localhost:3000/refresh -H 'Authorization: Bearer '"$JWT" | jq .
```

### 2.4 `POST /logout` — Invalidate session

- **URL:** `POST /logout`
- **Deskripsi:** Invalidates JWT (via deny-list / session store depending on guard).
- **Kontrol Akses:** `Bearer JWT`.

#### Response

**Sukses (204):** No body.

**Error (401 InvalidToken):** as above.

#### Usage

```bash
curl -i -X POST http://localhost:3000/logout -H 'Authorization: Bearer '"$JWT"
# HTTP/1.1 204 No Content
```

### 2.5 `GET /me` — Identity probe

- **URL:** `GET /me`
- **Deskripsi:** Returns authenticated identity; typed guard mismatch test: `Auth::guard("api")` where only `jwt` registered → `GuardMismatch`.
- **Kontrol Akses:** `Bearer JWT` (or session cookie on session guard).

#### Response

**Sukses (200):**

```json
{ "data": { "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "email": "ada@example.com" } }
```

**Error (401 InvalidToken/ExpiredToken/GuardMismatch):**

```json
{ "errors": [{ "status": "401", "code": "AuthError::GuardMismatch", "title": "Guard mismatch", "detail": "GuardMismatch { expected: \"jwt\", actual: \"api\" }" }] }
```

#### Usage

```bash
curl -s http://localhost:3000/me -H 'Authorization: Bearer '"$JWT" | jq .
```

### 2.6 CSRF-protected `POST /form` — Origin-aware forgery protection

- **URL:** `POST /form` — any write verb (`POST`/`PUT`/`DELETE`/`PATCH`) behind `PreventRequestForgery`.
- **Deskripsi:** Token checked first; then `Sec-Fetch-Site` cross-site → origin must be in `config.app.csrf_origins`. Missing `Sec-Fetch-Site` (older browsers) degrades to token-only. `GET`/`HEAD`/`OPTIONS` exempt. See `identity-access/csrf.md` + `testing/fixtures/csrf-matrix.json`.
- **Kontrol Akses:** `Bearer` or session cookie + valid `X-CSRF-TOKEN`/`_token` + `Sec-Fetch-Site` matrix.

#### Headers

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `X-CSRF-TOKEN` | string | yes* | forgery token (*exempt for safe methods) | `abc123` |
| `Sec-Fetch-Site` | enum `same-origin`/`cross-site`/`none` | no | Fetch Metadata — `none` treated as `same-origin` | `same-origin` |
| `Origin` | string origin | yes* | checked only when `Sec-Fetch-Site: cross-site` | `https://app.example.com` |

#### Response

**Sukses (200) — same-origin + valid token:**

```json
{ "ok": true }
```

**Error (403 UntrustedOrigin) — cross-site invalid origin + valid token:**

```json
{
  "errors": [{
    "status": "403",
    "code": "CsrfError::UntrustedOrigin",
    "title": "Untrusted origin",
    "detail": "Origin https://evil.com not in allow-list https://app.example.com.",
    "meta": { "sec_fetch_site": "cross-site", "origin": "https://evil.com" }
  }]
}
```

#### Usage

```bash
# same-origin (pass)
curl -s -X POST http://localhost:3000/form \
  -H 'X-CSRF-TOKEN: abc123' -H 'Sec-Fetch-Site: same-origin' \
  -H 'Authorization: Bearer '"$JWT" -d '{}' | jq .

# cross-site untrusted (403)
curl -s -X POST http://localhost:3000/form \
  -H 'X-CSRF-TOKEN: abc123' -H 'Sec-Fetch-Site: cross-site' \
  -H 'Origin: https://evil.com' -H 'Authorization: Bearer '"$JWT" -d '{}' | jq .

# missing Sec-Fetch-Site degrades to token-only (pass with valid token)
curl -s -X POST http://localhost:3000/form \
  -H 'X-CSRF-TOKEN: abc123' -H 'Authorization: Bearer '"$JWT" -d '{}' | jq .
```

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: Rustavel Identity — Auth + CSRF
  version: 0.1.0
  description: Auth guards + origin-aware CSRF per api-contracts.md §3
servers:
  - url: http://localhost:3000
paths:
  /login:
    post:
      summary: Login (JWT)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [email, password]
              properties:
                email: { type: string, format: email, example: ada@example.com }
                password: { type: string, minLength: 8, example: s3cr3tPass }
      responses:
        '200':
          description: Token pair
          content:
            application/json:
              example:
                access_token: eyJhbGciOiJIUzI1NiJ9.xxx
                refresh_token: eyJhbGciOiJIUzI1NiJ9.yyy
                token_type: Bearer
                expires_in: 3600
        '401':
          description: Bad credentials
          content:
            application/json:
              example:
                errors:
                  - status: '401'
                    code: AuthError::BadCredentials
                    title: Bad credentials
        '429':
          description: Too Many Requests (throttle adjacency)
          headers:
            Retry-After:
              schema: { type: integer, example: 42 }
          content:
            application/json:
              example:
                errors:
                  - status: '429'
                    code: Throttle
  /loginUsingId:
    post:
      summary: loginUsingId (testing/internal)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [user_id]
              properties:
                user_id: { type: string, format: uuid, example: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1 }
      responses:
        '200': { $ref: '#/paths/~1login/post/responses/200' }
        '403':
          description: Disabled in prod
          content:
            application/json:
              example:
                errors:
                  - status: '403'
                    code: GuardMismatch
  /refresh:
    post:
      summary: Rotate tokens
      security:
        - bearerAuth: []
      responses:
        '200': { $ref: '#/paths/~1login/post/responses/200' }
        '401':
          description: Expired/Invalid token
          content:
            application/json:
              example:
                errors:
                  - status: '401'
                    code: AuthError::ExpiredToken
                    title: Token expired
  /logout:
    post:
      summary: Logout
      security:
        - bearerAuth: []
      responses:
        '204': { description: No Content }
        '401': { $ref: '#/paths/~1refresh/post/responses/401' }
  /me:
    get:
      summary: Identity probe
      security:
        - bearerAuth: []
      responses:
        '200':
          description: Identity
          content:
            application/json:
              example:
                data: { id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1, email: ada@example.com }
        '401':
          description: Invalid/Expired/GuardMismatch
          content:
            application/json:
              example:
                errors:
                  - status: '401'
                    code: AuthError::GuardMismatch
                    title: Guard mismatch
  /form:
    post:
      summary: CSRF-protected write
      parameters:
        - name: X-CSRF-TOKEN
          in: header
          schema: { type: string, example: abc123 }
        - name: Sec-Fetch-Site
          in: header
          schema: { type: string, enum: [same-origin, cross-site, none], example: same-origin }
        - name: Origin
          in: header
          schema: { type: string, example: https://app.example.com }
      responses:
        '200':
          description: Accepted
          content:
            application/json:
              example: { ok: true }
        '403':
          description: Untrusted origin
          content:
            application/json:
              example:
                errors:
                  - status: '403'
                    code: CsrfError::UntrustedOrigin
                    title: Untrusted origin
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    JwtClaims:
      type: object
      properties:
        sub: { type: string, format: uuid, example: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1 }
        exp: { type: integer, example: 1726032800 }
```

## 4. Error Catalogue

| HTTP | Code | When |
|------|------|------|
| 401 | `BadCredentials` / `InvalidToken` / `ExpiredToken` / `GuardMismatch` | wrong creds / parse/refresh / `Auth::guard("api")` |
| 403 | `CsrfError::UntrustedOrigin` | `cross-site` + invalid origin even with valid token |
| 500 | `AuthError::StoreUnavailable` | Redis/DB down during guard store read |

## 5. Cross-References

- Module: [auth.md](../../modules/identity-access/auth.md) · [csrf.md](../../modules/identity-access/csrf.md) · [validation.md](../../modules/identity-access/validation.md)
- Testing: [testing/identity-access/test-auth-validation.md](../../testing/identity-access/test-auth-validation.md) · `testing/fixtures/csrf-matrix.json` (6 rows) · `testing/contracts/jwt-claims.schema.json`

## 6. A-Gate

- [x] 401/403 error bodies with `errors[]` examples.
- [x] YAML valid OpenAPI 3.0 with `security`, param constraints (email format, minLength, format: uuid).
- [x] curl valid (header variants for `Sec-Fetch-Site` matrix).

## 7. Chaos Note

`Sec-Fetch-Site` header evolution via nightly browsers is a `spec-version` gate (R-03); CSRF matrix is nightly-rerun.

