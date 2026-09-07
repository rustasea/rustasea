# API: DeveloperPlatform — CLI (`cargo rustavel list` + `make:*` + `Artisan::call`)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §4` · `requirements/prd FR-500..509` · `requirements/fsd FS-M5-01..04` · `requirements/tdd BC-5` · `design/component-inventory.md`
> **Crates:** `rustavel-cli` · `rustavel-macros` · `rustavel-testing` · `xtask`
> **Module:** [modules/developer-platform/overview.md](../../modules/developer-platform/overview.md)

## 1. Standar Global

- **CLI:** `cargo rustavel <command> [args] [--json]` via `xtask` (`cargo-xtask` bin) + `clap` derive + typed `Args`/`Flags`.
- **Listing:** `list [--json] [--all]` emits `{ name, usage, help, hidden }` per command; `#[usage("app:send {user}")]` / `#[help("...")]` / `#[hidden]` control visibility.
- **Format Tanggal:** CLI `migrate:status` shows executed_at RFC3339.

## 2. Endpoints (CLI)

### 2.1 `cargo rustavel list [--json] [--all]`

- **Deskripsi:** Enumerates registered commands with `usage`/`help`/`hidden`. `hidden` omitted without `--all`.
- **Kontrol Akses:** None.

#### Params

| Name | Type | Req | Default | Desc | Example |
|------|------|-----|---------|------|---------|
| `--json` | flag | no | false | machine-readable output | `--json` |
| `--all` | flag | no | false | include hidden commands | `--all` |

#### Response

**Sukses (human table — default):**

```
Available commands:
  make:controller  Make a new controller class
  migrate          Run pending migrations
```

**Sukses (--json):**

```json
[
  { "name": "make:controller", "usage": "make:controller {name} [--resource]", "help": "Make a new controller class", "hidden": false },
  { "name": "migrate", "usage": "migrate [--fresh] [--seed]", "help": "Run pending migrations", "hidden": false },
  { "name": "internal:debug", "usage": "internal:debug", "help": "Hidden", "hidden": true }
]
```

**Sukses (--json without `--all` — hidden omitted):**

```json
[
  { "name": "make:controller", "usage": "make:controller {name} [--resource]", "help": "Make a new controller class", "hidden": false }
]
```

#### Usage

```bash
cargo rustavel list --json | jq .
cargo rustavel list --json --all | jq '.[] | select(.hidden==true)'
```

### 2.2 `cargo rustavel make:*` generators

| Generator | Invocation | Output path | Notes |
|-----------|------------|-------------|-------|
| `make:controller` | `make:controller UserController [--resource]` | `app/http/controllers/user_controller.rs` | `--resource` adds 7 methods (index/store/show/update/destroy adjacency) |
| `make:model` | `make:model Post -m [--force]` | `app/models/post.rs` | `#[derive(Model)]` + `Factory`; `-m` adds `database/migrations/*_create_posts_table.rs` |
| `make:provider` | `make:provider AppProvider` | `app/providers/app_provider.rs` | `ServiceProvider` register/boot |
| `make:command` | `make:command SendEmails` | `app/console/commands/send_emails.rs` | `Command` with `#[usage]` |
| `make:job` | `make:job ProcessPodcast` | `app/jobs/process_podcast.rs` | `Job` + `#[tries]` scaffold |
| `make:event` | `make:event UserCreated` | `app/events/user_created.rs` | `Event` struct |
| `make:listener` | `make:listener SendWelcomeEmail` | `app/listeners/send_welcome_email.rs` | `Listener` `QUEUE=true` flag optionally |
| `make:observer` | `make:observer UserObserver` | `app/observers/user_observer.rs` | `Observer` on model lifecycle |
| `make:test` | `make:test UserTest` | `tests/feature/user_test.rs` | `TestCase` harness |
| `make:seeder` | `make:seeder UserSeeder` | `database/seeders/user_seeder.rs` | `Seeder::run` |
| `make:agent` | `make:agent SupportAgent` (M6) | `app/ai/agents/support_agent.rs` | `Agent` scaffold (see intelligence) |
| `make:tool` | `make:tool SearchDocs` (M6) | `app/ai/tools/search_docs.rs` | `Tool` impl |

All generated `.rs` files must pass `rustfmt --check` + `clippy -- -D warnings` (C-04); `cargo check` after `make:*` `<10s` incremental (NFR-Per-04). `AlreadyExists` without `--force` → non-zero.

#### Response (AlreadyExists)

