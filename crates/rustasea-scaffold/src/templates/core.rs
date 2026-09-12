//! Core application files shared by every variant.
//!
//! Emits the package entry points (`lib.rs`, `main.rs`), environment files, the
//! `askama.toml` template root for server-rendered variants, and the
//! `bootstrap/` kernel wiring that was an empty placeholder before the
//! starter kit existed (`bootstrap/providers.rs`, `bootstrap/commands.rs`).

use crate::variant::StarterKitVariant;

use super::TemplateFile;

/// Core templates for `variant`.
pub fn entries(variant: StarterKitVariant) -> Vec<TemplateFile> {
    let mut files = vec![
        (".env.example", ENV_EXAMPLE),
        (".gitignore", GITIGNORE),
        ("README.md", README),
        ("lib.rs", LIB_RS),
        ("main.rs", MAIN_RS),
        ("bootstrap/mod.rs", BOOTSTRAP_MOD),
        ("bootstrap/app.rs", BOOTSTRAP_APP),
        ("bootstrap/providers.rs", BOOTSTRAP_PROVIDERS),
        ("bootstrap/commands.rs", BOOTSTRAP_COMMANDS),
        ("storage/app/.gitkeep", ""),
        ("storage/logs/.gitkeep", ""),
    ];
    if variant.uses_askama() {
        files.push(("askama.toml", ASKAMA_TOML));
    }
    files
}

const ENV_EXAMPLE: &str = r##"APP_NAME=@@app_name@@
APP_ENV=local
APP_URL=http://localhost:3000
APP_KEY=

# Database (sqlite by default; override for Postgres/MySQL).
DB_CONNECTION=sqlite
DB_DATABASE=database/database.sqlite

# Session (browser starter kits authenticate with session cookies + CSRF).
SESSION_DRIVER=cookie
SESSION_LIFETIME=120
SESSION_SECURE=false
"##;

const GITIGNORE: &str = r##"/target
/.env
/database/*.sqlite
/storage/logs/*
!/storage/logs/.gitkeep
/storage/app/*
!/storage/app/.gitkeep
"##;

const README: &str = r##"# @@app_pascal@@

A RustaSea starter kit generated with:

```sh
cargo rustasea new @@app_name@@ --variant @@variant@@
```

## Layout

- `app/` — domain actions, concerns, HTTP controllers/middleware/requests, models, providers
- `bootstrap/` — application kernel wiring (providers, commands)
- `routes/` — `web`, `auth`, `settings`, and `console` route tables
- `database/` — migrations, factories, seeders
- `resources/` — presentation layer for the `@@variant@@` variant
- `tests/` — `feature` and `unit` test suites

## Development

```sh
cargo run
```

The server binds `0.0.0.0:3000` by default (`APP_URL` overrides it).
"##;

const LIB_RS: &str = r##"//! @@app_pascal@@ — RustaSea application library.
//!
//! The module tree mirrors the Laravel layout: `app/` holds domain actions,
//! concerns, HTTP, models, and providers; `bootstrap/` wires the kernel;
//! `routes/` owns the route tables; `database/` holds migrations, factories,
//! and seeders.

pub mod app;
pub mod bootstrap;
pub mod database;
pub mod routes;
"##;

const MAIN_RS: &str = r##"//! @@app_pascal@@ HTTP entry point.

use std::sync::Arc;

use @@app_snake@@::{bootstrap, routes};
use rustasea::http::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the container and run the provider boot DAG. `configure`
    // returns `Result` so a misconfigured boot aborts before the server starts
    // instead of silently ignoring the failure.
    let app = bootstrap::app::configure()?;

    // Create the shared HTTP state once; every route table receives this same
    // instance so handlers resolve one application state, not a disconnected one.
    let state = Arc::new(AppState::new("local", true));

    // Build the axum router from the generated route tables.
    let router = routes::router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("@@app_pascal@@ listening on http://0.0.0.0:3000");

    axum::serve(listener, router)
        .with_graceful_shutdown(app.shutdown())
        .await?;
    Ok(())
}
"##;

const BOOTSTRAP_MOD: &str = r##"//! Application bootstrap — providers, commands, and kernel configuration.

pub mod app;
pub mod commands;
pub mod providers;
"##;

const BOOTSTRAP_APP: &str = r##"//! Application bootstrap — `Application::configure` for @@app_pascal@@.
//!
//! Registers the generated service providers and runs the register → boot DAG
//! before the HTTP kernel starts serving.

use rustasea::foundation::BootError;
use rustasea::Application;

use crate::bootstrap::providers;

/// Build and boot the application container.
///
/// Returns [`BootError`] when the provider graph contains a cycle or an
/// unresolved dependency, so a misconfigured boot never starts the server.
pub fn configure() -> Result<Application, BootError> {
    let mut app = Application::configure(|_| {});
    for provider in providers::providers() {
        app.provider(provider);
    }
    app.boot()?;
    Ok(app)
}
"##;

const BOOTSTRAP_PROVIDERS: &str = r##"//! Provider registry — service providers registered by the application.
//!
//! This registry is populated by the starter kit (previously empty) and is the
//! registration site for providers generated with `cargo rustasea make:provider`.

use rustasea::ServiceProvider;

use crate::app::providers::{AppServiceProvider, AuthServiceProvider};

/// Providers wired into the boot DAG, in registration order.
///
/// Order is the tie-breaker for providers without `dependencies()`; the
/// foundation `Application::boot` topologically sorts them regardless.
pub fn providers() -> Vec<Box<dyn ServiceProvider>> {
    vec![Box::new(AppServiceProvider), Box::new(AuthServiceProvider)]
}
"##;

const BOOTSTRAP_COMMANDS: &str = r##"//! CLI command registry — `cargo artisan` console commands.
//!
//! This registry gives the previously-empty command site a real home; commands
//! generated into `app/console/commands/*` are appended here.

/// Names of the console commands registered for the application.
pub fn commands() -> Vec<&'static str> {
    vec![]
}
"##;

const ASKAMA_TOML: &str = r##"# askama template root — compiled into the binary at build time.
[general]
dirs = ["resources/views"]
"##;
