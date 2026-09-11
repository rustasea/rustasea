# ADR-0009 — FSD Exceeds the 500-Line File Limit (Documented Exception)

> **Status:** Accepted
> **Date:** 2026-09-11
> **Deciders:** Tech Lead, Documentation
> **Milestone:** — (documentation governance)
> **Related:** `docs/documentation-conventions.md` · `requirements/fsd.md` · coding-standards §5 (max 500 lines/file) · TASK-018

## Context

The coding standard caps every file at **500 lines** (`wc -l`, excluding generated trees); exceeding it requires a split or a documented ADR exception. `.agents/documents/requirements/fsd.md` is **516 lines** and is a **Final — P6** requirements document archived under the historical `Rustavel` name for traceability.

The FSD is not ordinary prose: it is cited throughout the doc tree by **stable `path:line` references** (for example `starter-kit-audit.md` cites `fsd.md:495,498-499`, `fsd.md:493-505`, `fsd.md:3`, `fsd.md:497`) and by **section anchors** (`fsd.md §4.1`, `§5`, `§6`, `§8`). [`docs/documentation-conventions.md`](../../docs/documentation-conventions.md) makes `path:line` the canonical evidence form for this repository.

Splitting the file would renumber every line below the split point, silently invalidating those citations across multiple documents and violating the repository's path:line evidence policy. The document is frozen/archived, so a structural rewrite carries cost with no maintenance benefit.

## Decision

**Grant a documented exception: `requirements/fsd.md` remains a single 516-line file.** No split is performed. The exception is recorded here and cross-referenced from the FSD header, satisfying the coding-standard requirement that an over-limit file be covered by an ADR.

- The FSD is a finalized P6 planning artifact; no further feature specs are appended.
- New or materially changed requirements are authored in a **new** document rather than by extending `fsd.md`.
- Any future edit that grows the file, or any decision to split it, requires a superseding ADR that also updates all `path:line` citations.

## Alternatives

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| **Documented ADR exception (chosen)** | Preserves every `path:line` citation; zero churn on a frozen artifact; honors the standard's explicit escape hatch | File remains >500 lines, so line-count linters must exempt it | **Chosen** |
| Split by milestone (M0–M6 files) | Each file under 500 lines | Renumbers lines; invalidates dozens of `path:line` citations across the tree; requires re-pointing every reference | Rejected — breaks the repository's path:line evidence policy |
| Split with preserved line numbers (padding/anchors) | Line numbers stable | Artificial padding is misleading and unmaintainable; anchor continuity cannot survive a multi-file split | Rejected — cosmetic, not a real remediation |
| Rewrite FSD to <500 lines | Fully compliant | Rewrites a frozen, archived, traceable document; risks semantic drift | Rejected — no maintenance value |

## Consequences

- `requirements/fsd.md` is explicitly exempt from the 500-line rule until superseded; the exemption is discoverable via this ADR and the FSD header note.
- `path:line` citations into the FSD remain valid — no downstream document needs updating for this decision.
- Positive: evidence stability is preserved for audit and traceability chains (`fsd.md` → `tdd.md` → `blueprint-audit.md`).
- Negative: a naive repo-wide line-count check will flag `fsd.md`; it must exclude this ADR-documented file.
- Neutral: the exception is documentation-only and sets no precedent for code files, which remain bound by the 500-line split rule.

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
