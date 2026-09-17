# ExampleApp

A RustaSea starter kit generated with:

```sh
cargo rustasea new example-app --variant blade
```

## Prerequisites

The RustaSea framework crates are not yet published to crates.io. Until the
first release, `cargo build` will not resolve the `rustasea = "0.1"` and
`rustasea-view = "0.1"` dependencies declared in `Cargo.toml`. For local
development, point those dependencies at the framework repository with a
`[patch.crates-io]` block:

```toml
[patch.crates-io]
rustasea = { git = "https://github.com/rustasea/framework.git" }
rustasea-view = { git = "https://github.com/rustasea/framework.git" }
```

## Layout

- `app/`: domain actions, concerns, HTTP controllers/middleware/requests, models, providers
- `bootstrap/`: application kernel wiring (providers, commands); `bootstrap/cache/` is runtime config cache
- `config/`: typed TOML configuration, auto-discovered by the loader (`config/*.toml`)
- `routes/`: `web`, `auth`, `settings`, and `console` route tables
- `database/`: migrations, factories, seeders
- `public/`: web root (`robots.txt`, `favicon.ico`); build output is ignored
- `resources/`: presentation layer for the `blade` variant
- `storage/`: runtime state (`app/private`, `framework/{cache/data,sessions,testing,views}`)
- `tests/`: `feature` and `unit` test suites

## Development

```sh
cargo run
```

The server binds `0.0.0.0:3000` by default (`APP_URL` overrides it).

## Docker

A container stack with laravel/sail parity is generated alongside the app:

```bash
docker-compose up -d                                          # app + infra
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up  # hot reload
docker-compose down                                           # tear down
```

| Service | Ports | Purpose |
|---|---|---|
| `app` | `3000` | The ExampleApp HTTP app |
| `postgres` | `5432` | Primary SQL store + pgvector |
| `redis` | `6379` | Cache + queue backend |
| `minio` | `9000`, `9001` | S3-compatible storage (`9001` = console) |
| `mailpit` | `1025`, `8025` | SMTP capture (`1025`) + web UI (`8025`) |

App: <http://localhost:3000> · Mailpit UI: <http://localhost:8025> ·
MinIO console: <http://localhost:9001>.
