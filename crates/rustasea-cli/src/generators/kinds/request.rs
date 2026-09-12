//! `make:request` template — app/http/requests/<snake>.rs.
//!
//! Emits a validated form-request payload mirroring
//! `crates/rustasea-validation/src/form_request.rs`: handlers extract
//! `FormRequest<{Name}>`, the JSON body is deserialized into the struct, and
//! the `Validatable` impl runs before the handler executes. Failures become the
//! standard `422` `ErrorBag` response.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the form-request file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/http/requests/{}.rs", slug(&opts.name));
    write_scaffold(root, rel, source(&opts.name), opts.force)
}

/// Pre-formatted form-request source for `name`.
fn source(name: &str) -> String {
    format!(
        r#"//! Form request scaffold — {name}.
//!
//! Extract `{name}Extractor` (or `FormRequest<{name}>`) in a handler signature:
//! the JSON body is deserialized into `{name}`, validated by the `Validatable`
//! impl below, and only then handed to the handler. Invalid payloads
//! short-circuit with a `422` `ErrorBag`.

use rustasea::validation::{{ErrorBag, FormRequest, Rules, Validatable}};
use serde::{{Deserialize, Serialize}};

/// Validated input for {kind}.
#[derive(Debug, Deserialize, Serialize)]
pub struct {name} {{
    /// Example field — replace with the request's real fields.
    pub name: String,
}}

impl Validatable for {name} {{
    /// Validate the payload against the declared rules.
    fn validate(&self) -> Result<(), ErrorBag> {{
        Rules::new()
            .field("name", "required|min:3")
            .validate(&rustasea::validation::to_value(self).expect("payload serializes"))
    }}
}}

/// Handler-facing extractor alias for this request.
pub type {name}Extractor = FormRequest<{name}>;
"#,
        kind = slug(name),
        name = name,
    )
}

#[cfg(test)]
mod tests {
    use super::source;

    /// Generated requests must serialize through the validation facade, never
    /// a bare `serde_json::` the scaffolded app cannot resolve.
    #[test]
    fn request_template_uses_resolvable_validation_path() {
        let src = source("StorePostRequest");
        assert!(
            src.contains("rustasea::validation::to_value(self)"),
            "request must serialize through rustasea::validation: {src}"
        );
        assert!(
            !src.contains("serde_json"),
            "request must not reference serde_json directly: {src}"
        );
        assert!(
            src.contains("use rustasea::validation::{ErrorBag, FormRequest, Rules, Validatable};"),
            "request must import the validation surface: {src}"
        );
    }
}
