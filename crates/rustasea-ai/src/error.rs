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

    /// MCP support was requested without the `mcp` feature.
    #[error("MCP support is unavailable: {hint}")]
    McpUnavailable {
        /// Remediation hint for the caller.
        hint: String,
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
}
