# Sprint 03 — M2 ORM & Database

> **Milestone:** M2 · **Window:** 2027-01-01 → 2027-03-31 · **Status:** Planned
> **Parents:** `../roadmap.md` · `prd.md` FR-200–FR-210 · `fsd.md` FS-M2-01–FS-M2-06 · `design/architecture.md` + `design/database.md` + `design/domain.md`
> **Depends On:** M0 (S01), M1 (S02)
> **Crates:** `rustavel-orm`, `rustavel-macros` (`#[derive(Model)]`) — `pgvector` behind `vector` feature

---

## 1. Goal

Fluent, type-safe database layer with migrations, seeders, factories, and initial vector support — so models, queries, and schema evolution feel like Eloquent with compile-time safety.

## 2. Scope (In / Out)

**In:**
- Query builder over `sqlx`/`sea-orm` (Postgres/MySQL/SQLite, `sqlx::migrate!`), drivers via `deadpool`
- `#[derive(Model)]` with `id`/`created_at`/`updated_at`/`deleted_at` (soft delete), `snake_plural` table convention, casts, relations, `serde` eager-relation round-trip (#13)
- Fluent `where`/`orWhere`/`whereJsonContains`/`whereJson` + `find`/`first`/`firstOrFail` + `create`/`save`/`update`/`delete`/`forceDelete` + `paginate`/`cursor`/`chunkBy`/`orWhereKey`/`orWhereKeyNot`/`whereBinary`/`StraightJoin`/`insertOrIgnoreReturning`/`saveOrIgnore`/`refreshForUpdate`
- Strict `upsert` (`uniqueBy` non-empty or `UpsertError::EmptyUniqueBy`) + MySQL `DELETE … JOIN … ORDER BY/LIMIT`
- `toSql`/`toRawSql`, pessimistic locks (`forUpdate`/`sharedLock`), scopes, transactions, raw queries
- `vector` Blueprint column + `whereVectorSimilarTo` + `dropVectorIndex` + `Str::toEmbeddings` trait with `pgvector` (MariaDB behind feature flag) — M2 initial; M6 extends
- Migrations `make:migration`/`migrate`/`migrate:fresh`/`migrate:fresh --seed` + `migrations` table; seeders; factories (`Factory::create`, `Str` sequence reset)
- `database/migrations/`, `database/seeders/`, schema docs in `design/database.md`

**Out:**
- Auth guards, validation, queue/cache — M3/M4. Full 12-provider embeddings are M6 (Sprint 07); S03 provides `pgvector` + `toEmbeddings` trait shape.

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S03-T01 | Connection & driver abstraction + pool + transactions | FR-200, FR-205 (transaction part) | FS-M2-01 | `crates/rustavel-orm/src/{connection,pool,transaction}.rs` | M | Postgres vs SQLite driver-specific SQL; `db.transaction(\|tx\| ...)` atomic; pool `min/max/idle_timeout` from `config.database`; NFR-Sca-01 bench hint (100 concurrent) |
| S03-T02 | `#[derive(Model)]` + relations + soft delete + serde round-trip | FR-201, FR-206 | FS-M2-02 | `crates/rustavel-macros/src/model.rs` + `crates/rustavel-orm/src/model.rs` | L | `User` with `has_many posts`, `User::with("posts").find(1)` eager-loads; `deleted_at` partial index; soft-deleted `find` returns `None`; `serde_json` round-trip preserves `relations`; relation cycle depth ≤3 |
| S03-T03 | Query builder — fluent chain, paginate/cursor, locks, scopes, raw | FR-202, FR-203, FR-205 | FS-M2-03 | `crates/rustavel-orm/src/builder.rs` | L | `chunkBy("id",500)` yields 500 without OOM; `firstOrFail` → `NotFound`; `forUpdate` blocks concurrent writer; `toSql`/`toRawSql` green; `scope` composable |
| S03-T04 | Upsert & delete strictness (MySQL DELETE JOIN) | FR-204 | FS-M2-04 | `crates/rustavel-orm/src/{upsert,delete}.rs` | S | `upsert(rows, unique_by: [])` → `Err(EmptyUniqueBy)`; empty rows → `Ok({0,0})`; MySQL `DELETE JOIN` compiles; previously-silent ignore now throws |
| S03-T05 | Migrations & seeders (`make:migration`, `migrate`, `migrate:fresh`) | FR-208 | FS-M2-05 | `crates/rustavel-orm/src/migration.rs` + `database/migrations/` | M | `make:migration create_users_table` scaffolds `YYYY_MM_DD_HHMMSS_name.rs` with `up`/`down`; `migrate` idempotent; `migrate:fresh --seed` reversible; NFR-Rel-02 |
| S03-T06 | Factories + vector extension (`whereVectorSimilarTo`, `vector` column) | FR-207, FR-209, FR-210 | FS-M2-06 | `crates/rustavel-orm/src/{factory,vector}.rs` + Blueprint `vector` | M | `UserFactory::create(5)`; `whereVectorSimilarTo("vector",&emb,limit:10)` returns top-10 by cosine; dimension mismatch → `VectorDimensionMismatch`; missing extension → `PgVectorError::ExtensionMissing`; `FetchMode` (FR-210 Could) behind flag |

## 4. Dependencies

- **Upstream:** S01 (M0) + S02 (M1) must be stable; `sqlx` primary vs `sea-orm` shim decision (ADR-002) finalized in this sprint's spike if not already — gates S03-T03.
- **Downstream:** Blocks S04 (M3 — auth needs `users` table), S05 (M4 — `jobs`/`failed_jobs`/`cache` tables), S07 (M6 full vector/search).

## 5. Deliverables

- Crate `rustavel-orm` + `#[derive(Model)]` macro; `database/migrations/` + `database/seeders/`.
- Generators `make:model`/`make:migration`/`make:seeder` (runtime in S03; CLI wiring finalized in S06).
- Tag `v0.3.0`; `vector` feature documented with managed-DB workaround.

## 6. Acceptance (Sprint Done)

- [ ] `User` model → `migrate` → `Factory::create(&user)` → `whereVectorSimilarTo` top-10 cosine → `serde` round-trip preserves eager relations — all green in `cargo test` (isolated via `sqlx::test`).
- [ ] `chunkBy("id", 500)` over 10k rows without OOM; `paginate`/`cursor` correct.
- [ ] `upsert` with empty `uniqueBy` throws; `migrate` re-run is no-op; `migrate:fresh` reversible.
- [ ] R-01 resolved: `sqlx` primary committed, `sea-orm` shim behind flag; ADR-002 updated if needed.
- [ ] `cargo check -p rustavel-orm` does not pull `async-openai`; `cargo tree` audit green.

## 7. Risks

- R-01 ORM duality (score 15) — mitigation is the spike → ADR pre-close.
- R-06 `pgvector` extension missing on managed Postgres — guard `has_extension("vector")` + feature-flag fallback.
