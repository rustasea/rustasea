# Architecture Decision Records (ADRs)

Canonical home for all RustaSea ADRs. Files use the 4-digit convention
`ADR-XXXX-<slug>.md`; each ADR is immutable once accepted — superseding decisions
are recorded in a new ADR that references the old one.

## Index

| ID | Title | Status | Date | File |
|---|---|---|---|---|
| ADR-0001 | jsonwebtoken 10 and workspace MSRV 1.88 | Accepted | 2026-09-11 | [ADR-0001-jsonwebtoken-10-msrv-bump.md](ADR-0001-jsonwebtoken-10-msrv-bump.md) |
| ADR-0002 | RustaSea starter-kit architecture (shared core + presentation variants) | Proposed | 2026-09-11 | [ADR-0002-rustasea-starter-kit-architecture.md](ADR-0002-rustasea-starter-kit-architecture.md) |
| ADR-0003 | HTTP Framework: axum over actix-web | Accepted | 2026-09-07 | [ADR-0003-axum-vs-actix.md](ADR-0003-axum-vs-actix.md) |
| ADR-0004 | ORM: sqlx Primary, sea-orm Optional | Accepted | 2026-09-07 | [ADR-0004-sqlx-vs-sea-orm.md](ADR-0004-sqlx-vs-sea-orm.md) |
| ADR-0005 | Async Stack: Single tokio Runtime | Accepted | 2026-09-07 | [ADR-0005-tokio-stack.md](ADR-0005-tokio-stack.md) |
| ADR-0006 | Workspace Crate Boundaries per FR Domain | Accepted | 2026-09-07 | [ADR-0006-workspace-crates.md](ADR-0006-workspace-crates.md) |
| ADR-0007 | Facades Replaced by AppState Arc | Accepted | 2026-09-07 | [ADR-0007-appstate-over-facades.md](ADR-0007-appstate-over-facades.md) |
| ADR-0008 | Vector as Feature-Flagged Postgres Extension | Accepted | 2026-09-07 | [ADR-0008-vector-feature-flag.md](ADR-0008-vector-feature-flag.md) |
| ADR-0009 | FSD Exceeds the 500-Line File Limit (Documented Exception) | Accepted | 2026-09-11 | [ADR-0009-fsd-500-line-exception.md](ADR-0009-fsd-500-line-exception.md) |

## Legacy ID mapping

Before TASK-005 the ADRs lived in two locations: `docs/adr/` (4-digit) and
`.agents/documents/design/decisions/` (`ADR-001`…`ADR-006`). The latter were moved
here and renumbered:

| Legacy ID | Current ID |
|---|---|
| ADR-001 (axum vs actix) | ADR-0003 |
| ADR-002 (sqlx vs sea-orm) | ADR-0004 |
| ADR-003 (tokio stack) | ADR-0005 |
| ADR-004 (workspace crates) | ADR-0006 |
| ADR-005 (AppState over facades) | ADR-0007 |
| ADR-006 (vector feature flag) | ADR-0008 |
