# API: Foundation — Bootstrap & Container Contracts

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §1` · `requirements/prd FR-000..008` · `requirements/fsd FS-M0-01..04` · `requirements/tdd BC-0`
> **Crates:** `rustasea` (umbrella) · `rustasea-foundation` · `rustasea-config`
> **Module:** [modules/foundation/overview.md](../../modules/foundation/overview.md) · **Testing:** [testing/foundation/overview.md](../../testing/foundation/overview.md)

> **Note:** M0 has no HTTP endpoints. This spec documents trait contracts per `api-module.md` Source Analysis — derived from `api-contracts.md §1` + `tdd.md BC-0` traits. Consumers are provider authors, not HTTP clients. Chaos-annotated status is included (§8).

## 1. Standar Global

- **Base URL:** n/a — Rust trait contracts consumed at `bootstrap/app.rs` compile time.
- **Content-Type:** n/a — `serde` typed `ConfigRegistry` instead.
- **Format Tanggal:** RFC3339 `YYYY-MM-DDTHH:mm:ssZ` for `created_at`/`updated_at` in `migrations` technical table.
- **Provenance:** Inferred from `api-contracts.md §1` + `tdd.md BC-0` — marked `x-inferred: true` in YAML.

## 2. Endpoints

### 2.1 `Application::configure` — Collect providers

- **Descriptor:** `Application::configure() -> AppBuilder` — builder collecting `Vec<Box<dyn ServiceProvider>>`; `AppBuilder::providers([...])`.
- **Deskripsi:** Entry point assembling provider DAG. No I/O.
- **Kontrol Akses:** Compile-time only. No auth.

#### Request
Not a wire request — Rust builder:

```rust
let state = Application::configure()
    .providers(vec![Box::new(ConfigProvider), Box::new(AuthProvider)])
    .build()
    .boot().await?;
```

#### Response

**Sukses (Ok(AppState)):**

```json
{ "app_running": true, "providers_booted": ["Config","Auth"], "app_state": "Arc<AppState>" }
```

**Error (BootError::Cycle):**

```json
{
  "errors": [{
    "status": "500",
    "code": "BootError::Cycle",
    "title": "Circular provider dependency",
    "detail": "Cycle detected: A -> B -> A. Booting cannot complete without reconfigure.",
    "source": { "pointer": "/depends_on" }
  }]
}
```

**Error (ConfigError::Parse):**

```json
{
  "errors": [{
    "status": "422",
    "code": "ConfigError::Parse",
    "title": "Invalid configuration",
    "detail": "Failed to parse config/database.toml at line 7: expected '=' after key.",
    "source": { "pointer": "/config/database.toml", "line": 7 }
  }]
}
```

#### Usage

No `curl` — prove boot via `cargo test`:

```bash
cargo test -p rustasea-foundation --test m0_foundation -- --nocapture
# stub is #[ignore = "stub: crate not yet implemented"] until crate ships
```

#### OpenAPI (valid OpenAPI 3.0 — trait-contract note)

```yaml
openapi: 3.0.3
info:
  title: RustaSea Foundation — Bootstrap Contracts (no HTTP)
  version: 0.1.0
  description: Trait-level contracts inferred from design/api-contracts.md §1 + tdd.md BC-0. Not a wire API.
x-inferred: true
paths: {}
components:
  schemas:
    BootError:
      type: object
      properties:
        code: { type: string, example: "BootError::Cycle" }
        chain: { type: array, items: { type: string }, example: ["A","B"] }
    ConfigError:
      type: object
      properties:
        file: { type: string, example: "config/database.toml" }
        line: { type: integer, example: 7 }
        source: { type: string, example: "invalid TOML" }
  securitySchemes:
    none: { type: http, scheme: bearer, bearerFormat: JWT, description: "No HTTP — Rust trait contracts" }