```json
{ "errors": [{ "status": "409", "code": "GeneratorError::AlreadyExists", "title": "Already exists", "detail": "app/models/post.rs already exists. Pass --force to overwrite.", "source": { "pointer": "/path", "value": "app/models/post.rs" } }] }
```

#### Usage

```bash
cargo rustavel make:controller UserController --resource
cargo rustavel make:model Post -m
cargo rustavel make:job ProcessPodcast
cargo rustavel make:event UserCreated
cargo rustavel make:listener SendWelcomeEmail
cargo rustavel make:agent SupportAgent   # M6 adjacency
cargo rustavel make:tool SearchDocs
rustfmt --check app/http/controllers/user_controller.rs && cargo clippy -- -D warnings
```

### 2.3 Prompts `ask` / `secret` / `confirm` / `choice` / `multiSelect` + output helpers

Demonstrated inside `app/console/commands` `handle`:

```rust
let ok: bool = confirm("Proceed?")?; // n -> abort non-zero
let name = ask("Controller name?")?;
let pw = secret("Password?")?;
let choice = choice("Kind?", ["controller","model"])?;
let picked = multi_select("Events?", ["Created","Updated"])?;
table(vec![vec!["Name","Created At"], vec!["Ada","2026-09-07"]]);
progress_bar(20, 500);
```

### 2.4 Programmatic — `Artisan::call(command, args)` (in-process)

```rust
let out: CommandOutput = Artisan::call("migrate", vec![]).await?; // no subprocess
let out = Artisan::call("migrate", vec!["--fresh".into(), "--seed".into()]).await?;
```

- Inspects via `cargo test` (see testing).

### 2.5 `did you mean?` (strsim)

```bash
cargo rustavel make:controll
# error: unknown command 'make:controll'. Did you mean make:controller ?
```

## 3. OpenAPI 3.0 Snippet (CLI listing as HTTP when app exposes `/cli/list` helper; otherwise x-inferred)

```yaml
openapi: 3.0.3
info:
  title: Rustavel DeveloperPlatform — CLI listing + generators
  version: 0.1.0
  description: cargo rustavel list --json contract + make:* outputs; inferred from api-contracts.md §4
x-inferred: true
servers:
  - url: http://localhost:3000
paths:
  /cli/list:
    get:
      summary: List commands (help helper when app exposes it)
      parameters:
        - name: json
          in: query
          schema: { type: boolean, example: true }
        - name: all
          in: query
          schema: { type: boolean, example: false }
      responses:
        '200':
          description: Command list
          content:
            application/json:
              example:
                - name: make:controller
                  usage: "make:controller {name} [--resource]"
                  help: Make a new controller class
                  hidden: false
  /cli/generate:
    post:
      summary: Generate code (app-level helper mapping to make:*)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [kind, name]
              properties:
                kind: { type: string, enum: [controller, model, job, event, listener], example: controller }
                name: { type: string, example: UserController }
                force: { type: boolean, example: false }
      responses:
        '201':
          description: Generated
          content:
            application/json:
              example: { path: "app/http/controllers/user_controller.rs", lint: "rustfmt+clippy clean" }
        '409':
          description: Already exists
          content:
            application/json:
              example:
                errors:
                  - status: '409'
                    code: GeneratorError::AlreadyExists
                    title: Already exists
components:
  securitySchemes:
    none: { type: http, scheme: bearer, description: "CLI is not HTTP — shown as helper when exposed" }
security: []
```

## 4. Error Catalogue

| HTTP / Exit | Typed error | When |
|-------------|-------------|------|
| 201 | — | generated |
| 409 | `GeneratorError::AlreadyExists{path}` | without `--force` |
| non-zero | `did you mean?` | unknown command |

## 5. Cross-References

- Module: [cli.md](../../modules/developer-platform/cli.md) · [generators.md](../../modules/developer-platform/generators.md) · [testing-harness.md](../../modules/developer-platform/testing-harness.md)
- Testing: [testing/developer-platform/test-cli.md](../../testing/developer-platform/test-cli.md) · BDD `@cli`, `@generators`, `@testing` · `testing/stubs/m5-cli-testing.stub.rs`

## 6. A-Gate

- [x] 201/409 examples with realistic `path` + `lint`.
- [x] YAML valid with constraints, `enum` on `kind`, security.
- [x] curl `list --json | jq` valid.

## 7. Chaos Note

`cargo check` after `make:*` incremental `<10s` — timeout gate; `Shutdownable` on `queue:work` drain is in `async-workloads`.

