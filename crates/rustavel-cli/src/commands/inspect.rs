//! Framework inspection commands — route:list and show:model.
//!
//! `route:list` emits the observable route table (FS-M1-03) and `show:model`
//! the model inspector surface (FR-109). Both honour `--json`.

use async_trait::async_trait;

use crate::artisan::{Command, Io};
use crate::error::{CliError, CliResult};
use crate::generator::Generator;
use crate::output;

/// `route:list` — enumerate registered HTTP routes.
pub struct RouteList;

#[async_trait]
impl Command for RouteList {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "route:list"
    }

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("route:list [--json]")
    }

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("List all registered routes")
    }

    /// Execute: print an empty route table until the router is wired.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let json = args.iter().any(|a| a == "--json");
        if json {
            io.line("[]");
            return Ok(());
        }
        io.line(
            output::table(vec![vec!["Method".into(), "Path".into(), "Name".into()]]).trim_end(),
        );
        Ok(())
    }
}

/// `show:model` — inspect a model's attributes/relations/casts (FR-109).
pub struct ShowModel;

#[async_trait]
impl Command for ShowModel {
    /// Command signature.
    fn signature(&self) -> &'static str {
        "show:model"
    }

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {
        Some("show:model {name}")
    }

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {
        Some("Show a model's attributes and relations")
    }

    /// Execute: print the model path + placeholder inspector output.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
        let name = args
            .first()
            .filter(|n| !n.starts_with('-'))
            .cloned()
            .unwrap_or_default();
        if name.is_empty() {
            return Err(CliError::InvalidArguments {
                command: "show:model".into(),
                detail: "expected a model name, e.g. show:model Post".into(),
            });
        }
        let snake = Generator::snake(&name);
        io.line(format!("model: {name}"));
        io.line(format!("file:  app/models/{snake}.rs"));
        io.line("table: <derived from Model::table_name()>");
        Ok(())
    }
}
