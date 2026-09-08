//! Generator scaffolding for `make:agent` / `make:tool`.
//!
//! The generators (S06 `rustavel-cli`) produce boilerplate into
//! `app/ai/agents/` and `app/ai/tools/`. This module owns the shared
//! templates and name normalization used by both the CLI generators and the
//! proc-macro surface (`#[derive(Tool)]`).

use crate::error::{AiError, Result};

/// Generated source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldKind {
    /// `make:agent` output — an agent struct.
    Agent,
    /// `make:tool` output — a tool struct.
    Tool,
}

/// One generated scaffold file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldFile {
    /// Destination path relative to `app/ai/` (`agents/foo.rs`).
    pub path: String,
    /// Generated source text.
    pub source: String,
}

/// Render the scaffold source for `make:agent`/`make:tool`.
///
/// Name is normalized to PascalCase for the struct and snake_case for the
/// file/module path; invalid identifiers error.
pub fn generate_scaffold(kind: ScaffoldKind, name: &str) -> Result<ScaffoldFile> {
    let pascal = to_pascal(name)?;
    let snake = to_snake(name)?;
    let (dir, body) = match kind {
        ScaffoldKind::Agent => (
            "agents",
            format!(
                "use rustavel::ai::Agent;\n\n\
                 /// {{name}} agent scaffold — register tools, run prompts.\n\
                 pub struct {pascal};\n\n\
                 impl {pascal} {{\n    /// Build the agent over a provider.\n    \
                 pub fn agent(provider: &'static str) -> Agent {{\n        \
                 Agent::new(provider)\n    }}\n}}\n"
            ),
        ),
        ScaffoldKind::Tool => (
            "tools",
            format!(
                "use rustavel::ai::Tool;\n\n\
                 /// {{name}} tool scaffold — implement run to execute.\n\
                 pub struct {pascal};\n\n\
                 #[rustavel::macros::async_trait]\n\
                 impl Tool for {pascal} {{\n    fn name(&self) -> &'static str {{ \"{snake}\" }}\n    \
                 fn description(&self) -> &'static str {{ \"{{name}} tool\" }}\n}}\n"
            ),
        ),
    };
    let module = format!("{dir}/{snake}");
    let source = body.replace("{{name}}", &pascal);
    Ok(ScaffoldFile {
        path: format!("{module}.rs"),
        source,
    })
}

/// Normalize an identifier to PascalCase.
fn to_pascal(name: &str) -> Result<String> {
    let parts = split_ident(name)?;
    let mut out = String::new();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_uppercase().next().unwrap_or(first));
            out.push_str(&chars.as_str().to_lowercase());
        }
    }
    if out.is_empty() {
        return Err(AiError::NotConfigured(format!(
            "scaffold name '{name}' produced an empty identifier"
        )));
    }
    Ok(out)
}

/// Normalize an identifier to snake_case.
fn to_snake(name: &str) -> Result<String> {
    let parts = split_ident(name)?;
    Ok(parts.join("_"))
}

/// Split an identifier on `-`/`_`/case boundaries.
fn split_ident(name: &str) -> Result<Vec<String>> {
    if name.is_empty() {
        return Err(AiError::NotConfigured(
            "scaffold name must not be empty".to_string(),
        ));
    }
    let raw: Vec<String> = name
        .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    // Split camelCase into parts.
    let mut parts = Vec::new();
    for word in raw {
        let mut current = String::new();
        for c in word.chars() {
            if c.is_uppercase() && !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            current.push(c);
        }
        if !current.is_empty() {
            parts.push(current);
        }
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_scaffold_normalizes_name() {
        let file = generate_scaffold(ScaffoldKind::Agent, "search-assistant").unwrap();
        assert_eq!(file.path, "agents/search_assistant.rs");
        assert!(file.source.contains("pub struct SearchAssistant"));
    }

    #[test]
    fn tool_scaffold_keeps_snake_name() {
        let file = generate_scaffold(ScaffoldKind::Tool, "SearchDocs").unwrap();
        assert_eq!(file.path, "tools/search_docs.rs");
        assert!(file
            .source
            .contains("fn name(&self) -> &'static str { \"search_docs\" }"));
    }
}
