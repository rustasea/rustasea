# Skenario Pengujian: Foundation (FOUND)

> Skenario pengujian untuk fitur Foundation — Boot, Container, Config & Shutdown.
> Per `test-planning/qa-design` §1 + `test-generation/bdd-gherkin` + `test-generation/api-contract-test` + `non-functional-testing/chaos`.

## Header & Navigation

- [Module Overview](../../modules/foundation/overview.md)
- [API Specification](../../api/foundation/api-bootstrap.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| FOUND-POS-001 | US-M0-01 | Boot DAG-ordered providers | provider Auth depends_on=[Config] | `Application::configure().providers([Config,Auth]).boot()` | `AppState` available, `Config::boot` before `Auth::boot` | High |
| FOUND-POS-002 | US-M0-01 | Layered config prefers env over file | `config/app.toml` port 3000, `.env` APP_PORT 4000 | `AppState::config::<AppConfig>().port` | `4000` | High |
| FOUND-POS-003 | US-M0-02 | Singleton preserves identity | `Singleton::<Counter(0)>` registered | `make::<Counter>()` twice | `Arc::ptr_eq` true, shared counter increments visible | High |
| FOUND-POS-004 | US-M0-02 | Manager::extend closure bound | cache driver via `extend(\|mgr,_\| MyStore(mgr.prefix()))` | `make::<CacheStore>()` | driver sees configured prefix | Medium |
| FOUND-POS-005 | US-M0-01 | Graceful drain completes | `GET /slow` 3s + `SIGTERM` + `shutdown_timeout=10s` | `SIGTERM` mid-request | `200`, exit `0` (NFR-Rel-01) | High |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| FOUND-NEG-001 | US-M0-01 | Circular dependency rejected | provider A→B, B→A | `boot()` | `BootError::Cycle { chain: ["A","B"] }`, not Running | High |
| FOUND-NEG-002 | US-M0-01 | Invalid config diagnostic | `config/database.toml` invalid on line 7 | `boot()` | `ConfigError::Parse { file: "config/database.toml", line: 7, hint }` | High |
| FOUND-NEG-003 | US-M0-02 | Missing optional binding resolves gracefully | no binding for Mailer | `Make::<Option<Mailer>>` | `None` not auto-construct (Laravel 13 #20) | Medium |
| FOUND-NEG-004 | US-M0-02 | Missing required binding is NotFound | no PaymentGateway | `Make::<PaymentGateway>` | `NotFound{ type_name: "PaymentGateway" }` | High |
| FOUND-NEG-005 | US-M0-01 | Duplicate provider warning (edge) | two providers same name "X" | `register` twice | last wins with warning log | Low |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| FOUND-MNK-001 | Concurrency | Concurrent `Make::<Counter>` Singleton from 10 tasks | `Singleton::<Counter>` registered | all task `Arc::ptr_eq` after converge |
| FOUND-MNK-002 | Shutdown liveness | `SIGTERM` twice (reentrancy) | request draining | drain not invoked twice, second signal no double-free |
| FOUND-MNK-003 | Reboot after Failed | `boot` failed Cycle then fixed providers | `Cycle` error then `configure().providers(fixed).boot()` | second `boot()` reaches `Running` |
| FOUND-MNK-004 | Bootstrap probe | `cargo check -p rustasea-foundation` | workspace only | no `sqlx`/`async-openai` in `cargo tree --depth 1` (NFR-Sca-02) |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| FOUND-SEC-001 | Operator | Config secret leak on invalid TOML | `config/database.toml` invalid line 7 with leaked secret | diagnostic does not echo raw `DATABASE_URL` secret value |
| FOUND-SEC-002 | Developer | Container missing secrets vs permissive | `Make::<Option<SecretStore>>` when unbound | `None`, not `SecretStore` with zeroed secret |


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
