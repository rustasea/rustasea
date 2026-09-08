//! `make:provider` template — app/providers/<snake>.rs.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the provider file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/providers/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! Service provider scaffold — {name}.

use rustavel::Application;
use rustavel::ServiceProvider;

/// Registers and boots {kind} bindings.
pub struct {name};

impl ServiceProvider for {name} {{
    /// Register bindings into the application container.
    fn register(&self, _app: &mut Application) {{}}

    /// Boot after every provider registered.
    fn boot(&self, _app: &Application) {{}}
}}
"#,
        kind = slug(&opts.name),
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
