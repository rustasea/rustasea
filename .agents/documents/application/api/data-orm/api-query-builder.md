# API: DataOrm — Query Builder, Pagination & Vector

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §1` · `requirements/prd FR-200..210` · `requirements/fsd FS-M2-01..06` · `requirements/tdd BC-2` · `design/database §1..5`
> **Crates:** `rustavel-orm` · `rustavel-macros`
> **Module:** [modules/data-orm/overview.md](../../modules/data-orm/overview.md) · **Testing:** [testing/data-orm/overview.md](../../testing/data-orm/overview.md)

> **Note:** Builder contracts are handler-return shapes (via `Json<T>`) plus wire pagination envelope and vector query. No standalone `/api/v1/models` CRUD endpoint exists until app defines routes — this spec documents the builder + envelope contracts used by every collection endpoint.

## 1. Standar Global

- **Base URL:** `http://localhost:3000` — collection routes are app-defined (e.g., `GET /users`, `GET /products`).
- **Content-Type:** `application/vnd.api+json` when wrapped as JSON:API; `application/json` for `Paginated<T>` via `Json` extractor `422 ErrorBag` adjacency still applies.
- **Format Tanggal:** RFC3339.

## 2. Endpoints (builder → wire shapes)

### 2.1 `GET /users?page=&per_page=` — `paginate(page, perPage)` envelope

- **URL:** `GET /users?page=1&per_page=15` — produced by `User::query().where(...).paginate(15).await` inside a handler returning `Json<Paginated<User>>`.
- **Deskripsi:** Cursor-safe pagination (standard) + links; `links.prev` nullable on first page. 7-route resource shape via `Route::resource("users")`.
- **Kontrol Akses:** `Bearer JWT` when route has `#[middleware("auth:jwt")]`; `429` when throttled.

#### Params

| Name | Type | Req | Default | Desc | Example |
|------|------|-----|---------|------|---------|
| `page` | integer 1..1000 | no | 1 | current page | `1` |
| `per_page` | integer 1..100 | no | 15 | per page (cursor qty) | `15` |
| `cursor` | string base64 | no | — | for `cursor()` mode alternative | `eyJpZCI6...` |
| `where.status` | string enum | no | — | example filter `status=active` | `active` |

#### Response

**Sukses (200 — Paginated<User>):**

```json
{
  "data": [{ "type": "users", "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "attributes": { "name": "Ada", "email": "ada@example.com" } }],
  "meta": { "current_page": 1, "per_page": 15, "total": 42, "last_page": 3 },
  "links": { "first": "/users?page=1", "prev": null, "next": "/users?page=2", "last": "/users?page=3" }
}
```

**Error (404 — firstOrFail):**

```json
{ "errors": [{ "status": "404", "code": "QueryError::NotFound", "title": "Not Found", "detail": "No rows for firstOrFail." }] }
```

#### Usage

```bash
curl -s 'http://localhost:3000/users?page=1&per_page=15' \
  -H 'Accept: application/vnd.api+json' \
  -H 'Authorization: Bearer '"$JWT" | jq .
```

### 2.2 `GET /products?q_embedding=&limit=` — `whereVectorSimilarTo` (M2 initial; M6 full via search)

- **URL:** `GET /products?limit=10` — handler computes embedding then `Product::query().whereVectorSimilarTo("embedding", &emb, 10).await`.
- **Deskripsi:** Nearest-neighbor cosine `ORDER BY embedding <=> $1 LIMIT k`. Guarded by `vector` feature + `has_extension("vector")`; HNSW/IVFFLAT index. Fallback after `dropVectorIndex` remains seq scan.
- **Kontrol Akses:** `Bearer JWT` optional; `FeatureUnavailable` when `vector` disabled.

#### Params

| Name | Type | Req | Default | Desc | Example |
|------|------|-----|---------|------|---------|
| `limit` | integer 1..100 | yes | — | top-k | `10` |
| `q` | string | yes (when app derives embedding) | — | query text to embed via `AiProvider::embeddings` | `semantic search` |
| `column` | string enum `[embedding]` | no | `embedding` | vector column | `embedding` |

#### Response

**Sukses (200):**

```json
{
  "data": [
    { "type": "products", "id": "c3d4e5f6-7890-4abc-8def-1234567890aa", "attributes": { "name": "Widget", "dist": 0.12 } }
  ],
  "meta": { "limit": 10, "distance": "cosine" },
  "links": { "self": "/products?limit=10" }
}
```

**Error (500 — extension missing):**

```json
{
  "errors": [{
    "status": "500",
    "code": "VectorError::ExtensionMissing",
    "title": "pgvector extension missing",
    "detail": "CREATE EXTENSION vector failed. Install pgvector: see database.md §4 guard.",
    "meta": { "hint": "CREATE EXTENSION IF NOT EXISTS vector;" }
  }]
}
```

**Error (422 — dimension mismatch):**

```json
{
  "errors": [{
    "status": "422",
    "code": "VectorError::VectorDimensionMismatch",
    "title": "Dimension mismatch",
    "detail": "Column vector(1536) vs query 768.",
    "meta": { "expected": 1536, "actual": 768 }
  }]
}
```

#### Usage

