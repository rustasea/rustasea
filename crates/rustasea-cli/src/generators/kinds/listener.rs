//! `make:listener` template — app/listeners/<snake>.rs.
//!
//! Listens for a domain event; `rustasea::events::Listener` declares an
//! async `handle(event)` that implementors honour via `#[async_trait]`.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the event listener file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/listeners/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! Event listener scaffold — {name}.
//!
//! Listens for `rustasea::events::JobAttempted` by default; swap the event
//! type and register in `bootstrap/providers.rs`.

use rustasea::events::{{async_trait, EventError, JobAttempted, Listener}};

/// Handles the event it listens for.
pub struct {name};

#[async_trait]
impl Listener<JobAttempted> for {name} {{
    /// React to the event.
    async fn handle(&self, event: JobAttempted) -> std::result::Result<(), EventError> {{
        let _ = event;
        Ok(())
    }}
}}
"#,
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
