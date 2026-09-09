# ADR-002 — ORM: sqlx Primary, sea-orm Optional

> **Status:** Accepted  
> **Date:** 2026-09-07  
> **Deciders:** Tech Lead, Data  
> **Milestone:** M2 (ORM & Database)  
> **Related:** FR-200–210 · FS-M2-01–06 · BR-03/C-02 · Laravel 13 #6/#13–#15

## Context

RustaSea's ORM must support: fluent Eloquent-like builder (`where`/`orWhere`/`paginate`/`chunkBy`/`toSql`), `#[derive(Model)]` with `id`/`created_at`/`updated_at`/`deleted_at` + `serde` relation preservation (Laravel 13 #13), Postgres/MySQL/SQLite drivers, `pgvector` `whereVectorSimilarTo` / `vector` column (Laravel 13 #6), strict `upsert` + MySQL `DELETE JOIN` (Laravel 13 #14), migrations and `deadpool` pooling. Two Rust options cover this surface: `sqlx` (compile-time-checked, query-builder + `query_as!`) and `sea-orm` (ActiveRecord-style, Entity/ActiveModel, richer relation ergonomics). Choice affects DX, `pgvector` path, and maintenance cost.

## Decision

**`sqlx` is the primary ORM. `sea-orm` is an optional shim behind a feature flag, not a co-primary.**

- `sqlx` powers `RustaSea-orm` query builder, `deadpool`/`sqlx` pools, `sqlx::migrate!` migrations, and `pgvector` integration (`vector <=> $1` emission). Feature `vector` gates pgvector.
- `sea-orm` (when enabled via `features = ["sea-orm"]` on `rustasea-orm`) provides an ActiveRecord-style `Entity`/`ActiveModel` compat layer over the same pools and migrations; it does not replace the builder.
- `sqlx::migrate!` is the canonical migration system; `sea-orm-migration` wraps it when the shim is active.

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **sqlx primary + sea-orm shim** | Compile-time checked queries; proven `pgvector` and `deadpool` integration; minimal magic; single migration source of truth | More proc-macro work to reach Eloquent-level DX (offset by `rustasea-macros`) | **Chosen** — best balance of safety, vector story, and operator flexibility |
| sqlx only | Simpler crate graph; no duality cost | Teams expecting ActiveRecord must adapt | Viable but strictly narrower than chosen; may revisit if shim sees no use by M3 |
| sea-orm primary | Richest Eloquent-like relations/scopes out of box | Weaker compile-time SQL checking; `pgvector` is bolt-on; migration system diverges | Rejected — safety/vector priorities favor `sqlx` |
| Diesel | Strong schema-checked DSL | Sync-first, heavier bridging with async `tokio`; less `pgvector` ecosystem at decision time | Rejected |

## Consequences

- Default `cargo check -p rustasea-orm` does **not** pull `sea-orm`; shim users add `features = ["sea-orm"]`.
- `rustasea-orm` crate exposes both `QueryBuilder<T>` (builder) and re-exported `sea_orm` entities when flagged — never both required simultaneously.
- Migration files are generated once (`YYYY_MM_DD_HHMMSS_name.rs` with `up`/`down`); `migrate`/`migrate:fresh` work regardless of ORM flag.
- Risk R-01 (ORM duality doubles maintenance) is mitigated by committing to `sqlx` as primary pre-M2; ADR gated before M2 implementation per FSD.

## Validation

- `cargo tree` with default features shows `sqlx` but no `sea-orm`.
- `cargo tree --features sea-orm -p rustasea-orm` shows both, behind the same pool types.
- Vector integration test (FR-207) runs via `sqlx` + `testcontainers` Postgres with `pgvector`; dimension-mismatch and `ExtensionMissing` diagnostics asserted.

## References

- README Tech Stack ORM row — `sqlx (primary) + sea-orm (optional)` rationale.
- `database.md §4–5` — vector extension guard and index strategies.
- FSD FS-M2-03 — builder method surface that favors `sqlx`.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
