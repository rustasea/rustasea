//! `make:agent` template — app/ai/agents/<snake>.rs (M6 adjacency).
//!
//! The concrete `AiProvider` trait lands in `rustavel-ai` (Sprint 07); this
//! scaffold compiles today behind the app's `ai` feature gate.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the AI agent file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/ai/agents/{}.rs", slug(&opts.name));
    let source = format!(
        r#"//! AI agent scaffold — {name} (M6 adjacency).
//!
//! The concrete `AiProvider` trait lands in `rustavel-ai` (Sprint 07); this
//! scaffold compiles today behind the app's `ai` feature gate.

/// Agent instruction set for {kind}.
#[derive(Debug, Clone)]
pub struct {name} {{
    /// System prompt driving the agent.
    pub system_prompt: String,
}}

impl {name} {{
    /// Create the agent with a default system prompt.
    pub fn new() -> Self {{
        Self {{
            system_prompt: "You are {name}, a helpful assistant.".to_string(),
        }}
    }}
}}

impl Default for {name} {{
    /// Create the agent with defaults.
    fn default() -> Self {{
        Self::new()
    }}
}}
"#,
        kind = slug(&opts.name),
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}
