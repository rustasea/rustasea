//! `make:job` template — app/jobs/<snake>.rs with `#[tries]`/`#[backoff]`.
//!
//! Retry/timing policy is declared with the M5 macro attributes; the queue
//! driver executes the same semantics (FR-506).

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the queue job file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/jobs/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! Queue job scaffold — {name}.
//!
//! `#[tries(3)]`/`#[backoff(5)]` declare the retry budget declaratively;
//! dispatch via `{name} {{ .. }}.dispatch().await`.

use rustavel::macros::{{backoff, tries}};
use rustavel::queue::{{async_trait, error::JobError, Job}};

/// Job payload (serialize any domain fields you need).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[tries(3)]
#[backoff(5)]
pub struct {name} {{
    /// Human-readable label for the attempt.
    pub label: String,
}}

#[async_trait]
impl Job for {name} {{
    /// Execute the job body once.
    async fn handle(self) -> std::result::Result<(), JobError> {{
        let _ = self.label;
        Ok(())
    }}
}}
"#,
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
