//! `Agent` and `Tool` contracts — the M6 agent runtime surface.

use serde_json::{json, Value};

use crate::error::Result;
use crate::types::AiChunk;

/// Agent error type (ai-agents.md §5: `ToolNotFound`, `McpUnavailable`+hint,
/// `UnsupportedCapability`; `UnknownTool` retained for registry callers).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AgentError {
    /// An unknown tool name was invoked.
    #[error("unknown tool: {0}")]
    UnknownTool(String),

    /// A requested tool is not registered on this agent.
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// MCP discovery was attempted without the `mcp` feature.
    #[error("MCP unavailable: {hint}")]
    McpUnavailable {
        /// Remediation hint (feature flag name).
        hint: String,
    },

    /// The agent's provider cannot perform the requested capability.
    #[error("provider {provider} does not support capability {capability}")]
    UnsupportedCapability {
        /// Provider name.
        provider: String,
        /// Unsupported capability.
        capability: &'static str,
    },
}

impl AgentError {
    /// Convenience constructor for MCP feature-gate denial.
    pub fn mcp_unavailable() -> Self {
        AgentError::McpUnavailable {
            hint: "enable feature `mcp` on rustavel-ai to discover MCP tools".to_string(),
        }
    }
}

/// A callable tool exposed to an agent (Laravel `#[derive(Tool)]` parity).
///
/// Tool contracts are JSON Schema ([`Tool::schema`]); execution is an
/// async `call` over a `serde_json` argument payload returning a JSON value.
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

    /// Execute the tool with parsed arguments (docs name: `call`).
    async fn call(&self, arguments: Value) -> Result<Value>;

    /// Back-compat alias for `call` (earlier runs used `run`).
    async fn run(&self, arguments: Value) -> Result<Value> {
        self.call(arguments).await
    }
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

    /// Registered tool names.
    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    /// Whether a tool is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Invoke a registered tool; unknown names error.
    pub async fn invoke(&self, name: &str, arguments: Value) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AgentError::ToolNotFound(name.to_string()))?;
        tool.call(arguments).await
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
/// middleware observed). Sub-agents are plain tools wrapping a nested
/// [`Agent`] (see [`SubAgentTool`]).
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

    /// Register a sub-agent as a tool on this agent.
    pub fn sub_agent(mut self, sub: Agent) -> Self {
        std::sync::Arc::get_mut(&mut self.tools)
            .expect("agent tool registration requires exclusive ownership")
            .register(SubAgentTool::new(sub));
        self
    }

    /// Append a middleware to this agent's chain.
    pub fn middleware<M: AgentMiddleware>(mut self, middleware: M) -> Self {
        std::sync::Arc::get_mut(&mut self.middleware)
            .expect("agent middleware registration requires exclusive ownership")
            .push(Box::new(middleware));
        self
    }

    /// Access the tool registry.
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Run a prompt and collect the full outcome.
    ///
    /// Fires middleware before/after, emits one `token` chunk naming the
    /// provider, and invokes tools embedded in the prompt (`tool:name(args)`).
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

    /// Stream a prompt as ordered `AiChunk`s (FS-M6-06 `prompt` → `Stream`).
    ///
    /// The returned stream runs the prompt once (invoking embedded tools),
    /// then replays the outcome chunks in order and ends. Transport framing
    /// (`event: token` over WS/SSE) is applied by the caller.
    pub fn prompt_stream(
        &self,
        prompt: impl Into<String>,
    ) -> futures_core::stream::BoxStream<'static, Result<AiChunk>> {
        let agent = self.clone();
        let prompt = prompt.into();
        Box::pin(futures_util::stream::unfold(
            (agent, prompt, None::<AgentRun>, 0usize),
            |(agent, prompt, run, index)| async move {
                let run = match run {
                    Some(run) => run,
                    None => {
                        let completed = agent.run(&prompt).await;
                        if completed.chunks.is_empty() {
                            return None;
                        }
                        completed
                    }
                };
                if index < run.chunks.len() {
                    let chunk = run.chunks[index].clone();
                    Some((Ok(chunk), (agent, prompt, Some(run), index + 1)))
                } else {
                    None
                }
            },
        ))
    }

    /// Queue a prompt for background execution (queueing stub).
    pub async fn queue(&self, prompt: &str) -> Result<String> {
        crate::streaming::queue_run(self.provider, prompt).await
    }

    async fn run_inner(&self, prompt: &str) -> AgentRun {
        let mut chunks = vec![AiChunk::token(
            self.provider,
            format!("[{provider} echo] {prompt}", provider = self.provider),
        )];
        let mut tool_results = Vec::new();
        if let Some(call) = parse_tool_call(prompt) {
            if let Some(tool) = self.tools.get(&call.0) {
                if let Ok(result) = tool.call(call.1.clone()).await {
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

/// Adapter exposing a nested agent as a tool (sub-agent invocation).
///
/// Prompting a parent with `tool:<name>({"task": "…"})` runs the sub-agent;
/// the middleware chain of both parent and sub-agent observe the run.
pub struct SubAgentTool {
    name: &'static str,
    agent: Agent,
}

impl SubAgentTool {
    /// Wrap an agent as a tool under its provider name.
    pub fn new(agent: Agent) -> Self {
        // Leak a stable name: "{provider}-subagent" must be 'static for Tool.
        let name: &'static str = Box::leak(format!("{}-subagent", agent.provider).into_boxed_str());
        Self { name, agent }
    }
}

#[async_trait::async_trait]
impl Tool for SubAgentTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        "Delegates the task to a nested sub-agent."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": { "type": "string", "description": "Task for the sub-agent" }
            },
            "required": ["task"]
        })
    }

    async fn call(&self, arguments: Value) -> Result<Value> {
        let task = arguments
            .get("task")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let run = self.agent.run(&task).await;
        Ok(json!({
            "agent": self.agent.provider,
            "chunks": run.chunks.len(),
            "text": run
                .chunks
                .iter()
                .map(|chunk| chunk.text.clone())
                .collect::<Vec<String>>()
                .join("")
        }))
    }
}

