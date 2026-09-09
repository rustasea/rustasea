//! `make:observer` template — app/observers/<snake>.rs.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the model observer file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/observers/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! Model observer scaffold — {name}.
//!
//! Hooks fire on model lifecycle events (created/updated/deleted). Register
//! the observer with its model in `bootstrap/providers.rs`.

/// Observes model lifecycle events for its target model.
pub struct {name};

impl {name} {{
    /// After a row was created.
    pub fn created(_model: &str) {{}}

    /// After a row was updated.
    pub fn updated(_model: &str) {{}}

    /// After a row was deleted.
    pub fn deleted(_model: &str) {{}}
}}
"#,
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
