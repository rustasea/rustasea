# Skenario Pengujian: DeveloperPlatform (DX)

> Skenario pengujian untuk fitur DeveloperPlatform — CLI, Generators, Harness.
> Per `qa-design §1–2`, `bdd-gherkin` + `api-contract-test` + `migration-test` (see hint below) + `chaos`.

## Header & Navigation

- [Module Overview](../../modules/developer-platform/overview.md)
- [API Specification](../../api/developer-platform/api-cli.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| DX-POS-001 | US-M5-01 | list shows commands with usage | available CLI | `list --json` | entries `make:controller` + `migrate` with `usage` | High |
| DX-POS-002 | US-M5-01 | Declared usage shown in help | command `AppSend` with `#[usage("app:send {user}")]` | `list` help | usage line present | High |
| DX-POS-003 | US-M5-01 | Confirm can abort | command asks `confirm("Proceed?")` | answer `n` | abort non-zero | High |
| DX-POS-004 | US-M5-01 | Artisan::call in-process | `Artisan::call("migrate", vec![])` | in-process invoke | migration completes without subprocess (`xtask` probe) | High |
| DX-POS-005 | US-M5-02 | Controller generation lint-clean | fresh scaffold | `make:controller UserController` | `app/http/controllers/user_controller.rs` exists + `rustfmt`+`clippy -D warnings` pass | High |
| DX-POS-006 | US-M5-02 | Model with migration creates both artifacts | scaffold | `make:model Post -m` | `app/models/post.rs` (`#[derive(Model)]`) + migration for `posts` | High |
| DX-POS-007 | US-M5-03 | Tries attribute drives retry | `#[tries(3)] MyJob` always failing `sync` | dispatch | 3 attempts before `failed_jobs` | High |
| DX-POS-008 | US-M5-04 | Parallel tests distinct isolated stores | two `TestCase` impls | `cargo test -- --test-threads=2` | distinct random PG ports | High |
| DX-POS-009 | US-M5-04 | Factory sequence resets | seq 10 in prior test | new test `Factory::create(1)` | sequence index 1 (no leak) | High |
| DX-POS-010 | US-M5-04 | Hidden not shown default | command `#[hidden]` | `list` without `--all` | not shown | Medium |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| DX-NEG-001 | US-M5-01 | Hidden not shown default (edge) duplicate probe | `#[hidden]` present | `list --json` without `--all` | hidden not in result | Medium |
| DX-NEG-002 | US-M5-02 | Duplicate generation without force | `app/models/post.rs` exists | `make:model Post` without `--force` | `AlreadyExists{path}` | High |
| DX-NEG-003 | US-M5-03 | Attribute shadows trait with warning | `#[tries(3)]` + `ShouldRetry` returning false | fails | attribute wins with `tries attribute shadows ShouldRetry` warning | Medium |
| DX-NEG-004 | US-M5-01 | Unknown command did you mean | `make:controll` | invoke | suggests `make:controller` | Medium |
| DX-NEG-005 | US-M5-01 | Unknown migrat → migrate suggestion (table row 2) | `migrat` | invoke | suggests `migrate` | Medium |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| DX-MNK-001 | Lint gate | `rustfmt --check` + `clippy -D warnings` on all `make:*` outputs | controller/model/job/event/listener/agent/tool | green on every generator |
| DX-MNK-002 | Check incremental after `make:*` | `cargo check --timings` in `target/.generated-bench` | `make:model Post -m` | `<10s` incremental (NFR-Per-04) |
| DX-MNK-003 | Container startup timeout (30s) | `TestCase` spawn via `testcontainers` | PG/Redis spawn | `TestError::ContainerTimeout` beyond 30s (retryable) |
| DX-MNK-004 | Secret not echoed | `secret("Password?")` prompt | `secret` answer | not echoed; no leak in log |
| DX-MNK-005 | Shutdownable drain on SIGTERM for queue:work | `Shutdownable` worker | `SIGTERM` during `queue:work` | drains per foundation §4 |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| DX-SEC-001 | Developer | hidden command enumeration | `list --json --all` vs `--json` | hidden visible only with `--all` |
| DX-SEC-002 | Operator | `secret` echo leakage | probe log after `secret` | no password in log |
| DX-SEC-003 | Auditor | AlreadyExists path disclosure | `AlreadyExists{path}` | contains sanitized `app/...` not absolute host path |
| DX-SEC-004 | CI | `cargo check` incremental regression | nightly `make:*` matrix | 15% regression gate before PR merge |

 > **Note on migration testing terminology:** The phrase "migration testing scenarios" in the task description corresponds to `test-generation/rules/migration-test.md` (round-trip/idempotence/bulk/irreversible) — **not** database table `migrations`. Those scenarios are documented in [testing/data-orm/test-orm.md](../data-orm/test-orm.md) §5. This doc covers `TestCase` migration-running harness adjacency.


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
