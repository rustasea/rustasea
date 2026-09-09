# ADR-006 — Vector as Feature-Flagged Postgres Extension

> **Status:** Accepted
> **Date:** 2026-09-07
> **Deciders:** Tech Lead, Data
> **Milestone:** M2/M6 (initial primitive + full integration)
> **Related:** BR-03/BR-07 · FR-207/FR-602 · FS-M2-06/FS-M6-07 · database.md §4–5 · tdd.md BC-2/BC-6

## Context

RustaSea must support `whereVectorSimilarTo`, `vector(1536)` Blueprint columns, and `Str::toEmbeddings`/`AiProvider::embeddings` (Laravel 13 #6, FR-207/602). The canonical store is `pgvector` on Postgres, but the framework also supports MySQL and SQLite (FR-200) where vectors are not universally available. Making `pgvector` mandatory would break MySQL/SQLite consumers and managed-DB users where `CREATE EXTENSION vector` requires elevated privileges (risk R-06). A decision is needed on how vector support is gated without fragmenting the ORM API.

Constraints: BR-08 incremental adoption; `cargo check -p rustasea-orm --no-default-features` must build without `pgvector`; vector code is feature-flagged; embedding dimension mismatches are typed errors.

## Decision

**`pgvector` is behind a `vector` feature flag on `rustasea-orm`; MariaDB vector is a second `mariadb-vector` flag. Core builds without vector. Migrations guard with `has_extension("vector")` + typed `ExtensionMissing` diagnostic.**

- `rustasea-orm/Cargo.toml`: `vector = ["dep:pgvector"]` optional; `#[cfg(feature="vector")]` gates `Blueprint::vector`, `whereVectorSimilarTo`, and HNSW/IVFFLAT index helpers.
- Migration `up()` checks `has_extension("vector")` before creating `VECTOR(n)` columns; failure surfaces `MigrationError::ExtensionMissing { extension: "vector", hint }` with managed-DB workaround docs.
- Distance operators: `whereVectorSimilarTo` emits `ORDER BY col <=> $1 LIMIT k` (cosine); alternatives `<->` (L2) / `<#>` (IP) selectable via API.
- `Str::toEmbeddings` (M6) calls `AiProvider::embeddings`; dimension mismatch with column DDL is `VectorDimensionMismatch { expected, actual }`.
- `dropVectorIndex` is a no-index fallback — query still works via sequential scan until reindexed.

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **Feature-flagged pgvector + guard (chosen)** | Works on MySQL/SQLite without vector; managed-DB diagnostic is explicit; `cargo tree` isolation preserved | Consumers must enable `features=["vector"]` to use vectors | **Chosen** |
| Mandatory pgvector | Simplest API — no cfg | Breaks MySQL/SQLite consumers; filters target audience to Postgres-only | Rejected — violates FR-200 multi-driver |
| Separate `rustasea-vector` crate | Clean isolation | Adds 19th crate; vector queries need ORM builder — cross-crate coupling anyway | Rejected — flag isolation is sufficient per ADR-004 |
| Runtime driver detection (no compile flag) | No Cargo feature matrix | Runtime branching hides missing-extension errors until query time | Rejected — migration guard needs compile-time cfg |

## Consequences

- Default `cargo check -p rustasea-orm` pulls no `pgvector`; `cargo check -p rustasea-orm --features vector` pulls `pgvector 0.5+` and HNSW index support.
- `database.md §4` documents `has_extension` guard and HNSW/IVFFLAT/`dropVectorIndex` strategies.
- Positive: non-Postgres projects pay no vector cost; `cargo tree` incremental adoption (NFR-Sca-02) includes vector isolation.
- Negative: docs must cover managed-DB `CREATE EXTENSION` privilege workarounds; mitigated by diagnostic hint.
- Neutral: vector index choice (HNSW vs IVFFLAT) is DDL-level, not trait-level — no API churn.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