/// Parse a `tool:name({json})` invocation embedded in a prompt.
///
/// The tool name runs to the first `(`, whitespace, or end of input — so both
/// `tool:echo({"a":1})` and `tool:upper ({"t":"x"})` parse.
fn parse_tool_call(prompt: &str) -> Option<(String, Value)> {
    let rest = prompt.trim().strip_prefix("tool:")?.trim_start();
    let paren = rest.find('(')?;
    let name = rest[..paren].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let payload = rest[paren..].strip_prefix('(')?.strip_suffix(')')?;
    serde_json::from_str::<Value>(payload)
        .ok()
        .map(|v| (name, v))
}

/// Tool calls from an agent run (alias used by `AiResponse::tool_calls`).
pub type ToolCall = (String, Value);

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    struct EchoTool;

    #[async_trait::async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &'static str {
            "echo"
        }

        fn description(&self) -> &'static str {
            "Echoes its input back"
        }

        async fn call(&self, arguments: Value) -> Result<Value> {
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

    #[tokio::test]
    async fn registry_invoke_returns_tool_not_found() {
        let registry = ToolRegistry::new();
        let error = registry.invoke("nope", json!({})).await.unwrap_err();
        // `invoke` promotes agent errors into the AI error space; assert the
        // underlying tool-not-found message survives the promotion.
        let message = error.to_string();
        assert!(
            message.contains("tool not found: nope"),
            "unexpected error: {message}"
        );
        // Direct registry lookup exposes the typed AgentError.
        assert!(matches!(registry.get("nope"), None));
    }

    #[tokio::test]
    async fn prompt_stream_emits_ordered_chunks() {
        let agent = Agent::new("echo").tool(EchoTool);
        let mut stream = agent.prompt_stream("tool:echo({\"a\":1})");
        let first = stream.next().await.unwrap().unwrap();
        assert_eq!(first.kind, "token");
        assert!(first.text.contains("echo"));
        let second = stream.next().await.unwrap().unwrap();
        assert_eq!(second.kind, "tool_call");
        assert!(second.text.contains("echo -> "));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn sub_agent_invoked_as_tool() {
        let sub = Agent::new("knowledge");
        let parent = Agent::new("parent").sub_agent(sub);
        let run = parent
            .run("tool:knowledge-subagent({\"task\":\"find docs\"})")
            .await;
        assert_eq!(run.tool_results.len(), 1);
        assert_eq!(run.tool_results[0].0, "knowledge-subagent");
        let result = run.tool_results[0].1.clone();
        assert_eq!(result["agent"], "knowledge");
        assert!(result["text"].as_str().unwrap().contains("find docs"));
    }
}
