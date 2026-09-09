# Testing: DataOrm (M2 — ORM & Database)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/data-orm/overview.md](../../modules/data-orm/overview.md)
> **BDD:** `@orm`, `@query-builder-additions`, `@upsert-delete`, `@collection-serialization`, `@vector-search` · **FSD:** FS-M2-01..06

## 1. Scope

Covers `#[derive(Model)]` + relations + soft delete, fluent builder, `vector` Blueprint + `whereVectorSimilarTo` + extension guard/dim mismatch, migrations/seeds/factories with `Str` reset, collection `serde` round-trip (TC-PROP-01). `sqlx::migrate!` round-trip/idempotence + bulk `migrate:fresh --seed`.

## 2. Trace

- Stubs: `testing/stubs/m2-orm.stub.rs`
- Contracts: `RouteEntry` adjacency not here; `toSql` snapshots; `fixtures/vector-dim.json`.
- BDD: `bdd-scenarios.md §2.3` (6 Features: fluent, builder additions, upsert/delete, collection, derived, vector).

## 3. Links

- Specs: [test-orm.md](test-orm.md)
- API: [api-query-builder](../../api/data-orm/api-query-builder.md)
- Module docs: [model-relations](../../modules/data-orm/model-relations.md) · [query-builder](../../modules/data-orm/query-builder.md) · [migrations](../../modules/data-orm/migrations.md) · [vector](../../modules/data-orm/vector.md)

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
