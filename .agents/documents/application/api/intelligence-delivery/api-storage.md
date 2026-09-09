# API: IntelligenceDelivery — Storage (read-through + `PathTraversal`)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §5` · `requirements/prd FR-603, FR-611` · `requirements/fsd FS-M6-02` · `requirements/tdd BC-6 StorageManager` · `design/database §2 storage_objects`
> **Crates:** `rustasea-storage` · **BDD:** `@storage-readthrough`

## 1. Standar Global

- **Storage config:** `readThrough { primary: "s3", fallback: "local", copy_back: true/false }`. Disks `s3`/`gcs`/`azure` (`object_store`) or `local` (`tokio::fs`). No direct wire unless the app wires `/storage/{path}` helpers — this spec documents both the Rust `StorageManager` contract and an example HTTP `GET /storage/{path}` read-through shape.
- **Security:** `Storage::path("a/b.txt")` canonicalizes and enforces `resolved.starts_with(disk_root)` → `PathTraversal` on escape; fuzzed with `testing/fixtures/path-traversal.corpus.json`.

## 2. Endpoints

### 2.1 `GET /storage/{path}` — Read-through example HTTP (app-level)

- **URL:** `GET /storage/{path}` — path segment verbatim (not double-encoded).
- **Deskripsi:** `Storage::disk("s3").get("a/b.txt").await?` read-through `primary->fallback` + optional `copy_back` (`api-contracts.md §5`). Returns file bytes.
- **Kontrol Akses:** `Bearer JWT` when guarded, else internal.

#### Params

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `path` | string 1..1024 | yes | logical path under disk root. `..` etc. checked at routing time | `a/b.txt` |

Targets checked `../../etc/passwd` variants via path-time confinement (not filesystem probe).

#### Response

**Sukses (200):**

```http
HTTP/1.1 200 OK
Content-Type: text/plain
Content-Length: 5

hello
```

**Error (404 — NotFound on both primary + fallback):**

```json
{ "errors": [{ "status": "404", "code": "StorageError::NotFound", "title": "Not Found", "detail": "No store contains a/b.txt.", "source": { "pointer": "/path", "value": "a/b.txt" } }] }
```

**Error (403 — PathTraversal):**

```json
{
  "errors": [{
    "status": "403",
    "code": "StorageError::PathTraversal",
    "title": "Path traversal blocked",
    "detail": "Path ../../etc/passwd escapes disk root.",
    "meta": { "disk_root": "/srv/app/storage/app", "resolved": "/etc/passwd" }
  }]
}
```

**Behavioral cases:**

| Precondition | Action | Result |
|--------------|--------|--------|
| file `a/b.txt` only on `local` (fallback) | `Storage::get("a/b.txt")` | bytes from `local` |
| same + `copy_back: true` | second `GET` | now also on `s3` |
| no store contains file | `Storage::get("a/b.txt")` | `NotFound` uniform |
| path `../../etc/passwd` | `Storage::path("...")` | `PathTraversal` without fs access |

#### Usage

```bash
curl -s http://localhost:3000/storage/a/b.txt -H 'Authorization: Bearer '"$JWT"
# or via trait directly (no HTTP):
# Storage::disk("s3").get("a/b.txt").await?; Storage::path("../../etc/passwd") -> Err(PathTraversal)
```

### 2.2 `Storage::path("a/b.txt")` — Canonical confinement (Rust trait)

- **Descriptor:** `disk.path("a/b.txt") -> Result<PathBuf, StorageError>` checks `resolved.starts_with(disk_root)`.
- **Behavior:** S3 virtual path still logically confined.

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: RustaSea Storage — read-through + path confinement
  version: 0.1.0
  description: Read-through primary+fallback + PathTraversal per api-contracts.md §5
servers:
  - url: http://localhost:3000
paths:
  /storage/{path}:
    get:
      summary: Read-through file
      parameters:
        - name: path
          in: path
          required: true
          schema: { type: string, minLength: 1, maxLength: 1024, example: "a/b.txt" }
      responses:
        '200':
          description: File bytes
          content:
            text/plain:
              example: hello
            application/octet-stream:
              schema: { type: string, format: binary }
        '403':
          description: Path traversal blocked
          content:
            application/json:
              example:
                errors:
                  - status: '403'
                    code: StorageError::PathTraversal
                    title: Path traversal blocked
                    meta: { disk_root: "/srv/app/storage/app" }
        '404':
          description: Not found on any store
          content:
            application/json:
              example:
                errors:
                  - status: '404'
                    code: StorageError::NotFound
                    title: Not Found
components:
  securitySchemes:
    bearerAuth: { type: http, scheme: bearer, bearerFormat: JWT }
  schemas:
    StorageObject:
      type: object
      properties:
        disk: { type: string, example: s3 }
        path: { type: string, example: a/b.txt }
        size_bytes: { type: integer, example: 5 }
        mime: { type: string, example: text/plain }
security: []
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 200 | — | bytes (read-through) |
| 403 | `PathTraversal` | `..` / `%2e%2e` / long chains / symlink escape |
| 404 | `NotFound` | neither `primary` nor `fallback` contains `path` |

## 5. Cross-References

- Module: [storage.md](../../modules/intelligence-delivery/storage.md)
- Testing: [testing/intelligence-delivery/test-advanced.md](../../testing/intelligence-delivery/test-advanced.md) · BDD `@storage-readthrough` · `testing/fixtures/path-traversal.corpus.json` (corpus) · `testing/stubs/m6-advanced.stub.rs`

## 6. A-Gate

- [x] 403/404 with `errors[]` + realistic `path`.
- [x] YAML valid, constraints, examples.
- [x] curl valid.

## 7. Chaos & Resilience Note

S3 down mid-readthrough → fallback path observable; corpus stall is nightly chaos.


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
