# Documentation Conventions

> **Last updated:** 2026-09-11
> **Scope:** Conventions for the RustaSea documentation tree (`README.md`, `docs/**`, `.agents/documents/**`).

## Commit-reference policy

Documentation MUST NOT embed raw commit SHAs or volatile branch hashes. A SHA
stales as soon as history is rebased, squashed, or garbage-collected, and once
the same event is pinned differently in two documents the tree contradicts
itself (audit finding **C9** in
[`starter-kit-audit.md`](../.agents/documents/application/starter-kit-audit.md)).

Instead:

- Anchor status and evidence to a **milestone ID** (`M0`–`M6`) plus a **task ID**
  (`GAP-*`, `DOC-*`, `TASK-*`) and a `path:line` citation — these survive history
  rewrites.
- Stamp every status-bearing document with a dated
  `> **Last updated:** YYYY-MM-DD` header; that date is the point-in-time anchor
  for the claims below it.
- If a point-in-time commit anchor is genuinely required, state exactly **one**
  canonical `short-sha` (YYYY-MM-DD) in that document's header. Never restate it
  inline and never repeat it in another document.

Worked example — a claim that an event landed in a specific commit becomes:

- **Before:** "queue drivers landed in `commit <sha>`".
- **After:** "queue drivers are live — `crates/rustasea-queue/src/driver/database.rs:36`; task `GAP-005` (see [`docs/milestones.md`](milestones.md), 2026-09-11)."

## Related documents

- [`docs/milestones.md`](milestones.md) — authoritative milestone status (dated snapshot).
- [`README.md`](../README.md) — project overview and milestone goals.
- [`starter-kit-audit.md`](../.agents/documents/application/starter-kit-audit.md) — finding **C9** (stale commit references) and its resolution.
