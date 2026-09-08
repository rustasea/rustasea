//! `make:event` template — app/events/<snake>.rs.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the domain event file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/events/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! Domain event scaffold — {name}.

use rustavel::events::Event;

/// Fired when the {kind} occurs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {name} {{
    /// Affected entity id (if any).
    pub entity_id: Option<uuid::Uuid>,
}}

impl Event for {name} {{
    /// Stable event name.
    fn event_name(&self) -> &'static str {{
        "{snake}"
    }}
}}
"#,
        snake = slug(&opts.name),
        name = opts.name,
        kind = slug(&opts.name),
    );
    write_scaffold(root, rel, source, opts.force)
}
