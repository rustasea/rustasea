# RustaSea — Starter-Kit & Documentation Audit

> **Status:** Active — 2026-09-11 | **Task:** TASK-003 (consolidates TASK-001 C1–C17 + TASK-002 starter-kit mapping)
> **Scope:** Documentation consistency between `README.md`, `docs/milestones.md`, and `.agents/documents/**`, plus the frontend/starter-kit gap.
> **Method:** Every finding re-verified against the source tree on `origin/master` as of 2026-09-11 — evidence is `path:line`, not prior task prose.
> **Status source of truth:** [`docs/milestones.md`](../../../docs/milestones.md) — M0/M1 **Partial**, M2 **Done**, M3–M6 **Partial** (`docs/milestones.md:40-46`). This audit never promotes a milestone beyond that matrix.
> **Companion design:** `.agents/documents/design/starter-kit-architecture.md` (Draft, TASK-004) · `docs/adr/ADR-0002-rustasea-starter-kit-architecture.md` (Proposed).

---

## 1. Purpose

This document consolidates the 17 findings produced by the TASK-001 audit into one
durable, re-verifiable register, and adds the Laravel starter-kit → RustaSea mapping
from TASK-002. It exists because the findings were previously only in task comments,
which are not part of the navigable doc tree.

The audit is **documentation-only**. It records drift and remediation; it does not
assert that any open item has been implemented.

---

## 2. Finding Register (C1–C17)

Types: **STALE** (outdated claim), **CONTRADICTION** (two docs disagree), **GAP**
(required surface absent). Severity: HIGH / MED / LOW.