security: []
```

### 2.2 `Container::Make<T>` — Typed resolution

- **Descriptor:** `Make::<T> -> Result<Arc<T>, ContainerError>` + `Make::<Option<T>> -> Option<Arc<T>>` + `Manager::extend`.
- **Deskripsi:** `Bind` (transient), `Singleton` (`ptr_eq` stable `Arc<T>`), `Instance` (value). Note: `Manager::extend` closure bound to manager so `self` resolves inside driver extensions (#20).
- **Kontrol Akses:** n/a.

#### Request

```rust
container.singleton::<Counter>(|| Counter(0));
let a: Arc<Counter> = state.make::<Counter>()?;
let b: Arc<Counter> = state.make::<Counter>()?;
assert!(Arc::ptr_eq(&a, &b));
let none: Option<Arc<Mailer>> = state.make_opt::<Mailer>(); // None when unbound (Laravel 13 #20)
```

#### Response

**Sukses (Arc<T> resolved):**

```json
{ "type": "Counter", "ptr_eq": true, "value": 0 }
```

**Error (ContainerError::NotFound):**

```json
{
  "errors": [{
    "status": "500",
    "code": "ContainerError::NotFound",
    "title": "Binding not found",
    "detail": "No binding registered for type PaymentGateway.",
    "source": { "pointer": "/bindings/PaymentGateway" }
  }]
}
```

#### OpenAPI

```yaml
openapi: 3.0.3
info:
  title: RustaSea Container — Make<T> contracts
  version: 0.1.0
paths: {}
components:
  schemas:
    ContainerError:
      type: object
      properties:
        code: { type: string, example: "ContainerError::NotFound" }
        type_name: { type: string, example: "PaymentGateway" }
  securitySchemes:
    none: { type: http, scheme: bearer, description: "No HTTP — in-process Make<T>" }
security: []
x-inferred: true
```

## 3. Error Catalogue

| Code | When | Body example |
|------|------|--------------|
| `BootError::Cycle` | circular `depends_on` | chain `["A","B"]` |
| `BootError::MissingDependency` | depends_on missing provider | `provider: "Config"` |
| `ConfigError::Parse` | invalid TOML | `file + line + source` |
| `ContainerError::NotFound` | unbound `Make::<T>` | `type_name` |
| `ContainerError::AlreadyBound` | strict duplicate bind | `type_name` |

## 4. Usage (copy-paste proof)

```bash
cargo test -p rustasea-foundation --test m0_foundation -- --include-ignored --list | grep stub:
cargo insta test --accept  # snapshots once crate lands
```

## 5. Cross-References

- Module: [boot.md](../../modules/foundation/boot.md) · [container.md](../../modules/foundation/container.md) · [config.md](../../modules/foundation/config.md)
- Design: `design/api-contracts.md §1` · `tdd.md BC-0` · `architecture.md §3 DAG`
- Testing: [testing/foundation/test-boot-container.md](../../testing/foundation/test-boot-container.md) · `testing/stubs/m0-foundation.stub.rs`
- BDD: `@foundation`, `@container`, `@observability-tooling`

## 6. Skill Reference

| Layer | Skill | Rule |
|-------|-------|------|
| API | `technical-documentation` Part A | `api-module.md` A-Gate |
| QA | `test-planning` | `qa-design` boot rows |
| Contract | `test-generation` | `api-contract-test` via trait probes |
| Chaos | `non-functional-testing` | drain timeout expiry is §8 of `boot.md` |

## 7. A-Gate

- [x] Error responses have realistic examples with `errors[]` + `code`.
- [x] YAML passes validation (valid OpenAPI 3.0, `security` present, `x-inferred` on non-wire).
- [x] Parameter constraints captured (type `Arc<T>`, `Option<T>` adjacency).
- [x] Usage uses valid CLI (`cargo test -p ...`).

## 8. Chaos & Resilience Note (non-functional-testing)

Per `chaos-engineering` rules: foundation is the reboot path — chaos inject is `docker pause postgres` during `migrate` + `kill -TERM` mid-boot to validate drain gate. See [testing/foundation/test-boot-container.md](../../testing/foundation/test-boot-container.md) §4 Monkey.


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
