//! `make:command` template — app/console/commands/<snake>.rs.
//!
//! The scaffold implements the `rustavel::cli::Command` trait. A generated
//! app crate depends on `rustavel` only (README M5 layout), so the
//! `#[async_trait]` expansion is imported from the umbrella crate's re-export
//! (`rustavel::cli::async_trait`) instead of the raw `async_trait` crate.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the console command file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/console/commands/{}.rs", slug(&opts.name));
    let signature = slug(&opts.name).replace('_', ":");
    let source = format!(
        r#"//! Console command scaffold — {name}.

use rustavel::cli::async_trait::async_trait;
use rustavel::cli::error::CliResult;
use rustavel::cli::{{Command, Io}};

/// Signature shown in `cargo artisan list`.
pub const SIGNATURE: &str = "{signature}";

/// Runs the `{signature}` command.
pub struct {name};

#[async_trait]
impl Command for {name} {{
    /// Command signature.
    fn signature(&self) -> &'static str {{
        SIGNATURE
    }}

    /// Usage line rendered by `list`.
    fn usage(&self) -> Option<&'static str> {{
        Some("{signature} [--force]")
    }}

    /// One-line help rendered by `list`.
    fn help(&self) -> Option<&'static str> {{
        Some("Run the {name} command")
    }}

    /// Execute the command.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {{
        let _ = args;
        io.line(format!("Running {signature}"));
        Ok(())
    }}
}}
"#,
        signature = signature,
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
