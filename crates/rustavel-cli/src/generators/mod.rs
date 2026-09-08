//! Generator module wiring — one template per `make:*` kind.
//!
//! Each kind knows its output path and emits rustfmt-clean source that only
//! references types already present in the M0–M5 crates. Grouping mirrors the
//! FSD FS-M5-02 split: CRUD (controller/model), async (job/event/listener/
//! observer/command/seeder/test), AI (agent/tool — M6 adjacency behind the
//! `ai` feature flag of the consuming app).

use std::path::Path;

use crate::error::{CliError, CliResult};
use crate::generator::GeneratorError;
use crate::generator::{Generated, Generator};

pub mod kinds;

/// Which `make:*` kind to scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `make:controller` — HTTP controller.
    Controller,
    /// `make:model` — ORM model (+ optional migration via flag).
    Model,
    /// `make:provider` — service provider.
    Provider,
    /// `make:command` — console command.
    Command,
    /// `make:job` — queue job.
    Job,
    /// `make:event` — domain event.
    Event,
    /// `make:listener` — event listener.
    Listener,
    /// `make:observer` — model lifecycle observer.
    Observer,
    /// `make:test` — feature test.
    Test,
    /// `make:seeder` — database seeder.
    Seeder,
    /// `make:agent` — AI agent scaffold (M6 adjacency).
    Agent,
    /// `make:tool` — AI tool scaffold (M6 adjacency).
    Tool,
}

impl Kind {
    /// Parse a `make:*` suffix into a kind.
    pub fn parse(suffix: &str) -> Option<Self> {
        Some(match suffix {
            "controller" => Kind::Controller,
            "model" => Kind::Model,
            "provider" => Kind::Provider,
            "command" => Kind::Command,
            "job" => Kind::Job,
            "event" => Kind::Event,
            "listener" => Kind::Listener,
            "observer" => Kind::Observer,
            "test" => Kind::Test,
            "seeder" => Kind::Seeder,
            "agent" => Kind::Agent,
            "tool" => Kind::Tool,
            _ => return None,
        })
    }

    /// The `make:*` command name for this kind.
    pub fn command(self) -> &'static str {
        match self {
            Kind::Controller => "make:controller",
            Kind::Model => "make:model",
            Kind::Provider => "make:provider",
            Kind::Command => "make:command",
            Kind::Job => "make:job",
            Kind::Event => "make:event",
            Kind::Listener => "make:listener",
            Kind::Observer => "make:observer",
            Kind::Test => "make:test",
            Kind::Seeder => "make:seeder",
            Kind::Agent => "make:agent",
            Kind::Tool => "make:tool",
        }
    }
}

/// Options shared by every generator invocation.
#[derive(Debug, Clone)]
pub struct MakeOptions {
    /// Scaffold name (PascalCase), e.g. `UserController`.
    pub name: String,
    /// Overwrite existing files.
    pub force: bool,
    /// `make:controller` — emit the full REST resource method set.
    pub resource: bool,
    /// `make:model` — also scaffold `database/migrations/*_create_<table>_table.rs`.
    pub with_migration: bool,
}

/// Run a generator for `kind`, writing all files under `root`.
///
/// Returns every file written, in order. When `--resource`/`-m` flags are
/// set the corresponding extra files are appended.
pub fn generate(kind: Kind, root: &Path, opts: &MakeOptions) -> CliResult<Vec<Generated>> {
    let kind_label = match kind {
        Kind::Controller => "controller",
        Kind::Model => "model",
        Kind::Provider => "provider",
        Kind::Command => "command",
        Kind::Job => "job",
        Kind::Event => "event",
        Kind::Listener => "listener",
        Kind::Observer => "observer",
        Kind::Test => "test",
        Kind::Seeder => "seeder",
        Kind::Agent => "agent",
        Kind::Tool => "tool",
    };
    if let Err(err) = Generator::validate_name(kind_label, &opts.name) {
        return Err(err.into());
    }

    let mut written = Vec::new();
    match kind {
        Kind::Controller => {
            written.push(kinds::controller::scaffold(root, opts)?);
        }
        Kind::Model => {
            written.push(kinds::model::scaffold(root, opts)?);
            if opts.with_migration {
                written.push(kinds::model::scaffold_migration(root, opts)?);
            }
        }
        Kind::Provider => {
            written.push(kinds::provider::scaffold(root, opts)?);
        }
        Kind::Command => {
            written.push(kinds::command::scaffold(root, opts)?);
        }
        Kind::Job => {
            written.push(kinds::job::scaffold(root, opts)?);
        }
        Kind::Event => {
            written.push(kinds::event::scaffold(root, opts)?);
        }
        Kind::Listener => {
            written.push(kinds::listener::scaffold(root, opts)?);
        }
        Kind::Observer => {
            written.push(kinds::observer::scaffold(root, opts)?);
        }
        Kind::Test => {
            written.push(kinds::test::scaffold(root, opts)?);
        }
        Kind::Seeder => {
            written.push(kinds::seeder::scaffold(root, opts)?);
        }
        Kind::Agent => {
            written.push(kinds::agent::scaffold(root, opts)?);
        }
        Kind::Tool => {
            written.push(kinds::tool::scaffold(root, opts)?);
        }
    }
    Ok(written)
}

/// Default help text per kind (used by `list`).
pub fn kind_help(kind: Kind) -> &'static str {
    match kind {
        Kind::Controller => "Make a new controller class",
        Kind::Model => "Make a new ORM model",
        Kind::Provider => "Make a new service provider",
        Kind::Command => "Make a new console command",
        Kind::Job => "Make a new queue job",
        Kind::Event => "Make a new domain event",
        Kind::Listener => "Make a new event listener",
        Kind::Observer => "Make a new model observer",
        Kind::Test => "Make a new feature test",
        Kind::Seeder => "Make a new database seeder",
        Kind::Agent => "Make a new AI agent (M6)",
        Kind::Tool => "Make a new AI tool (M6)",
    }
}

/// Default usage signature per kind (used by `list`).
pub fn kind_usage(kind: Kind) -> &'static str {
    match kind {
        Kind::Controller => "make:controller {name} [--resource]",
        Kind::Model => "make:model {name} [-m] [--force]",
        Kind::Provider
        | Kind::Command
        | Kind::Job
        | Kind::Event
        | Kind::Listener
        | Kind::Observer
        | Kind::Test
        | Kind::Seeder
        | Kind::Agent
        | Kind::Tool => "make:* {name} [--force]",
    }
}

/// Map a generator error onto the CLI error surface.
pub fn into_cli(kind: &str, name: &str, err: GeneratorError) -> CliError {
    match err {
        GeneratorError::AlreadyExists { path } => CliError::AlreadyExists { path },
        other => CliError::GenerationFailed {
            kind: kind.to_string(),
            name: name.to_string(),
            detail: other.to_string(),
        },
    }
}
