//! MCP (Model Context Protocol) client — tool discovery and invocation.
//!
//! With the `mcp` feature enabled, [`McpRegistry`] connects to configured MCP
//! servers over stdio or streamable HTTP, performs the JSON-RPC handshake
//! (`initialize` → `notifications/initialized`), then discovers tools via
//! `tools/list` and invokes them via `tools/call`. Unreachable servers surface
//! as [`crate::error::AiError::McpServerUnreachable`]; malformed frames surface
//! as [`crate::error::AiError::McpProtocol`] — never a panic.
//!
//! Without the feature the crate stays dependency-light and every MCP
//! operation degrades to [`crate::error::AiError::McpUnavailable`]
//! (NFR-Sca-02).

#[cfg(feature = "mcp")]
pub mod client;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

#[cfg(feature = "mcp")]
pub use client::{McpClient, McpTransport};

/// A tool discovered from an MCP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpTool {
    /// Server the tool came from.
    pub server: String,
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON Schema of the tool's input.
    pub input_schema: Value,
}

/// Connection settings for one MCP server.
///
/// `command` is either the stdio transport program (plus `args`) or, when it
/// starts with `http://`/`https://`, the streamable-HTTP endpoint URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name (`filesystem`, `github`, …).
    pub name: String,
    /// Transport command (stdio) or URL (streamable HTTP).
    pub command: String,
    /// Arguments passed to the transport command.
    pub args: Vec<String>,
}

impl McpServerConfig {
    /// Create a stdio MCP server config.
    pub fn stdio(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// Create a stdio MCP server config with transport arguments.
    pub fn stdio_with_args<I, S>(
        name: impl Into<String>,
        command: impl Into<String>,
        args: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            command: command.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// Create a streamable-HTTP MCP server config.
    pub fn http(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: url.into(),
            args: Vec::new(),
        }
    }

    /// Whether this config addresses an HTTP endpoint rather than a stdio program.
    pub fn is_http(&self) -> bool {
        self.command.starts_with("http://") || self.command.starts_with("https://")
    }
}

/// MCP tool discovery registry.
///
/// Records configured servers and, with the `mcp` feature, discovers and
/// invokes tools against them over the real JSON-RPC transport.
#[derive(Debug, Clone, Default)]
pub struct McpRegistry {
    /// Configured servers.
    pub servers: Vec<McpServerConfig>,
}

impl McpRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an MCP server config.
    pub fn add_server(&mut self, config: McpServerConfig) -> &mut Self {
        self.servers.push(config);
        self
    }

    /// List configured servers.
    pub fn servers(&self) -> &[McpServerConfig] {
        &self.servers
    }

    /// Discover tools from configured servers.
    ///
    /// Connects to each configured server, performs the JSON-RPC handshake, and
    /// returns the union of `tools/list` results. An unreachable server yields a
    /// typed [`crate::error::AiError::McpServerUnreachable`] rather than an empty
    /// list, so a down server is never mistaken for "no tools".
    #[cfg(feature = "mcp")]
    pub async fn discover_tools(&self) -> Result<Vec<McpTool>> {
        let mut tools = Vec::new();
        for config in &self.servers {
            let mut client = McpClient::connect(config).await?;
            tools.extend(client.list_tools().await?);
        }
        Ok(tools)
    }

    /// Discover tools from configured servers (degraded mode).
    #[cfg(not(feature = "mcp"))]
    pub async fn discover_tools(&self) -> Result<Vec<McpTool>> {
        let _ = &self.servers;
        Err(crate::error::AiError::mcp_unavailable())
    }

    /// Invoke a previously discovered tool on its originating server.
    ///
    /// Reconnects to `tool.server`, then issues `tools/call`. Errors when the
    /// server is not configured or unreachable, or when the tool reports
    /// `isError`.
    #[cfg(feature = "mcp")]
    pub async fn call_tool(&self, tool: &McpTool, arguments: Value) -> Result<Value> {
        let config = self
            .servers
            .iter()
            .find(|server| server.name == tool.server)
            .ok_or_else(|| {
                crate::error::AiError::mcp_server_unreachable(
                    &tool.server,
                    "server is not configured in this registry",
                )
            })?;
        let mut client = McpClient::connect(config).await?;
        client.call_tool(&tool.name, arguments).await
    }

    /// Invoke a tool when the `mcp` feature is disabled (degraded mode).
    #[cfg(not(feature = "mcp"))]
    pub async fn call_tool(&self, tool: &McpTool, arguments: Value) -> Result<Value> {
        let _ = (tool, arguments);
        Err(crate::error::AiError::mcp_unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_records_servers() {
        let mut registry = McpRegistry::new();
        registry.add_server(McpServerConfig::stdio("fs", "mcp-server-fs"));
        assert_eq!(registry.servers().len(), 1);
    }

    #[test]
    fn http_configs_are_detected_by_scheme() {
        assert!(McpServerConfig::http("remote", "https://mcp.example/sse").is_http());
        assert!(!McpServerConfig::stdio("local", "mcp-server-fs").is_http());
    }

    #[cfg(not(feature = "mcp"))]
    #[tokio::test]
    async fn discovery_degrades_without_feature() {
        let registry = McpRegistry::new();
        assert!(matches!(
            registry.discover_tools().await,
            Err(crate::error::AiError::McpUnavailable { .. })
        ));
    }
}
