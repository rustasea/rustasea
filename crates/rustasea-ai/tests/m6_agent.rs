//! Integration test: agent + middleware + deferred loader end-to-end (M6).
//!
//! Verifies the FS-M6-07 contract surface with the crate's default features:
//! an agent invoking a tool produces ordered token chunks, and deferred
//! loaders degrade gracefully when their backing feature is disabled.

use rustasea_ai::agent::Tool;
use rustasea_ai::loaders::SimilaritySearch;
use rustasea_ai::types::AiChunk;
use rustasea_ai::{Agent, AgentRun, AiError, Result};
use serde_json::{json, Value};

/// Tool that uppercases its `text` argument.
struct UpperTool;

#[async_trait::async_trait]
impl Tool for UpperTool {
    fn name(&self) -> &'static str {
        "upper"
    }

    fn description(&self) -> &'static str {
        "Uppercases the text argument"
    }

    async fn call(&self, arguments: Value) -> Result<Value> {
        let text = arguments["text"].as_str().unwrap_or_default();
        Ok(json!({ "text": text.to_uppercase() }))
    }
}

fn agent() -> Agent {
    Agent::new("echo").tool(UpperTool)
}

fn flatten(run: &AgentRun) -> Vec<String> {
    run.chunks
        .iter()
        .map(|c: &AiChunk| c.text.clone())
        .collect()
}

#[tokio::test]
async fn agent_invokes_tool_and_emits_ordered_chunks() {
    let run = agent().run("tool:upper({\"text\":\"hi\"})").await;
    let texts = flatten(&run);
    assert_eq!(run.chunks.len(), 2);
    assert_eq!(run.chunks[0].kind, "token");
    assert_eq!(run.chunks[1].kind, "tool_call");
    assert_eq!(run.tool_results[0].0, "upper");
    assert!(texts.iter().any(|t| t.contains("hi")));
}

#[cfg(not(feature = "search"))]
#[tokio::test]
async fn deferred_loader_degrades_without_search_feature() {
    let loader = SimilaritySearch::new("docs");
    assert!(!loader.ready());
    // Default features (no `search`) → capability-style denial.
    assert!(matches!(
        loader.handle(),
        Err(AiError::UnsupportedCapability { .. })
    ));
}

#[cfg(feature = "search")]
#[tokio::test]
async fn deferred_loader_requires_binding_with_search_feature() {
    let loader = SimilaritySearch::new("docs");
    // Feature on but no engine bound → configured-error (never panics).
    assert!(!loader.ready());
    assert!(matches!(loader.handle(), Err(AiError::NotConfigured(_))));
}
