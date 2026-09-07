# API: AsyncWorkloads — Cache (`Store`/`Repository` + `touch` + `Lock`)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §4` · `requirements/prd FR-403..405` · `requirements/fsd FS-M4-04` · `requirements/tdd BC-4 Store/Lock`
> **Crates:** `rustavel-cache` (`Store` `moka` + `deadpool-redis`) · **BDD:** `@cache-touch`, `@cache`

## 1. Standar Global

- **Producer:** `Cache::store("redis").put("k","v", Duration::from_secs(60)).await?` etc. — not a wire API unless app exposes `/cache` helpers.
- **Serialization:** JSON default with hyphenated `-cache-` prefix (hardening from M3 `serializable_classes` allow-list).
- **Driver-agnostic:** `memory` vs `redis` stores isolated: `put` in `redis` → `get` in `memory` is `None`.

## 2. Endpoints (Store/Repository contract via HTTP examples if exposed)

### 2.1 `Cache::put` / `Cache::get` — basic K/V

Demonstrates expiration handling + `-cache-` prefix contract.

**Example app endpoint `GET /cache/{k}` / `POST /cache/{k}`** (if app wires it; otherwise trait calls shown):

```bash
# app-level example
curl -s http://localhost:3000/cache/k -H 'Authorization: Bearer '"$JWT" | jq .
# -> { "key":"-cache-k", "value":"v", "expires_at":"2026-09-07T10:05:00Z" }
```

**Trait:**

```rust
Cache::store("redis").put("k","v", Duration::from_secs(60)).await?;
let v: Option<String> = Cache::store("redis").get::<String>("k").await?; // Some("v")
let v: Option<String> = Cache::store("memory").get::<String>("k").await?; // None (isolated)
Cache::store("redis").remember("k", Duration::from_secs(60), || async { Ok(compute().await) }).await?;
```

**Error (StoreUnavailable):**

```json
{ "errors": [{ "status": "500", "code": "CacheError::StoreUnavailable", "title": "Store unavailable", "detail": "deadpool-redis BRPOP/down." }] }
```

### 2.2 `Cache::touch(key, ttl)` — extend TTL without get/set

- **Descriptor:** `Cache::store("redis").touch("k", Duration::from_secs(120)).await? -> bool`
- **Rule:** `false` if missing, not error; `true` if extended; `CacheTouchFailed` only on store errors.

#### Params

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `key` | string | yes | cache key (prefix `-cache-` added) | `k` |
| `ttl` | Duration 0..86400s | yes | extends to ttl from touch point | `120s` |

#### Response

**Sukses (hit) → touched true:**

```json
{ "key": "k", "touched": true, "new_ttl": 120 }
```

**Sukses (miss) → touched false:**

```json
{ "key": "k", "touched": false }
```

#### Usage

```bash
curl -s -X POST http://localhost:3000/cache/k/touch \
  -H 'Content-Type: application/json' -H 'Authorization: Bearer '"$JWT" \
  -d '{"ttl":120}' | jq .
```

### 2.3 `Lock` — distributed mutual exclusion

- **Descriptor:** `Cache::lock("billing", Duration::from_secs(10)).get().await? -> Option<LockGuard>` ; `block(Duration::from_secs(5))` waits or `AlreadyHeld`.
- **Backing:** Redis `SET NX EX` / `moka` entry guard; lease expiry prevents deadlock.

| Field | Type | Req | Desc | Example |
|-------|------|-----|------|---------|
| `key` | string | yes | lock key | `billing` |
| `ttl` | Duration | yes | lock lease | `10s` |
| `wait` | Duration | yes (for block) | wait window | `5s` |

#### Response

**Sukses (`get` when free):**

```json
{ "key": "billing", "owner": "worker-1a2b", "acquired": true, "ttl": 10 }
```

**Error (`block` timeout — AlreadyHeld):**

