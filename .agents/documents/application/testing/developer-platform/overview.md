# Testing: DeveloperPlatform (M5 — DX, CLI & Testing)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/developer-platform/overview.md](../../modules/developer-platform/overview.md)
> **BDD:** `@cli`, `@generators`, `@testing`, `@attributes` · **FSD:** FS-M5-01..04

## 1. Scope

Covers `cargo rustavel list --json` (+ `#[usage]`/`#[help]`/`#[hidden]`), all `make:*` generators (controller/model -m/provider/command/job/event/listener/observer/test/seeder/agent/tool, `AlreadyExists`, `rustfmt`+`clippy` gate), declarative attributes bundle (`#[tries]` shadowing trait), and the `TestCase` harness (`testcontainers` random ports + `Str` reset + teardown + `.env.testing` + `Artisan::call` + paginator).

## 2. Trace

- Stubs: `testing/stubs/m5-cli-testing.stub.rs` · `testing/stubs/harness.stub.rs`.
- BDD: `bdd-scenarios.md §2.6` (4 Features).
- QA: `qa-design §1.2` — M5 positive/contract/BVA rows.

## 3. Links

- Specs: [test-cli.md](test-cli.md)
- API: [api-cli](../../api/developer-platform/api-cli.md)
- Module: [cli](../../modules/developer-platform/cli.md) · [generators](../../modules/developer-platform/generators.md) · [testing-harness](../../modules/developer-platform/testing-harness.md)
