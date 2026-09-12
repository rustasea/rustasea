//! Mail error types.

use thiserror::Error;

use rustasea_queue::QueueError;

/// Alias for results produced by mail operations.
pub type Result<T> = std::result::Result<T, MailError>;

/// Top-level mail error type.
#[derive(Debug, Error)]
pub enum MailError {
    /// The transport failed to deliver the message.
    #[error("mail transport failed: {0}")]
    Transport(String),

    /// An address is malformed or a required recipient is missing.
    #[error("invalid mail recipient: {0}")]
    InvalidRecipient(String),

    /// A message could not be assembled from its parts.
    #[error("mail message could not be built: {0}")]
    Render(String),

    /// Enqueueing a queued notification failed.
    #[error(transparent)]
    Queue(#[from] QueueError),
}
