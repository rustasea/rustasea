//! Adapter-level errors.

use rustasea_inertia_client::ClientError;

/// Error produced by a WASM presentation adapter.
///
/// Wraps the protocol [`ClientError`] and adds the browser-transport failure
/// that can only happen once a real `fetch` is issued.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The underlying Inertia client rejected the operation (parse, mount, or a
    /// malformed version-conflict response).
    #[error(transparent)]
    Client(#[from] ClientError),

    /// The browser request failed before an Inertia response was received.
    #[error("the Inertia request failed: {0}")]
    Request(String),
}
