//! `make:tool` template — app/ai/tools/<snake>.rs (M6 adjacency).
//!
//! Implements the tool contract that `rustavel-ai` (Sprint 07) exposes; this
//! scaffold defines the schema + call surface early.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the AI tool file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/ai/tools/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! AI tool scaffold — {name} (M6 adjacency).
//!
//! Implements the tool contract that `rustavel-ai` (Sprint 07) exposes; this
//! scaffold defines the schema + call surface early.

use serde_json::Value;

/// JSON schema for this tool's arguments.
pub fn input_schema() -> Value {{
    serde_json::json!({{
        "type": "object",
        "properties": {{
            "query": {{ "type": "string", "description": "Search query" }}
        }},
        "required": ["query"]
    }})
}}

/// Executes the tool with validated arguments.
pub async fn run(arguments: Value) -> Value {{
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default();
    serde_json::json!({{
        "tool": "{kind}",
        "query": query,
        "result": null
    }})
}}
"#,
        kind = slug(&opts.name),
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
