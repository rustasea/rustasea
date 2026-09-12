//! Provider error types.

use thiserror::Error;

/// Alias for results produced by AI operations.
pub type Result<T> = std::result::Result<T, AiError>;

/// Top-level AI SDK error type.
#[derive(Debug, Error)]
pub enum AiError {
    /// The requested capability is not supported by this provider.
    #[error("provider {provider} does not support capability {capability}")]
    UnsupportedCapability {
        /// Provider name.
        provider: String,
        /// Unsupported capability.
        capability: &'static str,
    },

    /// The provider rejected the request.
    #[error("provider error from {provider}: {message}")]
    Provider {
        /// Provider name.
        provider: String,
        /// Provider-supplied failure detail.
        message: String,
    },

    /// No provider is registered under the requested name.
    #[error("unknown AI provider: {0}")]
    UnknownProvider(String),

    /// No provider is configured (missing API key/endpoint).
    #[error("AI provider not configured: {0}")]
    NotConfigured(String),

    /// The provider rejected the supplied credentials (HTTP 401/403).
    #[error("authentication failed for provider {provider}: {message}")]
    AuthenticationFailed {
        /// Provider name.
        provider: String,
        /// Provider-supplied authentication detail.
        message: String,
    },

    /// Required provider configuration (API key, endpoint, model) is missing.
    #[error("AI provider configuration error: {0}")]
    ConfigurationError(String),

    /// Transport-level failure (connection, DNS, timeout).
    #[error("transport error for provider {provider}: {message}")]
    Transport {
        /// Provider name.
        provider: String,
        /// Underlying transport detail.
        message: String,
    },

    /// The provider rate-limited the request (HTTP 429).
    #[error("provider {provider} rate limited the request")]
    RateLimited {
        /// Provider name.
        provider: String,
        /// `Retry-After` delay in seconds when the provider supplied it.
        retry_after: Option<u64>,
    },

    /// MCP support was requested without the `mcp` feature.
    #[error("MCP support is unavailable: {hint}")]
    McpUnavailable {
        /// Remediation hint for the caller.
        hint: String,
    },

    /// An MCP server could not be reached (spawn/connect/timeout failure).
    #[error("MCP server {server} is unreachable: {message}")]
    McpServerUnreachable {
        /// Server name from [`crate::mcp::McpServerConfig`].
        server: String,
        /// Transport detail (spawn error, connect error, EOF, timeout).
        message: String,
    },

    /// An MCP server replied with a malformed or unexpected JSON-RPC frame.
    #[error("MCP protocol error from {server}: {message}")]
    McpProtocol {
        /// Server name from [`crate::mcp::McpServerConfig`].
        server: String,
        /// Protocol detail (missing field, JSON-RPC error, bad handshake).
        message: String,
    },

    /// The background queue backend rejected or could not accept an agent run.
    #[error("AI queue unavailable: {message}")]
    QueueUnavailable {
        /// Underlying queue/store detail.
        message: String,
    },

    /// A wired backend (search/storage) failed to serve a loader operation.
    #[error("backend {backend} failed: {message}")]
    Backend {
        /// Backend name (`similarity-search`, `file-storage`, …).
        backend: String,
        /// Underlying backend error detail.
        message: String,
    },
}

impl From<crate::agent::AgentError> for AiError {
    /// Promote an agent error (e.g. unknown tool) into the AI error space.
    fn from(e: crate::agent::AgentError) -> Self {
        AiError::Provider {
            provider: "agent".to_string(),
            message: e.to_string(),
        }
    }
}

impl AiError {
    /// Convenience constructor for capability denials.
    pub fn unsupported(provider: impl Into<String>, capability: &'static str) -> Self {
        AiError::UnsupportedCapability {
            provider: provider.into(),
            capability,
        }
    }

    /// Convenience constructor for the MCP feature gate denial.
    pub fn mcp_unavailable() -> Self {
        AiError::McpUnavailable {
            hint: "enable the `mcp` feature on rustasea-ai to discover MCP tools".to_string(),
        }
    }

    /// Convenience constructor for an unreachable MCP server.
    pub fn mcp_server_unreachable(server: impl Into<String>, message: impl Into<String>) -> Self {
        AiError::McpServerUnreachable {
            server: server.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for an MCP protocol violation.
    pub fn mcp_protocol(server: impl Into<String>, message: impl Into<String>) -> Self {
        AiError::McpProtocol {
            server: server.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for a queue backend failure.
    pub fn queue_unavailable(message: impl Into<String>) -> Self {
        AiError::QueueUnavailable {
            message: message.into(),
        }
    }

    /// Convenience constructor for rejected provider credentials.
    pub fn authentication_failed(provider: impl Into<String>, message: impl Into<String>) -> Self {
        AiError::AuthenticationFailed {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for missing/invalid provider configuration.
    pub fn configuration(message: impl Into<String>) -> Self {
        AiError::ConfigurationError(message.into())
    }

    /// Convenience constructor for transport-level failures.
    pub fn transport(provider: impl Into<String>, message: impl Into<String>) -> Self {
        AiError::Transport {
            provider: provider.into(),
            message: message.into(),
        }
    }
}
