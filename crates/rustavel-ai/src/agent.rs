//! `Agent` and `Tool` contracts — the M6 agent runtime surface.

use serde_json::{json, Value};

use crate::error::Result;
use crate::types::AiChunk;

/// Agent error type.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AgentError {
    /// An unknown tool name was invoked.
    #[error("unknown tool: {0}")]
    UnknownTool(String),
}

/// A callable tool exposed to an agent (Laravel `#[derive(Tool)]` parity).
///
/// Tool contracts are JSON Schema ([`Tool::schema`]); execution is an
/// async `run` over a `serde_json` argument payload returning a JSON value.
#[async_trait::async_trait]
pub trait Tool: Send + Sync + 'static {
    /// Stable tool name (`search_docs`).
    fn name(&self) -> &'static str;

    /// Tool description surfaced to the model.
    fn description(&self) -> &'static str;

    /// JSON Schema for the tool's arguments.
    fn schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    /// Execute the tool with parsed arguments.
    async fn run(&self, arguments: Value) -> Result<Value>;
}

/// Tool registry owned by an agent.
#[derive(Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool by name.
    pub fn register<T: Tool>(&mut self, tool: T) -> &mut Self {
        self.tools.insert(tool.name(), Box::new(tool));
        self
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Invoke a registered tool; unknown names error.
    pub async fn invoke(&self, name: &str, arguments: Value) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AgentError::UnknownTool(name.to_string()))?;
        tool.run(arguments).await
    }
}

/// Streamed run outcome carrying ordered token chunks.
#[derive(Debug)]
pub struct AgentRun {
    /// Token chunks in emission order (`event: token` framing over WS).
    pub chunks: Vec<AiChunk>,
    /// Final tool results keyed by tool name.
    pub tool_results: Vec<(String, Value)>,
}

/// Agent lifecycle hook (Laravel agent middleware parity).
///
/// Middleware runs around agent execution — the M6 contract requires
/// middleware to be *observed* firing around tool execution. Each hook gets
/// the full run state so middleware can record or annotate.
#[async_trait::async_trait]
pub trait AgentMiddleware: Send + Sync + 'static {
    /// Called before the agent executes a prompt.
    async fn before_run(&self, prompt: &str);

    /// Called after the agent produced a run (full outcome visible).
    async fn after_run(&self, prompt: &str, run: &AgentRun);
}

/// Configurable agent over a provider.
///
/// An agent owns a tool registry and a middleware chain and runs prompts to
/// token streams (FS-M6-07 contract: tools invoked, output streamed,
/// middleware observed).
#[derive(Clone)]
pub struct Agent {
    /// Provider this agent runs on.
    pub provider: &'static str,
    tools: std::sync::Arc<ToolRegistry>,
    middleware: std::sync::Arc<Vec<Box<dyn AgentMiddleware>>>,
}

impl Agent {
    /// Create an agent bound to a provider name.
    pub fn new(provider: &'static str) -> Self {
        Self {
            provider,
            tools: std::sync::Arc::new(ToolRegistry::new()),
            middleware: std::sync::Arc::new(Vec::new()),
        }
    }

    /// Register a tool on this agent.
    pub fn tool<T: Tool>(mut self, tool: T) -> Self {
        std::sync::Arc::get_mut(&mut self.tools)
            .expect("agent tool registration requires exclusive ownership")
            .register(tool);
        self
    }

    /// Append a middleware to this agent's chain.
    pub fn middleware<M: AgentMiddleware>(mut self, middleware: M) -> Self {
        std::sync::Arc::get_mut(&mut self.middleware)
            .expect("agent middleware registration requires exclusive ownership")
            .push(Box::new(middleware));
        self
    }

    /// Stream a prompt through the agent.
    ///
    /// Stub: emits a single `token` chunk naming the provider, invokes tools
    /// embedded in the prompt (`tool:name(args)`), and fires middleware
    /// around the run.
    pub async fn run(&self, prompt: &str) -> AgentRun {
        for middleware in self.middleware.iter() {
            middleware.before_run(prompt).await;
        }
        let run = self.run_inner(prompt).await;
        for middleware in self.middleware.iter() {
            middleware.after_run(prompt, &run).await;
        }
        run
    }

    async fn run_inner(&self, prompt: &str) -> AgentRun {
        let mut chunks = vec![AiChunk::token(
            self.provider,
            format!("[{provider} echo] {prompt}", provider = self.provider),
        )];
        let mut tool_results = Vec::new();
        if let Some(call) = parse_tool_call(prompt) {
            if let Some(tool) = self.tools.get(&call.0) {
                if let Ok(result) = tool.run(call.1.clone()).await {
                    tool_results.push((call.0.clone(), result));
                    chunks.push(AiChunk {
                        kind: "tool_call".to_string(),
                        text: format!("{} -> {}", call.0, call.1),
                        model: self.provider.to_string(),
                    });
                }
            }
        }
        AgentRun {
            chunks,
            tool_results,
        }
    }
}

/// Parse a `tool:name({json})` invocation embedded in a prompt.
fn parse_tool_call(prompt: &str) -> Option<(String, Value)> {
    let rest = prompt.trim().strip_prefix("tool:")?;
    let name = rest.split_whitespace().next()?.to_string();
    let payload = rest[name.len()..]
        .trim()
        .strip_prefix('(')?
        .strip_suffix(')')?;
    serde_json::from_str::<Value>(payload)
        .ok()
        .map(|v| (name, v))
}

/// Tool calls from an agent run (alias used by `Ai::agent` output).
pub type ToolCall = (String, Value);

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "Echoes its input back"
        }

        async fn run(&self, arguments: Value) -> Result<Value> {
            Ok(arguments)
        }
    }

    #[tokio::test]
    async fn agent_runs_prompt_and_invokes_tool() {
        let agent = Agent::new("echo").tool(EchoTool);
        let run = agent.run("tool:echo({\"a\":1})").await;
        assert_eq!(run.chunks.len(), 2);
        assert_eq!(run.tool_results.len(), 1);
        assert_eq!(run.tool_results[0].0, "echo");
    }

    #[tokio::test]
    async fn unknown_tool_is_reported_but_does_not_panic() {
        let agent = Agent::new("echo");
        let run = agent.run("tool:missing({\"a\":1})").await;
        assert!(run.tool_results.is_empty());
    }
}
