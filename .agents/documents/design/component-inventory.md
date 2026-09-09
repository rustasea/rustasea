# RustaSea — Component Inventory (Crate & CLI Components)

> **Owner:** vheins/rustasea | **Phase:** P4 Design Planning (CLI & DX)  
> **Date:** 2026-09-07 | **Task:** TASK-009 (parent TASK-001)  
> **Parents:** `requirements/brd.md` + `requirements/prd.md` (FR-000 … FR-612) + `requirements/fsd.md` (FS-M0-01 … FS-M6-07) + `requirements/bdd-scenarios.md`  
> **Adaptation note:** This inventory adapts `design-specification` component-spec rules to a **Rust workspace / framework DX** context. "Components" are **crates, proc-macros, CLI commands, and prompt/output primitives** — not browser UI components. Each entry has PascalCase name, states (implemented as Rust states/errors), and reuse level.

---

## 1. Inventory — Workspace Crates (Umbrella + Milestone Crates)

Reuse levels: `P0 shared` (used by ≥3 milestones) · `P1 domain` (1–2 milestones) · `P2 feature-flagged` (opt-in).

| # | Crate (PascalCase) | Crate slug | Milestone | Reuse | Status | Depends On | Provides (public API surface) |
|---|--------------------|------------|-----------|-------|--------|------------|-------------------------------|
| 1 | `RustaSea` | `rustasea` | M0 | P0 shared | planned | all below (re-export) | Umbrella re-export — `pub use rustasea_foundation::*` etc. (like `laravel/framework`); feature-gated per domain |
| 2 | `RustaSeaFoundation` | `rustasea-foundation` | M0 | P0 shared | planned | `rustasea-config`, `rustasea-container` | `Application`, `ServiceProvider` (register→boot DAG), `Runner` (HTTP/Queue/Schedule), `AppState(Arc)` |
| 3 | `RustaSeaConfig` | `rustasea-config` | M0 | P0 shared | planned | — | Layered loader `config/*.toml` + env overlay + `.env` via `dotenvy`; typed `Config` via `serde`; `ConfigError` |
| 4 | `RustaSeaContainer` | `rustasea-container` | M0 | P0 shared | planned | — | `Container { Bind, Singleton, Instance, Make<T> }`, `Manager::extend` closure binding, `ContainerError` |
| 5 | `RustaSeaRouter` | `rustasea-router` | M1 | P0 shared | planned | `rustasea-foundation`, `rustasea-http` | `Route::get/post/.../any`, group `prefix/name/middleware`, `resource` helper, domain-route prioritization, route table + binding fields |
| 6 | `RustaSeaHttp` | `rustasea-http` | M1 | P0 shared | planned | `rustasea-foundation` | Typed extractors `Json<T>/Query<T>/Path<T>/State`, responses `Json/View/Redirect`, middleware `Throttle/Cors`, HTTP client `Http::get(...).throw(...).timeout(...)`, `HttpError` |
| 7 | `RustaSeaOrm` | `rustasea-orm` | M2 | P0 shared | planned | `rustasea-config`, `rustasea-macros` | Query builder (`where/orWhere/chunkBy/...`), `Model` trait, `Db/Transaction`, relations, `vector` column + `whereVectorSimilarTo`, migrations, seeders, `QueryError` |
| 8 | `RustaSeaMacros` | `rustasea-macros` | M2/M5 | P0 shared | planned | — (proc-macro) | `#[derive(Model)]`, `#[route]`, `#[middleware]`, `#[authorize]`, `#[validate]`, `#[tries]`/`#[backoff]`/`#[timeout]`/`#[usage]`/`#[help]`/`#[hidden]` (Laravel 13 #7); compile-time only |
| 9 | `RustaSeaAuth` | `rustasea-auth` | M3 | P1 domain | planned | `rustasea-http`, `rustasea-orm` | Guards `JwtGuard/SessionGuard`, `Auth::guard("jwt").login/parse/refresh/logout/user`, `Auth::extend`, `Error::GuardMismatch` |
| 10 | `RustaSeaValidation` | `rustasea-validation` | M3 | P1 domain | planned | `rustasea-macros` | `Rule` set, `ErrorBag { field -> Vec<ValidationError> }`, `#[validate]` wiring, strict `in_array`/`contains` helpers, `Validatable` |
| 11 | `RustaSeaQueue` | `rustasea-queue` | M4 | P1 domain | planned | `rustasea-foundation`, `rustasea-orm` | `Job<T>` trait, `ShouldRetry`, `Queue::route::<Job>(connection:, queue:)`, drivers `sync/database/redis`, `dispatch/chain/batch/delay/onQueue`, `failed_jobs`, `QueueError` |
| 12 | `RustaSeaCache` | `rustasea-cache` | M4 | P1 domain | planned | `rustasea-foundation` | `Store` trait (`get/put/touch/...`), `Repository` (`remember/forever/...`), `Lock` (`get/block/release`), stores `memory(moka)/redis`, `CacheError` |
| 13 | `RustaSeaEvents` | `rustasea-events` | M4 | P1 domain | planned | `rustasea-queue` | `Event` trait, `Listener { queue: Queue{enable} }`, `Dispatcher::dispatch/dispatchAfterResponse`, `JobAttempted{exception}`/`QueueBusy{connectionName}` |
| 14 | `RustaSeaSchedule` | `rustasea-schedule` | M4 | P1 domain | planned | `rustasea-cache`, `rustasea-events` | `Schedule::command(...).daily/cron/everyMinute/skipIfStillRunning/onOneServer`, `schedule:list/run/pause/resume`, `SchedulePaused/Resumed` events |
| 15 | `RustaSeaCli` | `rustasea-cli` | M5 | P0 shared | planned | all above via `AppState` | `clap` CLI (`list`, `route:list`, `migrate`, `queue:*`, `schedule:*`), `make:*` generators (12), prompts (`ask/secret/confirm/choice/multiSelect`), output (`table/progressBar/spinner`), `Shutdownable`, `Artisan::call` |
| 16 | `RustaSeaTesting` | `rustasea-testing` | M5 | P1 domain | planned | `rustasea-orm`, `rustasea-foundation` | `TestCase` harness, `testcontainers` PG/Redis isolation, per-package `.env.testing`, `Factory::create`, `Str` factory reset, paginator views |
| 17 | `RustaSeaBroadcast` | `rustasea-broadcast` | M6 | P2 feature-flagged | planned | `rustasea-http`, `rustasea-auth` | `ShouldBroadcast`, channel auth, WebSocket (`axum::extract::ws` + `tokio-tungstenite`), `Response::eventStream` (SSE), `BroadcastError` |
| 18 | `RustaSeaStorage` | `rustasea-storage` | M6 | P2 feature-flagged | planned | `rustasea-config` | `Storage/{disk, get, put, path}` over `object_store`/`tokio::fs`, read-through (primary+fallback, copy_back), `StorageError::PathTraversal` |
| 19 | `RustaSeaSearch` | `rustasea-search` | M6 | P2 feature-flagged | planned | `rustasea-orm`, `rustasea-ai` | `whereVectorSimilarTo`, `Str::toEmbeddings`, `dropVectorIndex`, embedding trait, `pgvector` driver |
| 20 | `RustaSeaAi` | `rustasea-ai` | M6 | P2 feature-flagged | planned | `rustasea-search`, `rustasea-queue`, `rustasea-broadcast` | `AiProvider` trait (12 providers), `Agent`/`Tool`, streaming/broadcast/queue/MCP, sub-agents/middleware, `AiError` |
| 21 | `RustaSeaJsonApi` | `rustasea-jsonapi` | M6 | P2 feature-flagged | planned | `rustasea-orm`, `rustasea-http` | `JsonApiResource` (sparse fieldsets, `include`, `links`, `Content-Type: application/vnd.api+json`), `JsonApiError::RelationNotLoaded` |
| 22 | `Xtask` | `xtask` | M0 | P0 shared | planned | `rustasea-cli` | Build tooling binary (`cargo xtask check/migrate/...`); not a framework crate, but part of CLI DX |

### Atomic Grouping (shared primitives)

| Primitive | Lives In | Reused By |
|-----------|----------|-----------|
| `AppState(Arc)` | `rustasea-foundation` | Every HTTP handler, middleware, test, CLI command that needs app context |
| `ContainerError` / `ConfigError` | `rustasea-container` / `rustasea-config` | Boot diagnostics; surfaced by every `M0` flow |
| `ErrorBag` | `rustasea-validation` | HTTP extractors + `show:model` + any `#[validate]` handler |
| `Store` trait | `rustasea-cache` | Cache, session, schedule `onOneServer` lock, queue metrics backend |
| `Job<T>` generic | `rustasea-queue` | Queue, events (async listeners), AI tool calls queued as jobs, notifications |
| `Tool` trait | `rustasea-ai` | AI agents, MCP discovery, `make:tool` scaffold |
| CLI prompt primitives | `rustasea-cli` | `new`, `migrate:fresh` (confirm), `make:*` (ask/choice), long-running workers (spinner) |

### Gap Analysis (what the inventory intentionally does NOT invent)

- No REST endpoint inventory (framework, not an app) — endpoints are generated per project via `routes/web.rs`.
- No browser component library (no Filament/Nova admin per `brd.md` Won't list — post-M6).
- No per-DB driver crates (drivers are feature flags inside `rustasea-orm`, not separate crates).
- No hosting/PaaS crate (framework ≠ platform).

---

## 2. CLI Component Inventory (Prompt & Output Primitives)

Each prompt/output primitive is a **CLI component** with states analogous to browser component states (resting/hover → idle/focused, loading → spinner, error → validation error, empty/disabled, etc.).

| # | Component (PascalCase) | Kind | Framework | Prompt Spec | States |
|---|------------------------|------|-----------|-------------|--------|
| 1 | `AskPrompt` | prompt — text input | `dialoguer::Input` | `ask("What is your name?") -> String` | **Resting:** prompt line with `>` cursor; **Focused:** cursor blink, input echo; **ValidationError:** `error: <msg>` below prompt + re-prompt; **Disabled:** N/A (prompts never disabled — they block); **Success:** value returned, prompt line replaced with `✔ <value>` |
| 2 | `SecretPrompt` | prompt — masked input | `dialoguer::Password` | `secret("Password?") -> String` (no echo) | **Resting:** prompt + `••••••` mask per keystroke; **Focused:** same; **Error:** same as Ask; **Success:** masked confirmation `✔ ••••••` |
| 3 | `ConfirmPrompt` | prompt — y/n | `dialoguer::Confirm` | `confirm("Proceed?") -> bool` | **Resting:** `Proceed? [y/N]`; **Focused:** waiting for `y`/`n`/`Enter`; **Error:** invalid char → re-prompt; **Success:** `true` → continue, `false` → abort exit 1 (see F-03 FreshConfirm) |
| 4 | `ChoicePrompt` | prompt — single select | `dialoguer::Select` | `choice("Pick one", &options) -> usize` | **Resting:** numbered list; **Focused:** arrow-key highlight (`❯ option`); **Active:** selection highlighted; **Disabled:** option greyed with `(unavailable)` suffix |
| 5 | `MultiSelectPrompt` | prompt — multi select | `dialoguer::MultiSelect` | `multiSelect("Pick many", &options) -> Vec<usize>` | **Resting:** checklist `[ ]`; **Focused:** arrow + `Space` toggles `[*]`; **Active:** toggled items checked; **Success:** `Vec` of indices |
| 6 | `TableOutput` | output — tabular | `comfy-table` / `tabled` | `table(headers, rows)` — adapts to terminal width per `flows.md` §3 | **Resting:** header + rows; **Loading:** N/A (instant); **Empty:** `No records found.` row with hint; **Error:** N/A — errors go to stderr, not table; **Overflow (narrow):** columns truncated with `…` |
| 7 | `ProgressBar` | output — progress | `indicatif::ProgressBar` | `progressBar(total, |bar| ...)` | **Resting:** `0/N`; **Loading:** `████░░░░ 42% (ETA 3s)` with elapsed; **Success:** `✔ Done in 4.2s`; **Error:** `✘ Failed at 12/20` with error line below; **Disabled:** hidden when `--json` (machine output has no bar) |
| 8 | `Spinner` | output — indeterminate | `indicatif::ProgressBar::new_spinner` | `spinner("Migrating...")` for long ops | **Loading:** `⠋ Migrating...` (braille spinner); **Success:** `✔ Migrated`; **Error:** `✘ Migration failed`; **Disabled:** hidden when `--json` |
| 9 | `JsonOutput` | output — machine | `serde_json` | `--json` flag on any command that has a table | **Resting:** valid JSON array/object to `stdout`; **Error:** `stderr` still human text even with `--json` (errors never JSON unless `--json` + structured error flag — TBD RFC) |
| 10 | `DidYouMean` | output — suggestion | `strsim` | Unknown command `make:controll` → `did you mean 'make:controller'?` | **States:** only one — suggestion line in `stderr` below `error: unknown command` |

### Form-like Composition (CLI arguments as a form)

The CLI itself is a **form** — typed `Args`/`Flags` per command via `clap` derive. Validation mirrors `form-design-specification` states.

| Field | Type | Required | Rules | Error message | Help text |
|-------|------|----------|-------|---------------|-----------|
| `<Name>` (e.g., `make:controller`) | `String` (PascalCase) | Yes | `^[A-Z][A-Za-z0-9]*$` (or `snake_case` for `make:migration`) | `error: invalid name 'foo_bar' — hint: use PascalCase e.g. UserController` | `The name of the controller class` |
| `--resource` / `-m` / `--force` | `bool flag` | No | flag present = true | N/A | Shown in `cargo rustasea make:controller --help` |
| `<id>` for `queue:retry` | `String` | Yes | non-empty, matches `failed_jobs.id` | `error: job 'abc' not found in failed_jobs` | `The ID of the failed job` |
| `--json` | `bool flag` | No | — | N/A | `Output as JSON` |
| `--all` (for `list`) | `bool flag` | No | — | N/A | `Show hidden commands` |

**Submission states (CLI invocation as form submit):**

- **Loading:** command executing — `Spinner` or `ProgressBar` shown (stderr is unbuffered, stdout buffered until complete for `--json`).
- **Success:** exit 0, `stdout` with table/JSON or `Created app/...`, no `stderr`.
- **Error:** exit 1, `stderr` with `error[E...]:` diagnostic + `hint:` line (see `flows.md` §5.4). On invalid args, `clap` prints `error: unexpected argument` before framework code runs.
- **Empty:** e.g., `route:list` with zero routes → table with `(no routes registered) — hint: define routes in routes/web.rs`.
- **Disabled:** hidden commands (`#[hidden]`) not shown in `list` unless `--all`; `queue:*` commands disabled (error) when queue driver not configured — `error: queue driver not configured — hint: set queue.driver in config/queue.toml`.

### Accessibility (CLI a11y)

- No `aria-*` — terminal a11y is **screen-reader via stdout order** + **colour not sole signal**: every colour-coded status (`✔`/`✘`) also has a text label (`Done`/`Failed`).
- `NO_COLOR` env respected (https://no-color.org) — when set, no ANSI colours emitted.
- `FORCE_COLOR` overrides for CI.
- Keyboard: `Tab` completion via `clap_complete` (generates shell completions for `bash`/`zsh`/`fish`); prompts support `Ctrl+C` abort uniformly.

---

## 3. Validation & Review Gates

| Gate | Status | Evidence |
|------|--------|----------|
| Component names use PascalCase | ✅ | All crate + prompt components listed as PascalCase |
| No invented components — derived from requirements only | ✅ | Every crate traces to PRD FRs; every prompt component traces to FSD FS-M5-01 prompt list |
| Validation simplest (clap derive, not runtime `any`) | ✅ | Typed `Args`/`Flags` via `clap` derive; `Job<T>` generics; `ErrorBag` typed |
| Error messages clear + actionable (code + hint) | ✅ | Every error has `error[E...]` + `hint:` per NFR-Usa-02 |
| Form states (loading/success/error/empty/disabled) defined for CLI args | ✅ | §2 form table + §2 prompt states |
| Prompt focus/management addressed | ✅ | Each prompt kind documents resting/focused/error/success; `Ctrl+C` uniform abort |
| Dismissal logic clear (terminal modals) | ✅ | Prompt dismissal = `Ctrl+C` / `n` / `Escape`; no backdrop — see `flows.md` §5.2 |
| Z-index / colour conflicts prevented | ✅ | Single terminal layer — no z-index; colour tokens use `NO_COLOR` escape hatch |

---

## 4. Cross-References

| Document | Link |
|----------|------|
| BRD/PRD/FSD | `../requirements/brd.md` / `../requirements/prd.md` / `../requirements/fsd.md` |
| Flows | `./flows.md` — every crate's commands have a user flow in flows.md |
| Design system | `./design-system.md` — tokens for scaffold naming, file paths, colours, spacing, error codes |

---

*Generated for TASK-009 · P4 Design Planning. Adapted from `design-specification` component-spec rules to CLI/framework DX context.*

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