```bash
# Direct builder call inside handler — wire is POST /search/vector with body { q, limit }
curl -s -X POST http://localhost:3000/search/vector \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer '"$JWT" \
  -d '{"q":"semantic search","limit":10}' | jq .
```

### 2.3 Builder mutators (upsert, chunkBy, toSql) — not wire endpoints but contract

Documented for `api-contract-test` coverage — handler authors call them; no separate `/api/v1/upsert` endpoint.

- `upsert(rows, unique_by: &[&str], update: &[&str]) -> UpsertResult{inserted,updated}` — empty `unique_by` → `UpsertError::EmptyUniqueBy` before round-trip → surfaced as `422`.
- `chunkBy("id", 500, |chunk| ...)` — not wire; wire `chunk` job uses iterator.
- `toSql`/`toRawSql` → `cargo insta` snapshots per driver (see `testing/data-orm/test-orm.md`).

**Error (422 upsert):**

```json
{
  "errors": [{
    "status": "422",
    "code": "UpsertError::EmptyUniqueBy",
    "title": "Empty uniqueBy",
    "detail": "upsert unique_by may not be empty.",
    "source": { "pointer": "/unique_by" }
  }]
}
```

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: Rustavel DataOrm — Query Builder + Vector
  version: 0.1.0
  description: Paginated envelope + vector nearest-neighbor — inferred from api-contracts.md §1 + tdd.md BC-2 + database.md §2
servers:
  - url: http://localhost:3000
paths:
  /users:
    get:
      summary: List users (Paginated) — where/whereJson/paginate/forUpdate
      security:
        - bearerAuth: []
      parameters:
        - name: page
          in: query
          schema: { type: integer, minimum: 1, maximum: 1000, example: 1 }
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
                    attributes: { name: Ada, email: ada@example.com }
                meta: { current_page: 1, per_page: 15, total: 42, last_page: 3 }
                links: { first: "/users?page=1", next: "/users?page=2", last: "/users?page=3" }
        '404':
          description: Not Found (firstOrFail)
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '404'
                    code: QueryError::NotFound
                    title: Not Found
        '422':
          description: Upsert strict or validation (when POST uses same envelope)
          content:
            application/json:
              example:
                errors:
                  - status: '422'
                    code: UpsertError::EmptyUniqueBy
                    title: Empty uniqueBy
  /search/vector:
    post:
      summary: Vector nearest-neighbor (whereVectorSimilarTo)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [q, limit]
              properties:
                q: { type: string, example: "semantic search" }
                limit: { type: integer, minimum: 1, maximum: 100, example: 10 }
      responses:
        '200':
          description: Nearest products (cosine)
          content:
            application/vnd.api+json:
              example:
                data:
                  - type: products
                    id: c3d4e5f6-7890-4abc-8def-1234567890aa
                    attributes: { name: Widget, dist: 0.12 }
                meta: { limit: 10, distance: cosine }
        '422':
          description: Dimension mismatch
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '422'
                    code: VectorError::VectorDimensionMismatch
                    title: Dimension mismatch
                    meta: { expected: 1536, actual: 768 }
        '500':
          description: pgvector extension missing
          content:
            application/vnd.api+json:
              example:
                errors:
                  - status: '500'
                    code: VectorError::ExtensionMissing
                    title: pgvector extension missing
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    Paginated:
      type: object
      properties:
        data: { type: array, items: { type: object } }
        meta:
          type: object
          properties:
            current_page: { type: integer, example: 1 }
            per_page: { type: integer, example: 15 }
            total: { type: integer, example: 42 }
            last_page: { type: integer, example: 3 }
        links:
          type: object
          properties:
            first: { type: string, example: "/users?page=1" }
            prev: { type: string, nullable: true, example: null }
            next: { type: string, example: "/users?page=2" }
            last: { type: string, example: "/users?page=3" }
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 404 | `QueryError::NotFound` | `firstOrFail` |
| 422 | `UpsertError::EmptyUniqueBy` / `VectorDimensionMismatch` | empty `unique_by` / dim 1536 vs 768 |
| 500 | `VectorError::ExtensionMissing` | no `CREATE EXTENSION vector` |
| 400 | `QueryError::InvalidCursor` | cursor pagination invalid |

## 5. Cross-References

- Module: [model-relations.md](../../modules/data-orm/model-relations.md) · [query-builder.md](../../modules/data-orm/query-builder.md) · [vector.md](../../modules/data-orm/vector.md) · [migrations.md](../../modules/data-orm/migrations.md)
- Testing: [testing/data-orm/test-orm.md](../../testing/data-orm/test-orm.md) · BDD `@orm`, `@query-builder-additions`, `@upsert-delete`, `@vector-search` · `fixtures/vector-dim.json`

## 6. A-Gate

- [x] Error responses with realistic examples (422/404/500 all with bodies).
- [x] YAML valid OpenAPI 3.0 with `security`, constraints, example.
- [x] curl uses valid CLI.

## 7. Chaos Note

Concurrent `select_for_update` + `dropVectorIndex` mid-search seq-scan fallback (TC-M6-25) are in [testing/data-orm/overview.md](../../testing/data-orm/overview.md) § Chaos.