| ID | Type | Sev | Finding | Evidence (`path:line`) | Resolution / status |
|---|---|---|---|---|---|
| **C1** | STALE | HIGH | README says the project is in "discovery"; real status is M2 Done + M0/M1/M3–M6 Partial. | `README.md:7` (`status-discovery`), `README.md:318` ("project is in discovery"), `README.md:155`, `README.md:177`; matrix `docs/milestones.md:40-46`; stale planning claim `requirements/brief.md:67` ("no code") | **Fixed in `README.md`** — badge → `status-alpha`, prose points to `docs/milestones.md`. `brief.md:67` remains historical (see C15). |
| **C2** | CONTRADICTION | MED | README uses `cargo artisan`; canonical command across requirements/design/ADR is `cargo rustasea`. | `README.md:32,111,121,130,131,132,141,151,162,163,164,173,189,194,264` (15 hits) vs `requirements/brief.md:28`, `requirements/brd.md:76`, `requirements/prd.md:46` (FR-005), `application/api/developer-platform/api-cli.md:10`, `application/modules/foundation/boot.md:114,127`, `docs/adr/ADR-0002-rustasea-starter-kit-architecture.md:24` | **Fixed in `README.md`** — all 15 replaced with `cargo rustasea`. |
| **C3** | CONTRADICTION | MED | README queue tech-stack row calls database/Redis drivers "in-process stubs"; real drivers + worker loop are live. | `README.md:196` vs `README.md:155` and `docs/milestones.md:44` (`crates/rustasea-queue/src/driver/database.rs:36`, `driver/redis.rs:30`, `driver/worker.rs:78`) | **Fixed in `README.md`** — row now describes live `database`/`redis` drivers + worker; only the Redis **cache** store remains stubbed (`crates/rustasea-cache/src/redis.rs:37`). |
| **C4** | GAP | MED | README crate inventory omits `rustasea-jsonapi` and the runnable `rustasea-app` crate. | Actual `crates/` listing; `crates/rustasea-jsonapi/` (cited `docs/milestones.md:272`, `application/api/intelligence-delivery/api-jsonapi.md:5`, `application/modules/intelligence-delivery/overview.md:5`); `crates/rustasea-app/Cargo.toml` (cited `docs/milestones.md:64`) | **Fixed in `README.md`** — both added to the `crates/` tree. |
| **C5** | GAP | MED | Historical finding: ADRs were split across two directories and numbering schemes. | [Canonical ADR index](../../../docs/adr/README.md); updated references in `requirements/fsd.md:495,498-499` and `requirements/tdd.md:308-309` | **Resolved by TASK-005** — `docs/adr/` is the canonical location with four-digit IDs; ADR-0001/0002 remain unchanged and the six migrated decisions are ADR-0003–ADR-0008. |
| **C6** | GAP | MED | `fsd.md`/`tdd.md` name `design/*.md` as P6-final parents, but those design docs still declare themselves Draft P3. | `requirements/fsd.md:493-505`, `requirements/tdd.md:305-315` vs `.agents/documents/design/architecture.md:3`, `design/domain.md:3`, `design/database.md:3`, `design/api-contracts.md:3` ("Draft — P3 (TASK-008)") | **Open** — either promote design docs to Final or downgrade the parent claim in fsd/tdd. |
| **C7** | STALE | LOW | `user-stories.md` and `bdd-scenarios.md` still say "Draft — P2" while sibling requirements docs are "Final — P6". | `requirements/user-stories.md:3`, `requirements/bdd-scenarios.md:3` vs `requirements/brd.md:3`, `prd.md:3`, `fsd.md:3`, `tdd.md:3` | **Open** — update status headers. |
| **C8** | GAP | HIGH | No presentation/starter-kit layer exists: no view engine, no Inertia/WASM client, no auth/session screens, no `cargo rustasea new`. | `resources/views/` holds one static file (`resources/views/welcome.html`, served via `routes/web.rs:16`); `README.md:199`, `README.md:233`; `application/modules/foundation/boot.md:117` ("No browser UI"); `docs/milestones.md:280`; `docs/laravel-parity.md:80` (`Illuminate\View` **Planned**); `requirements/brd.md:39` (Blade parity non-goal); no `package.json`/`vite.config.*`/`components.json` anywhere | **Open** — design exists (`.agents/documents/design/starter-kit-architecture.md`, `docs/adr/ADR-0002-...`); implementation is the SK-0…SK-6 plan. |
| **C9** | STALE | LOW | README and milestones pinned different raw commit SHAs for the same merged tree; both are volatile. | `README.md:155` vs `docs/milestones.md:29` | **Resolved by TASK-009** — all raw SHAs removed; docs now cite milestone/task + `path:line` with a dated header per [`docs/documentation-conventions.md`](../../../docs/documentation-conventions.md). |
| **C10** | GAP | MED | The `new` project scaffolder is listed as an M0/M5 deliverable but does not exist; canonical name is `cargo rustasea new`. | `docs/milestones.md:74,247` (Missing); `README.md:111`; `requirements/prd.md:46` (FR-005); `application/modules/foundation/boot.md:114,127`; design in `docs/adr/ADR-0002-...:24` | **Open** — implement `cargo rustasea new` (SK-4, M5) per ADR-0002; `artisan new` becomes an alias only. |
| **C11** | GAP | LOW | README directory tree omits the `rustasea-app` crate and the `xtask` workspace member. | `Cargo.toml:2` (`members = ["crates/*", "xtask"]`); `docs/milestones.md:64`; `crates/rustasea-app/Cargo.toml` | **Fixed in `README.md`** — both reflected in the tree (see C4). |
| **C12** | CONTRADICTION | MED | `application/README.md` states a blanket JSON:API content type; per-module API specs carve out `application/json` for auth and 422 validation. | `application/README.md:56` ("All HTTP endpoints use `application/vnd.api+json` … unless noted") vs `application/api/README.md:12`, `api/identity-access/api-auth.md:11`, `api/identity-access/api-validation.md:10`, `api/http-routing/api-routing.md:11`, `api/data-orm/api-query-builder.md:13` | **Open** — reword `application/README.md:56` to an explicit default + documented exceptions (the "unless noted" qualifier softens but does not resolve it). |
| **C13** | CONTRADICTION | MED | Crate counts drift across docs; no doc matches the workspace. | Actual: 21 crates under `crates/` + `xtask` (`Cargo.toml:2`). Claims: `README.md:245-263` (19, pre-fix), `application/presentation-brief.md:17` ("18 crates + umbrella"), `application/blueprint-audit.md:118` ("22 crates + xtask"), `application/README.md:40` ("20 crates"), `application/modules/manifest.md:11,17` | **Resolved by TASK-006** — the [canonical crate inventory](modules/manifest.md#canonical-crate-inventory-source-of-truth) records 21 crates under `crates/` + `xtask` (22 workspace packages); inventory summaries now reference it. Evidence in this row records the pre-fix claims. |
| **C14** | CONTRADICTION | MED | `rustasea-container` is listed as a separate crate but is actually a module inside `rustasea-foundation`. | **Verified:** no `crates/rustasea-container/`; `Container` struct at `crates/rustasea-foundation/src/lib.rs:35-41`. Wrong: `application/modules/manifest.md:11`, `application/README.md:44`, `requirements/fsd.md:497`. Correct: `application/modules/foundation/overview.md:5` ("inside foundation"). Omission: `requirements/brd.md:76`, `requirements/tdd.md:305-306` (BC-0) | **Resolved by TASK-006** — `Container` is a type inside `rustasea-foundation`, not a separate crate; the manifest, application README, FSD, and related design inventory now agree. |
| **C15** | STALE | MED | Planning docs still read as pre-implementation while as-built status is recorded elsewhere. | `application/presentation-brief.md:4` ("Discovery Complete"), `:156-162` (all milestones "Planned"); `requirements/brief.md:67` ("no code"); testing stubs `#[ignore]` (`application/testing/README.md`) vs `docs/milestones.md` (2026-09-11), `docs/laravel-parity.md` (2026-09-11), `tasks/sprints/sprint-06.md:3`, `sprint-07.md:3` ("In Progress — core surfaces landed") | **Open** — mark planning docs historical; `docs/milestones.md` is the live status source. |
| **C16** | GAP | LOW | Design docs self-declare Draft P3 while requirements are Final P6 and the blueprint audit already treats the ADR stack as Accepted. | `design/*.md:3` ("Draft — P3 (TASK-008)") vs `requirements/*.md:3` ("Final — P6"); `application/blueprint-audit.md:165` (ADRs Accepted), `:51` (D4 PASS) | **Open** — reconcile doc statuses (overlaps C6). |
| **C17** | STALE | LOW | Blueprint audit's requirements line counts are each exactly 6 short of the current files. | `application/blueprint-audit.md:65-72` vs actual `wc -l`: brief 73 (was 67), brd 184 (178), prd 323 (317), fsd 515 (509), tdd 321 (315), user-stories 844 (838), bdd-scenarios 1151 (1145), validation 305 (299) | **Open** — update counts or replace the column with an existence check. |

---

## 3. Laravel Starter-Kit Mapping (TASK-002)

Source of the Laravel side: `.agents/documents/design/starter-kit-architecture.md:32-40`
(Laravel 13 shared Fortify core + presentation-only variants). RustaSea-side evidence is
re-verified from this tree. Legend: **COVERED** / **PARTIAL** / **MISSING**.

### 3.1 Shared core

| Laravel starter-kit surface | RustaSea equivalent | State | Evidence (`path:line`) |
|---|---|---|---|
| `app/Http/Controllers` | `app/http/controllers` + `make:controller` | **COVERED** | `app/http/controllers/`; `crates/rustasea-cli/src/generators/mod.rs:19` |
| `app/Models` | `app/models` + `make:model` | **COVERED** | `app/models/`; `crates/rustasea-cli/src/generators/mod.rs:19` |
| `app/Console/Commands` | `app/console/commands` + `make:command` | **COVERED** | `app/console/commands/` |
| `app/Jobs` | `app/jobs` + `make:job` | **COVERED** | `app/jobs/` |
| `app/Events` / `app/Listeners` | `app/events` + `app/listeners` + `make:event`/`make:listener` | **COVERED** | `app/events/`, `app/listeners/` |
| `app/Actions/Fortify` | `app/actions/auth/*` | **MISSING** | no `app/actions/` (planned `starter-kit-architecture.md:244-247`) |
| `app/Concerns` | `app/concerns/*` | **MISSING** | no `app/concerns/` (planned `starter-kit-architecture.md:247`) |
| `app/Http/Requests/Settings` | `app/http/requests/settings/*` | **MISSING** | no `app/http/requests/`; `make:request` absent (`docs/milestones.md:248`) |
| `app/Http/Middleware` | `app/http/middleware` | **PARTIAL** | dir exists; no `make:middleware` (`docs/milestones.md:248`) |
| `bootstrap/app.php` + `providers.php` | `bootstrap/app.rs`, `bootstrap/providers.rs` | **PARTIAL** | `bootstrap/app.rs:15-21` no-op provider; `bootstrap/providers.rs:11` empty registry (`docs/milestones.md:67,71`) |
| `config/{fortify,inertia}.php` | `config/*.toml` | **PARTIAL** | loader reads only `config/app.toml` — `crates/rustasea-config/src/lib.rs:16` (`docs/milestones.md:68`) |
| `routes/{web,auth,settings,console}.php` | `routes/{web,auth,settings,console}.rs` | **PARTIAL** | only `routes/web.rs` exists |
| `database/{migrations,factories,seeders}` | `database/migrations`, `database/seeders`, runtime factories | **PARTIAL** | `database/migrations/`, `database/seeders/` present (`.gitkeep`); `database/factories/` absent; ORM runtime factory exists at `crates/rustasea-orm/src/factory.rs` |
| `tests/{Feature,Unit}` | `tests/feature`, `tests/unit` | **PARTIAL** | `tests/feature/` present (`.gitkeep`); `tests/unit/` absent |
| Auth (session + CSRF) | `rustasea-auth` JWT real, session guard placeholder | **PARTIAL** | `crates/rustasea-auth/src/session.rs:117,147`; `docs/milestones.md:43,172` |
| Artisan CLI (`make:*`) | `cargo rustasea list` + 13 `make:*` generators | **COVERED** | `crates/rustasea-cli/src/generators/mod.rs:19`; `docs/milestones.md:239` |
| `route:list` introspection | `route:list` CLI | **PARTIAL** | empty table — `crates/rustasea-cli/src/commands/inspect.rs:33` (`docs/milestones.md:103`) |
| `laravel new` / `composer create-project` | `cargo rustasea new <app>` | **MISSING** | `docs/milestones.md:74,247`; design `docs/adr/ADR-0002-...:24` |

### 3.2 Presentation variants

| Variant | Laravel delta | RustaSea plan | State | Evidence |
|---|---|---|---|---|
| **Blade** | `resources/views` | `rustasea-view` (askama default) | **PARTIAL** | `resources/views/welcome.html` static, served via `routes/web.rs:16`; no template engine — `docs/milestones.md:280`, `docs/laravel-parity.md:80` |
| **React** | `resources/js` + Inertia + shadcn + Wayfinder + Vite | Dioxus WASM + `rustasea-inertia` | **MISSING** | no `resources/js/`, `package.json`, `vite.config.*`, `components.json`; plan `starter-kit-architecture.md:160-172` |
| **Vue** | `resources/js` + Inertia + shadcn + Wayfinder + Vite | Leptos WASM + `rustasea-inertia` | **MISSING** | as above |
| **Livewire** | `resources/views` + Flux | askama + HTMX + `rustasea-broadcast` | **MISSING** | `resources/views/` has no partials; broadcast crate is real (`crates/rustasea-broadcast/src/lib.rs:36`) |

### 3.3 Additional starter-kit surfaces called out by TASK-002

| Surface | State | Evidence |
|---|---|---|
| `make:middleware` / `make:request` generators | **MISSING** | `docs/milestones.md:248`; `crates/rustasea-cli/src/generators/mod.rs:19` |
| `database/factories` + `make:factory` (app-level) | **MISSING** | `database/` has only `migrations/` + `seeders/`; ORM runtime factory at `crates/rustasea-orm/src/factory.rs` |
| `tests/unit` | **MISSING** | `tests/` has only `feature/` |
| Wayfinder typed-route generator | **MISSING** | no crate/generator found |
| 2FA / passkeys | **MISSING** | no auth surface found |
| `install:features` installer | **MISSING** | no CLI command found (`crates/rustasea-cli/src/commands/builtins.rs`) |

---

## 4. Top Blockers

Ordered by impact on the starter-kit goal.

1. **C8 — no presentation layer (HIGH).** Without a view engine and auth screens the
   framework cannot deliver a `laravel new`-class experience. Design is ready
   (`starter-kit-architecture.md` + ADR-0002); delivery is SK-0…SK-6.
2. **C10 — no `cargo rustasea new` scaffolder (MED).** The single entrypoint Laravel
   parity depends on is still missing; it is the first thing a new user runs.
3. **C1/C2/C3/C11 — README drift (HIGH/MED).** Stale status, wrong CLI name, and
   wrong crate list actively mislead readers. **Resolved in this task** for `README.md`.
   **C9** (volatile commit references) is **resolved by TASK-009** — see
   [`docs/documentation-conventions.md`](../../../docs/documentation-conventions.md).
4. **C14 — phantom container crate (MED).** **Resolved by TASK-006:** `Container`
   belongs to `rustasea-foundation`; the canonical inventory and related docs agree.
5. **C6/C7/C15/C16/C17 — doc lifecycle drift (MED/LOW).** Draft/Final markers
   and stale counts erode trust in the doc tree. **Open.** C5 ADR consolidation is
   **resolved by TASK-005**.
6. **Uncommitted starter-kit artifacts.** `docs/adr/ADR-0002-...` and
   `.agents/documents/design/starter-kit-architecture.md` exist on disk but are
   **untracked** (`git status`), so they are invisible to a fresh clone. Commit them
   with the C5/C8 remediation.

---

## 5. Remediation Backlog

| Finding(s) | Action | Owner | Target |
|---|---|---|---|
| C1, C2, C3, C4, C11 | README sync | TASK-003 (this task) | Done |
| C9 | Volatile commit-ref policy: milestone/task + dated header, no raw SHAs | TASK-009 | Done |
| C8, C10 | Implement starter kit (SK-0…SK-6) + `cargo rustasea new` | engineering | M5/M6 |
| C14 | Keep `Container` inside `rustasea-foundation`; reconcile inventory references | TASK-006 | Done |
| C5 | Consolidate ADRs in `docs/adr/` with four-digit IDs and a canonical index | TASK-005 | Done |
| C6, C16 | Reconcile `design/*.md` status with fsd/tdd parent claims | docs | next doc touch |
| C7 | Update `user-stories.md:3`, `bdd-scenarios.md:3` status headers | docs | next doc touch |
| C12 | Reword `application/README.md:56` content-type default + exceptions | docs | next doc touch |
| C13 | Canonical inventory: 21 crates under `crates/` + `xtask`; align document counts | TASK-006 | Done |
| C15 | Mark planning docs historical | docs | next doc touch |
| C17 | Update `blueprint-audit.md:65-72` line counts | docs | next doc touch |

---

## 6. Related Documents

- [`docs/milestones.md`](../../../docs/milestones.md) — authoritative milestone status (M0/M1 Partial, M2 Done, M3–M6 Partial).
- [`README.md`](../../../README.md) — synced in this task (C1, C2, C3, C4, C9, C11).
- `.agents/documents/design/starter-kit-architecture.md` — starter-kit design proposal (TASK-004).
- `docs/adr/ADR-0002-rustasea-starter-kit-architecture.md` — decision record for the starter-kit architecture.
- [`application/blueprint-audit.md`](blueprint-audit.md) — P8A D1–D6 blueprint audit.
- [`application/presentation-brief.md`](presentation-brief.md) — stakeholder brief (historical).
- [`application/modules/manifest.md`](modules/manifest.md) — canonical crate inventory and module index (C13/C14 resolved by TASK-006).
- [`docs/documentation-conventions.md`](../../../docs/documentation-conventions.md) — commit-reference policy (C9, TASK-009).
