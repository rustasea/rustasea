# Rustavel — Design System (Scaffold & Framework DX Tokens)

> **Owner:** vheins/rustavel | **Phase:** P4 Design Planning (CLI & DX)  
> **Date:** 2026-09-07 | **Task:** TASK-009 (parent TASK-001)  
> **Parents:** `requirements/brief.md` + `requirements/brd.md` + `requirements/prd.md` (C-04, NFR-Usa-02/03, FR-501, FR-103) + `requirements/fsd.md` (FS-M5-02, FS-M0-03) + `docs/laravel-13-research.md`  
> **Adaptation note:** Rustavel has no browser UI. This design system defines **scaffold tokens** (file naming, directory layout, generated code style) and **CLI DX tokens** (colours, spacing, typography for terminal output, error codes). It adapts `design-specification` design-system rules — the "brand" is the framework's developer experience. There is no Figma/MCP tool setup — `cargo` + `xtask` is the design tool.

---

## 1. Brand & Style Direction

| Field | Value |
|-------|-------|
| **Brand name** | Rustavel |
| **Primary concept** | Laravel ergonomics × Rust safety — the DX *is* the brand |
| **Style direction** | Minimal, convention-over-configuration, zero-cost ergonomics — same values as the brief. Scaffold must feel "Laravel `artisan make:*`" to an ex-Laravel developer, and feel "idiomatic Rust" to a Rustacean on first `cargo run`. |
| **Target framework** | Rust workspace (edition 2021, MSRV 1.80+) — `tokio` + `axum` + `sqlx` via `cargo xtask` + `clap` (derive) |
| **Dark mode** | Not applicable (no browser UI). CLI respects `NO_COLOR` (https://no-color.org) and `FORCE_COLOR` for terminal theming — see §4. |

---

## 2. Design Tokens

### 2.1 Scaffold Naming Tokens (the core of this system)

These tokens govern **every generated file and directory name**. Violating them breaks `cargo check` or convention.

| Token | Value | Usage | Example |
|-------|-------|-------|---------|
| `scaffold.crate.prefix` | `rustavel-` | Workspace crate names | `rustavel-router`, `rustavel-orm` |
| `scaffold.crate.umbrella` | `rustavel` | Re-export crate (like `laravel/framework`) | `use rustavel::prelude::*` |
| `scaffold.xtask.bin` | `xtask` | CLI entry binary (`cargo xtask` + `cargo rustavel` alias) | `cargo rustavel list` dispatches to `xtask` |
| `scaffold.dir.bootstrap` | `bootstrap/` | App wiring | `bootstrap/app.rs`, `bootstrap/providers.rs`, `bootstrap/commands.rs` |
| `scaffold.dir.config` | `config/` | Layered config (TOML) | `config/app.toml`, `config/database.toml` |
| `scaffold.dir.routes` | `routes/` | Route definitions | `routes/web.rs` |
| `scaffold.dir.database.migrations` | `database/migrations/` | Versioned migrations | `database/migrations/20260907_120000_create_users_table.rs` |
| `scaffold.dir.database.seeders` | `database/seeders/` | Seeders | `database/seeders/user_seeder.rs` |
| `scaffold.dir.app.http.controllers` | `app/http/controllers/` | Controllers | `app/http/controllers/user_controller.rs` |
| `scaffold.dir.app.http.middleware` | `app/http/middleware/` | Middleware | `app/http/middleware/authenticate.rs` |
| `scaffold.dir.app.models` | `app/models/` | Models | `app/models/user.rs`, `app/models/post.rs` |
| `scaffold.dir.app.providers` | `app/providers/` | Service providers | `app/providers/billing_provider.rs` |
| `scaffold.dir.app.console` | `app/console/commands/` | Console commands | `app/console/commands/send_emails.rs` |
| `scaffold.dir.app.jobs` | `app/jobs/` | Queue jobs | `app/jobs/send_email.rs` |
| `scaffold.dir.app.events` | `app/events/` | Events | `app/events/user_created.rs` |
| `scaffold.dir.app.listeners` | `app/listeners/` | Listeners | `app/listeners/send_welcome.rs` |
| `scaffold.dir.app.observers` | `app/observers/` | Observers | `app/observers/user_observer.rs` |
| `scaffold.dir.app.ai.agents` | `app/ai/agents/` | AI agents (M6) | `app/ai/agents/support_agent.rs` |
| `scaffold.dir.app.ai.tools` | `app/ai/tools/` | AI tools (M6) | `app/ai/tools/search_docs.rs` |
| `scaffold.dir.storage` | `storage/` | Runtime storage (git-ignored) | `storage/app/`, `storage/logs/` |
| `scaffold.dir.resources.views` | `resources/views/` | Templates (askama) | `resources/views/users/index.html` |
| `scaffold.dir.tests` | `tests/feature/` | Integration tests | `tests/feature/user_test.rs` |
| `scaffold.file.cargo` | `Cargo.toml` | Workspace manifest — `[workspace] members = ["crates/*"]` | — |
| `scaffold.file.env_example` | `.env.example` | Env template | Committed; `.env` is git-ignored |
| `scaffold.file.rustavel_toml` | `rustavel.toml` | Optional framework config | Feature flags, generator defaults |

### 2.2 File & Symbol Naming Conventions

| Token | Rule | Regex / Pattern | Rationale |
|-------|------|-----------------|-----------|
| `naming.crate` | kebab-case, `rustavel-` prefix | `^rustavel-[a-z0-9-]+$` | Cargo convention; workspace discoverability |
| `naming.file.rust` | snake_case, `.rs` ext | `^[a-z0-9_]+\.rs$` | `rustc` module resolution; `snake_case` is idiomatic |
| `naming.file.migration` | `YYYY_MM_DD_HHMMSS_snake_name.rs` | `^\d{4}_\d{2}_\d{2}_\d{6}_[a-z0-9_]+\.rs$` | Chronological ordering; lexicographic sort = execution order |
| `naming.struct` | PascalCase | `^[A-Z][A-Za-z0-9]*$` | Rust type convention; `UserController`, `SendEmail` |
| `naming.struct.controller` | Suffix `Controller` | `^[A-Z][A-Za-z0-9]*Controller$` | Mirrors Laravel `UserController`; `route:list` can filter by suffix |
| `naming.struct.job` | Descriptive noun-verb | `^[A-Z][A-Za-z0-9]*$` | `SendEmail`, `ProcessPodcast` — matches `Queue::route::<SendEmail>` |
| `naming.struct.event` | Past-tense noun | `^[A-Z][A-Za-z0-9]*$` | `UserCreated`, `OrderShipped` — event naming |
| `naming.table` | snake_plural | `^[a-z_]+s$` (plural) | Laravel convention; `User` → `users`, `Post` → `posts`; `#[table("users")]` is override escape hatch |
| `naming.route.name` | dot-namespaced snake | `^[a-z0-9_.]+$` | `users.index`, `api.v1.posts.show` — group prefix concatenation |
| `naming.route.path` | kebab/snake, leading `/` | `^/[a-z0-9/_\-{}:]*$` | `/users`, `/users/{user:slug}` (binding field syntax) |
| `naming.command.signature` | `kebab:snake` | `^[a-z][a-z0-9-:]*$` | `make:controller`, `queue:work`, `schedule:pause` — matches `clap` subcommand |
| `naming.config.key` | snake_case, dot path | `^[a-z_]+(\.[a-z_]+)*$` | `app.port`, `database.url`, `cache.prefix` — maps to `APP_PORT` env via `APP__PORT` (config crate) |
| `naming.env.key` | SCREAMING_SNAKE | `^[A-Z][A-Z0-9_]*$` | `DATABASE_URL`, `APP_PORT`, `CACHE_PREFIX` — process env |
| `naming.error_code` | `E` + 4 digits | `^E\d{4}$` | `E0101 AlreadyExists`, `E0201 RouteConflict` — stable for programmatic matching |
| `naming.cache_prefix` | hyphenated | `.*-cache-.*` | `-cache-`, `-session-` per Laravel 13 #12 — not `_cache_` |

### 2.3 CLI Colour Tokens (Terminal Output)

Terminal colours are the "palette" for this framework. All CLI output uses these tokens via `anstyle`/`owo-colors` or `clap`'s colour system.

| Token | Light (ANSI) | Dark (ANSI 256 / NO_COLOR) | Usage |
|-------|--------------|----------------------------|-------|
| `color.primary` | `cyan` (36) | `bright-cyan` (96) / no ANSI when `NO_COLOR` | Command names in `list`, highlighted paths in `route:list` |
| `color.primary-hover` | `bright-cyan` (96) | `cyan` (36) | Hover not applicable — used for `--help` emphasis |
| `color.success` | `green` (32) `#16a34a` | `bright-green` (92) `#22c55e` | `✔` checkmarks, `Created app/...`, `Migrated:` lines, success exit |
| `color.warning` | `yellow` (33) `#d97706` | `bright-yellow` (93) `#f59e0b` | `warning:` prefix, `cargo check` advisory after `new`, shadowing attribute warnings |
| `color.error` | `red` (31) `#dc2626` | `bright-red` (91) `#ef4444` | `error[E...]:` prefix, `✘` marks, `AlreadyExists` diagnostics |
| `color.info` | `blue` (34) `#0284c7` | `bright-blue` (94) `#38bdf8` | `hint:` lines, `info:` lines, `did you mean?` suggestions |
| `color.neutral-50` | `white` (37) on dark bg | `black` (30) on light bg | Table borders, muted help text |
| `color.neutral-900` | `white` (97) / `black` (30) depending on bg detection | — | Body text, table cells |
| `color.muted` | `bright-black` (90) / `dim` | `dim` | Timestamps, secondary info (` (0 pending)`) |

**WCAG note:** Terminal contrast is governed by the user's terminal theme, not the framework. The framework must not rely on colour as sole signal — every coloured status (`✔`/`✘`/`warning:`) also has a text label. `NO_COLOR` must disable all ANSI codes.

### 2.4 Typography Tokens (Generated Code Style)

These tokens define the **typographic rules for generated Rust code** — not browser fonts.

| Token | Value | Usage |
|-------|-------|-------|
| `code.font` | Monospace — whatever `rustfmt` uses | Generated `.rs` files are `rustfmt`-formatted — no manual font choice |
| `code.line_length` | `100` (rustfmt `max_width = 100`) | `rustfmt.toml` default — enforced in CI |
| `code.indent` | `4 spaces` | `rustfmt` default; tabs never emitted |
| `code.import_order` | `std` → external crates → `crate::` → `super::` | `rustfmt` `imports_granularity = Crate` ; reordered by `rustfmt` on generation |
| `code.doc_comment` | `///` for items, `//!` for modules | Doc comments on generated structs explain the scaffold (e.g., `/// Controller for User resource.`) |
| `output.table.font` | Monospace, terminal-dependent | `comfy-table` renders with `─ │ ┌ ┐` box-drawing; falls back to `+ - |` when `NO_COLOR` or non-UTF8 locale |
| `output.json.indent` | `2 spaces` | `serde_json::to_string_pretty` for `--json` output |

### 2.5 Spacing & Radius Tokens (Scaffold Layout Spacing)

| Token | Value | Usage |
|-------|-------|-------|
| `spacing.scaffold.indent` | `4` (spaces per dir level in file trees) | `cargo rustavel list --verbose` file trees; `new` success output |
| `spacing.cli.gap` | `2` (spaces between table columns) | `route:list` table column gap |
| `spacing.cli.section` | `1 blank line` between sections | `list` groups (`make:*` vs `migrate` vs `queue:*`) separated by blank line |
| `spacing.code.blank_between_items` | `1 blank line` | Generated code: blank line between `impl` blocks, between `use` groups |
| `radius` | N/A (no browser UI) | — |
| `shadow` | N/A (no browser UI) | — |

### 2.6 Breakpoints (Terminal Width — Responsive CLI)

| Breakpoint | Width | Behaviour |
|------------|-------|-----------|
| `bp.narrow` | `< 80 cols` | Compact tables; truncate `middleware` to count; help text wraps |
| `bp.standard` | `80 – 120 cols` | Full tables; inline binding fields |
| `bp.wide` | `≥ 120 cols` | Table + `handler` file:line column for `route:list`; diagnostics on one line |

*Identical to `flows.md` §3 responsive rules — single source of truth is here; flows.md references this table.*

---

## 3. Semantic Tokens (Usage-Mapped)

| Semantic | Token | Example |
|----------|-------|---------|
| Success | `color.success` + `✔` glyph | `✔ Created app/http/controllers/user_controller.rs` |
| Warning (advisory) | `color.warning` + `warning:` | `warning: cargo check failed — run 'cargo check' in demo for details` |
| Error | `color.error` + `error[E####]:` | `error[E0101]: AlreadyExists — app/models/post.rs already exists` |
| Hint | `color.info` + `hint:` | `hint: use --force to overwrite` |
| Info / suggestion | `color.info` + `did you mean?` | `did you mean 'make:controller'?` |
| Muted meta | `color.muted` | ` (0 pending)` after `Nothing to migrate` |
| Table header | `color.neutral-900` bold | `METHOD  PATH  NAME  MIDDLEWARE` |
| Table cell | `color.neutral-900` | Row values |

---

## 4. Error Code Registry (Stable Codes for DX)

Every framework diagnostic carries a stable `E####` code per `NFR-Usa-02` and `flows.md` §5.4.

| Code | Name | When emitted | Hint |
|------|------|--------------|------|
| `E0101` | `AlreadyExists` | `make:*` target file exists without `--force` | `use --force to overwrite` |
| `E0102` | `InvalidName` | `new` or `make:*` name fails naming regex | `use PascalCase e.g. UserController` or `use kebab-case e.g. my-app` |
| `E0103` | `UnknownGenerator` | `make:foo` where `foo` not in generator table | `did you mean 'make:tool'?` via `strsim` |
| `E0201` | `RouteConflict` | Duplicate `name("users.index")` at router build | `rename one route` |
| `E0202` | `InvalidPattern` | Route path fails `^/[a-z0-9/_\-{}:]*$` | `check path syntax` |
| `E0203` | `AmbiguousDomain` | Two domain routes match same host+path | `order domain routes explicitly` |
| `E0301` | `MissingTable` | Query against un-migrated table | `run cargo rustavel migrate` |
| `E0302` | `UpsertEmptyUniqueBy` | `upsert(rows, unique_by: [])` | `provide at least one unique column` |
| `E0303` | `VectorDimensionMismatch` | `whereVectorSimilarTo` dim mismatch | `expected 1536, got 768` |
| `E0304` | `PgVectorExtensionMissing` | Migration with `vector` column but `CREATE EXTENSION vector` failed | `install pgvector or disable vector feature` |
| `E0401` | `GuardMismatch` | `Auth::guard("api")` where only `jwt` registered | `expected jwt, got api` |
| `E0402` | `UntrustedOrigin` | `Sec-Fetch-Site: cross-site` with untrusted `Origin` | `add origin to csrf_origins in config/app.toml` |
| `E0403` | `NotAllowed` | Deserialization type not in `serializable_classes` allow-list | `add type to serializable_classes` |
| `E0501` | `DuplicateRoute` (Queue) | `Queue::route::<Job>` registered twice | `register each job type once` |
| `E0502` | `UnknownConnection` | `onConnection("bad")` where connection not configured | `check config/queue.toml` |
| `E0503` | `PathTraversal` | `Storage::path("../../etc/passwd")` | `path must stay under disk root` |
| `E0601` | `ModelNotFound` | `show:model Foo` where `Foo` not derived | `check app/models/foo.rs and #[derive(Model)]` |
| `E0602` | `RelationNotLoaded` | `JsonApiResource::include("posts")` without `with("posts")` | `eager-load with with("posts")` |
| `E0701` | `MigrationAlreadyApplied` | `migrate` re-applies known migration | `nothing to do — 0 pending` (not an error; advisory) |
| `E0702` | `ContainerNotFound` | `Make::<T>` where `T` not bound | `register binding in provider register()` |

*Codes are grouped by milestone prefix: `01` scaffold/generators, `02` routing, `03` ORM, `04` auth/validation, `05` queue/cache/schedule, `06` observability/storage, `07` migrations/container.*

---

## 5. Framework Export — `rustfmt.toml` & `clippy.toml` Tokens

Every generated project ships these configs — they are the "framework export" for this design system.

```toml
# rustfmt.toml — scaffold token export
max_width = 100
hard_tabs = false
tab_spaces = 4
edition = "2021"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
wrap_comments = true
comment_width = 100
```

```toml
# Cargo.toml [lints.clippy] — scaffold token export
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
pedantic = { level = "warn", priority = -1 }
```

Generated code must pass `rustfmt --check` and `clippy -- -D warnings` — see `NFR-Usa-03` and `C-04`. The generator itself runs `rustfmt` post-render as a post-processing step (see `flows.md` F-02).

---

## 6. Validation Gates

| Gate | Status | Evidence |
|------|--------|----------|
| Simplest token architecture — no redundant tokens | ✅ | One token namespace per concern (`scaffold.*`, `naming.*`, `color.*`, `code.*`); no duplicate colour scales |
| Dark mode / `NO_COLOR` handling defined | ✅ | §2.3 — `NO_COLOR` disables ANSI; colour never sole signal |
| Framework export syntax valid (`rustfmt.toml`, colour ANSI codes) | ✅ | §5 — `rustfmt.toml` is valid TOML; colour tokens are ANSI codes verifiable via `NO_COLOR` test |
| Semantic colours follow a11y (not just brand) | ✅ | §3 semantic mapping; §2.3 WCAG note; `✔`/`✘` + text label dual signal |
| Config environment-specific | ✅ | `config/*.toml` + `.env` + process env layering per `brief.md`; tokens reference this layering |
| No MCP/API token handling needed | ✅ | Framework is `cargo`-local; no external design tool tokens — `cargo xtask` is the design tool |
| Scaffold naming tokens cover every generated path | ✅ | §2.1 covers all 22 scaffold directories/files including M6 `ai/` dirs |
| Error codes stable and documented | ✅ | §4 — `E####` registry with 20 codes grouped by milestone |

---

## 7. Cross-References

| Document | Link |
|----------|------|
| Brief / BRD / PRD | `../requirements/brief.md` / `../requirements/brd.md` / `../requirements/prd.md` (C-04, NFR-Usa-02/03) |
| FSD | `../requirements/fsd.md` (FS-M5-02 generator table, FS-M0-03 config) |
| Flows | `./flows.md` — every scaffold path and CLI flow uses naming tokens from here |
| Component inventory | `./component-inventory.md` — every crate slug uses `naming.crate` token |
| Tech stack | `README.md` §Tech Stack + `docs/laravel-13-research.md` |

---

*Generated for TASK-009 · P4 Design Planning. Adapted from `design-specification` design-system rules to CLI/framework DX — scaffold & naming tokens are the "palette" for a framework with no browser UI.*
