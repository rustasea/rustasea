//! MCP (Model Context Protocol) support stub.
//!
//! Tool discovery over MCP servers is gated behind the `mcp` feature; without
//! it every MCP operation degrades to [`crate::error::AiError::McpUnavailable`]
//! so the core crate never pulls MCP deps (NFR-Sca-02).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
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
}

/// MCP tool discovery registry.
///
/// Real discovery talks to MCP servers over stdio/streamable HTTP in M6-full;
/// the stub records configured servers and reports availability per the
/// `mcp` feature flag.
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
    /// Feature-gated: `mcp` on returns an empty discovery (stub), off returns
    /// [`AiError::McpUnavailable`] — never a panic.
    #[cfg(feature = "mcp")]
    pub fn discover_tools(&self) -> Result<Vec<McpTool>> {
        Ok(Vec::new())
    }

    /// Discover tools from configured servers (degraded mode).
    #[cfg(not(feature = "mcp"))]
    pub fn discover_tools(&self) -> Result<Vec<McpTool>> {
        let _ = &self.servers;
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

    #[cfg(not(feature = "mcp"))]
    #[test]
    fn discovery_degrades_without_feature() {
        let registry = McpRegistry::new();
        assert!(registry.discover_tools().is_err());
    }
}
