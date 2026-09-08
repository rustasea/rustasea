/// Broadcast error type for the realtime layer.
use thiserror::Error;

/// Alias for results produced by broadcast operations.
pub type Result<T> = std::result::Result<T, BroadcastError>;

/// Top-level broadcast error type.
#[derive(Debug, Error)]
pub enum BroadcastError {
    /// A channel requires authentication but no identity was supplied.
    #[error("channel {channel} requires authentication")]
    Unauthenticated {
        /// Channel that demanded an identity.
        channel: String,
    },

    /// The caller is not allowed on a private/presence channel.
    #[error("forbidden on channel {channel}: {reason}")]
    Forbidden {
        /// Channel the caller attempted to join.
        channel: String,
        /// Authorization denial reason.
        reason: String,
    },

    /// The underlying transport (WebSocket/SSE) failed.
    #[error("broadcast transport failed: {0}")]
    Transport(String),

    /// A serialization failure occurred while encoding a payload.
    #[error("broadcast serialization failed: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for BroadcastError {
    /// Convert a JSON encoding failure into a typed broadcast error.
    fn from(e: serde_json::Error) -> Self {
        BroadcastError::Serialization(e.to_string())
    }
}
