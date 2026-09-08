//! `make:test` template — tests/feature/<snake>.rs (TestCase harness).

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the feature test file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("tests/feature/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! Feature test scaffold — {name}.
//!
//! Extend `TestCase` and override `setup` to provision isolated stores via
//! the M5 harness; factory sequences reset between tests.

use rustavel::testing::TestCase;

/// Feature test exercising the {kind} flow.
pub struct {name};

impl Default for {name} {{
    /// Build a test with default configuration.
    fn default() -> Self {{
        Self
    }}
}}

impl TestCase for {name} {{
    /// Provision isolated test stores.
    fn setup(&mut self) {{}}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    /// Smoke: setup completes and the harness resets factories.
    #[test]
    fn smoke() {{
        let mut test = {name}::default();
        test.setup();
        rustavel::testing::reset_factory_sequences();
    }}
}}
"#,
        kind = slug(&opts.name),
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