```json
{ "errors": [{ "status": "408", "code": "LockError::AlreadyHeld", "title": "Lock contention", "detail": "Worker B waited 2s; lease still held by A." }] }
```

#### Usage

```rust
let guard: Option<LockGuard> = Cache::lock("billing", Duration::from_secs(10)).get().await?;
let guard2: LockGuard = Cache::lock("billing", Duration::from_secs(10)).block(Duration::from_secs(5)).await?;
// guard RAII — release on drop
```

## 3. OpenAPI 3.0 Snippet (app-level HTTP examples when app exposes `/cache`)

```yaml
openapi: 3.0.3
info:
  title: Rustavel Cache — Store + Lock (app-level HTTP examples)
  version: 0.1.0
  description: Store/Repository Touch + Lock per api-contracts.md §4; isolation: memory vs redis
servers:
  - url: http://localhost:3000
paths:
  /cache/{key}:
    get:
      summary: Get cache entry
      parameters:
        - name: key
          in: path
          required: true
          schema: { type: string, example: k }
      responses:
        '200':
          description: Value (JSON-backed)
          content:
            application/json:
              example: { key: "-cache-k", value: "v", expires_at: "2026-09-07T10:05:00Z" }
        '404':
          description: Miss (null result, not HTTP 404 when via trait Option)
          content:
            application/json:
              example: { value: null }
        '500':
          description: Store unavailable
          content:
            application/json:
              example:
                errors:
                  - status: '500'
                    code: CacheError::StoreUnavailable
    post:
      summary: Put cache entry
      parameters:
        - name: key
          in: path
          required: true
          schema: { type: string, example: k }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [value, ttl]
              properties:
                value: { type: string, example: "v" }
                ttl: { type: integer, minimum: 0, maximum: 86400, example: 60 }
      responses:
        '200': { description: Stored }
  /cache/{key}/touch:
    post:
      summary: Extend TTL without get/set
      parameters:
        - name: key
          in: path
          required: true
          schema: { type: string, example: k }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [ttl]
              properties:
                ttl: { type: integer, minimum: 0, example: 120 }
      responses:
        '200':
          description: Touched (or miss false)
          content:
            application/json:
              example: { key: k, touched: true, new_ttl: 120 }
  /cache/lock/{key}:
    post:
      summary: Acquire Lock (GET NX EX then BLOCK)
      parameters:
        - name: key
          in: path
          required: true
          schema: { type: string, example: billing }
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                ttl: { type: integer, example: 10 }
                wait: { type: integer, example: 5 }
      responses:
        '200':
          description: Lock acquired
          content:
            application/json:
              example: { key: billing, owner: worker-1a2b, acquired: true, ttl: 10 }
        '408':
          description: Lock contention
          content:
            application/json:
              example:
                errors:
                  - status: '408'
                    code: LockError::AlreadyHeld
components:
  securitySchemes:
    bearerAuth: { type: http, scheme: bearer, bearerFormat: JWT }
security: []
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 200 | `touched:false` (not HTTP error) | `touch` on missing key |
| 408 | `LockError::AlreadyHeld` | `block` timeout |
| 500 | `CacheError::StoreUnavailable` | Redis/Memory store down |

## 5. Cross-References

- Module: [cache.md](../../modules/async-workloads/cache.md)
- Design: `api-contracts.md §4 cache` · `tdd.md BC-4 Store/Lock` · `database.md §2 cache/cache_locks` · `capacity.md §3`
- Testing: [test-queue-cache](../../testing/async-workloads/test-queue-cache.md) · BDD `@cache-touch` · `testing/stubs/m4-queue-cache-schedule.stub.rs`

## 6. A-Gate

- [x] 200/408/500 examples (touch miss false is 200 with `touched:false`).
- [x] YAML valid with constraints (ttl min/max, key).
- [x] curl valid.

## 7. Chaos Note

`Lock::block` contention + `docker pause redis` → `StoreUnavailable` typed (not panic) is nightly chaos.

