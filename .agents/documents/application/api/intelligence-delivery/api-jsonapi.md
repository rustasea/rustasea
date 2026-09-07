# API: IntelligenceDelivery — JSON:API Resources (sparse + include + `RelationNotLoaded`)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §5` · `requirements/prd FR-604` · `requirements/fsd FS-M6-03` · `requirements/tdd BC-6 JsonApiResource` · `design/domain BC-6` · BDD `@jsonapi`
> **Crates:** `rustavel-jsonapi` (and `rustavel-orm` eager `with(...)`)

## 1. Standar Global

- **Base URL:** `http://localhost:3000` — wired via app-level resource routes (e.g., `GET /users/:id?include=posts&fields[users]=name,email`).
- **Content-Type:** `application/vnd.api+json` (response and request helper meta).
- **Sparse:** `fields[users]=name,email` filters `attributes` to `["name","email"]` exactly.
- **Include:** `include=posts` expects `User::with("posts")` eager-loaded; missing → `RelationNotLoaded`.
- **Format Tanggal:** RFC3339 in `meta` if used.

## 2. Endpoints

### 2.1 `GET /users/{id}?include=&fields=` — JSON:API resource

- **URL:** `GET /users/{id}?include=posts&fields[users]=name,email&fields[posts]=title`
- **Deskripsi:** `UserResource::new(user).include("posts").fields(["name","email"]).to_response()` builds `{ data:{type,id,attributes,relationships,links{self}}, included:[{type,id,attributes}], links{self}, meta }` per JSON:API 1.1 (see `testing/contracts/jsonapi.schema.json` + provider stub).
- **Kontrol Akses:** `Bearer JWT`.

#### Params

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `id` | string uuid | yes | `users` id | `b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1` |
| `include` | string comma-list `posts` etc. | no | relationship inclusion (must be eager) | `posts` |
| `fields[users]` | string comma-list | no | sparse fieldset for `users` | `name,email` |
| `fields[posts]` | string comma-list | no | sparse for included `posts` | `title` |

#### Response

**Sukses (200 with include+fields filtering):**

```json
{
  "data": {
    "type": "users",
    "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1",
    "attributes": { "name": "Ada" },
    "relationships": {
      "posts": {
        "data": [{ "type": "posts", "id": "p1" }],
        "links": { "self": "/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1/relationships/posts", "related": "/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1/posts" }
      }
    },
    "links": { "self": "/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1" }
  },
  "included": [{ "type": "posts", "id": "p1", "attributes": { "title": "Hello" } }],
  "links": { "self": "/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1?include=posts" },
  "meta": {}
}
```

`fields[name]` outline — `name` → `{name}` only, `name,email` → `{name,email}` (see BDD `@jsonapi` outline).

**Error (500 — RelationNotLoaded) → surfaced as 500 not 404 because it's programmer error (requires eager load):**

```json
{
  "errors": [{
    "status": "500",
    "code": "JsonApiError::RelationNotLoaded",
    "title": "Relation not loaded",
    "detail": "include posts requires eager-loaded relation; call User::with(\"posts\") first.",
    "source": { "pointer": "/include", "parameter": "posts" }
  }]
}
```

#### Usage

```bash
curl -s http://localhost:3000/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1?include=posts \
  -H 'Accept: application/vnd.api+json' -H 'Authorization: Bearer '"$JWT" | jq .

curl -s 'http://localhost:3000/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1?fields[users]=name&include=posts' \
  -H 'Accept: application/vnd.api+json' -H 'Authorization: Bearer '"$JWT" | jq .
```

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: Rustavel JSON:API Resources
  version: 0.1.0
  description: Sparse fieldsets + include per api-contracts.md §5
servers:
  - url: http://localhost:3000
paths:
  /users/{id}:
    get:
      summary: JSON:API user with sparse fieldsets and compound inclusion
      parameters:
        - name: id
          in: path
          required: true
          schema: { type: string, format: uuid, example: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1 }
        - name: include
          in: query
          schema: { type: string, example: posts }
        - name: fields[users]
          in: query
          style: form
          schema: { type: string, example: name }
      responses:
        '200':
          description: JSON:API document
          content:
            application/vnd.api+json:
              example:
                data:
                  type: users
                  id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1
                  attributes: { name: Ada }
                  relationships:
                    posts:
                      data: [{ type: posts, id: p1 }]
                included:
                  - type: posts
                    id: p1
                    attributes: { title: Hello }
                links: { self: "/users/b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1?include=posts" }
                meta: {}
        '500':
          description: Relation not loaded
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '500'
                    code: JsonApiError::RelationNotLoaded
                    title: Relation not loaded
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    JsonApiDocument:
      type: object
      properties:
        data: { type: object }
        included: { type: array, items: { type: object } }
        links: { type: object }
        meta: { type: object }
security:
  - bearerAuth: []
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 200 | — | document `{data,included,links,meta}` with correct `application/vnd.api+json` |
| 500 | `RelationNotLoaded{relation}` | `include=posts` without `with("posts")` |

## 5. Cross-References

- Module: [jsonapi.md](../../modules/intelligence-delivery/jsonapi.md)
- Testing: [testing/intelligence-delivery/test-advanced.md](../../testing/intelligence-delivery/test-advanced.md) · BDD `@jsonapi` · `testing/contracts/jsonapi.schema.json` + `__snapshots__/jsonapi-user.json.snap`

## 6. A-Gate

- [x] 200 + 500 examples (sparse outline noted).
- [x] YAML valid with `security`, constraints (format uuid, sparse list), examples.
- [x] curl valid.

## 7. Chaos Note

Include large compound doc size guard is nightly chaos (see `jsonapi.md`).

