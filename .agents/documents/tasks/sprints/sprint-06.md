# Sprint 06 — M5 DX, CLI & Testing

> **Milestone:** M5 · **Window:** 2027-05-15 → 2027-08-31 · **Status:** In Progress — core surfaces landed (M5 generators, M6 broadcast/search/storage/ai), drivers/SDKs pending · Delivered ahead of 2027 window per 2026-09 automation (core at bdcb18c/63c9e66)
> **Parents:** `../roadmap.md` · `prd.md` FR-500–FR-509 · `fsd.md` FS-M5-01–FS-M5-07 · `design/architecture.md`
> **Depends On:** M0–M4 (Sprints 01–05)
> **Crates:** `rustavel-cli`, `rustavel-macros` (generators + attrs), `rustavel-testing`

---

## 1. Goal

First-class developer experience: `cargo rustavel` CLI, `make:*` generators, and a testing harness that feels like Laravel — so the inner loop (generate → code → test) is fast and `rustfmt`/`clippy`-clean.

## 2. Scope (In / Out)

**In:**
- `cargo rustavel` CLI via `clap` derive + `cargo xtask`: `list` command, typed args/flags per command, `#[usage]`/`#[help]`/`#[hidden]` for commands, programmatic `Artisan::call(command, args)`
- `make:*` generators: `controller`, `model`, `provider`, `command`, `job`, `event`, `listener`, `observer`, `test`, `seeder`, `agent`, `tool` — all producing `rustfmt` + `clippy -D warnings` clean code into `app/*` + `database/*` + `tests/*`
- Declarative attributes `#[middleware]`/`#[authorize]`/`#[tries]`/`#[backoff]`/`#[timeout]`/`#[failOnTimeout]`/`#[withoutBroadcasting]` etc. matching Laravel 13 #7 surface (runtime in S02/S04/S05; S06 finalizes macro surface + `make:*` wiring)
- Interactive prompts `ask`/`secret`/`confirm`/`choice`/`multiSelect` + output helpers `table`/`progressBar`/`spinner` (`dialoguer`/`indicatif`)
- Graceful shutdown for commands via `Shutdownable` trait (long-running workers `queue:work`, `schedule:run`)
- Testing: `TestCase` harness + per-package `.env.testing` + isolated DB/cache via `testcontainers` + `sqlx::test` (per-test + per-file DB, random ports, teardown) + `Factory::create` ergonomic + `Str` factory sequence resets + paginator `bootstrap-3` views
- `bootstrap/commands.rs`, `tests/feature/`

**Out:**
- Advanced generators `make:agent`/`make:tool` runtime — M6 (Sprint 07) provides the AI trait they scaffold against; S06 provides the generator scaffolds behind `ai` feature.

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S06-T01 | `cargo rustavel` CLI core (`clap` derive + `xtask` + `list` + typed args/flags) | FR-500, FR-502 | FS-M5-01/02 | `crates/rustavel-cli/src/{cli,commands,artisan}.rs` + `cargo xtask` manifest | M | `cargo rustavel list --json` emits JSON with command names + signatures; typed args/flags via proc-macro; `#[usage("app:send {user}")]` rendered in `list` |
| S06-T02 | `make:*` generators (controller/model/provider/command/job/event/listener/observer/test/seeder/agent/tool) | FR-501, FR-608 (agent/tool scaffolds behind flag) | FS-M5-03 | `crates/rustavel-cli/src/generators/*.rs` + `rustavel-macros` generator hooks | L | `make:controller UserController` → `app/http/controllers/user_controller.rs` `rustfmt`+`clippy` clean + `cargo check` passes; all 12 `make:*` variants produce compiling code; `make:test` scaffolds `tests/feature/<name>.rs` |
| S06-T03 | Prompts + output helpers (`ask`/`secret`/`confirm`/`choice`/`multiSelect` + `table`/`progressBar`/`spinner`) | FR-503 | FS-M5-04 | `crates/rustavel-cli/src/{prompt,output}.rs` (`dialoguer`/`indicatif`) | S | `confirm("Proceed?")` with `n` aborts; `choice`/`multiSelect` return typed selections; `table`/`progressBar`/`spinner` render in TTY and degrade in non-TTY/CI |
| S06-T04 | `Shutdownable` + `Artisan::call` + command attributes `#[hidden]`/`#[usage]` | FR-504, FR-505, FR-502 | FS-M5-05 | `crates/rustavel-cli/src/{shutdown,artisan}.rs` | S | `queue:work` with `SIGTERM` drains then exits 0; `Artisan::call("migrate", vec![])` in test runs in-process; `#[hidden]` omits from `list` |
| S06-T05 | Declarative attributes finalization (`#[middleware]`/`#[authorize]`/`#[tries]`/`#[backoff]`/`#[timeout]`/… #7 surface) | FR-506 | FS-M5-06 | `crates/rustavel-macros/src/attrs.rs` (consolidated) | M | All #7 attrs from FR-506 applied and exercised: `#[tries(3)]` on `MyJob` retry semantics observed; `#[withoutBroadcasting]` suppresses broadcast |
| S06-T06 | Testing harness (`TestCase` + `testcontainers` + `.env.testing` + factories + `Str` reset + paginator views) | FR-507, FR-508, FR-509 | FS-M5-07 | `crates/rustavel-testing/src/{test_case,factory,containers}.rs` + `tests/feature/` example suite + paginator views | M | Two test files with `TestCase` run in parallel on isolated Postgres random ports without collision; `Factory::create` ergonomic; factory sequence resets between tests (score: sequence 10 → next test starts at 1); paginator `bootstrap-3` view renders HTML; `harness.stub.rs` contract passes |

## 4. Dependencies

- **Upstream:** S01–S05 (all prior crates) — S06 is the aggregator milestone (depends on M0–M4 per README). Generators touch every domain crate.
- **Downstream:** Blocks S07 (M6) — `make:agent`/`make:tool` scaffolds land here; S07 makes them functional.

## 5. Deliverables

- Crates `rustavel-cli`, `rustavel-testing` + consolidated `rustavel-macros` attrs.
- `bootstrap/commands.rs`, `tests/feature/` directory, 12 `make:*` generators.
- Tag `v0.6.0`; CLI stable surface documented; `cargo xtask ci` covers `rustfmt`+`clippy` on generated code.

## 6. Acceptance (Sprint Done)

- [ ] `make:controller UserController` scaffolds controller with route that compiles and appears in `route:list`.
- [ ] All `make:*` outputs are `rustfmt`+`clippy -D warnings` clean; `cargo check` incremental <10s (NFR-Per-04).
- [ ] `cargo rustavel list --json` emits signatures; `Artisan::call` in-process migration green.
- [ ] `cargo test` spins isolated Postgres via `testcontainers`, tears down; parallel suites do not collide; `Str` factories reset per test.
- [ ] `Shutdownable` long-running commands drain on `SIGTERM`; paginator views render.

## 7. Risks

- R-08 `Str` factory / `testcontainers` state leaks across parallel tests — harness resets per test + per-worker PG ports.
- R-07 Scope creep (M6 stories entering M5) — strict MoSCoW gate; RFC required to promote M6 FR to Must.
