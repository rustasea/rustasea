//! Client-side errors.

/// Error produced by the Inertia client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The page JSON could not be parsed into a page envelope.
    #[error("failed to parse the Inertia page")]
    ParsePage {
        /// Underlying JSON failure.
        #[source]
        source: serde_json::Error,
    },

    /// No component is registered for the page's key.
    #[error("unknown Inertia component `{component}`")]
    UnknownComponent {
        /// Component key that could not be resolved.
        component: String,
    },

    /// A component's mount function failed.
    #[error("failed to mount component `{component}`: {message}")]
    Mount {
        /// Component key being mounted.
        component: String,
        /// Human-readable failure detail.
        message: String,
    },

    /// A `409` response omitted the required `X-Inertia-Location` header.
    #[error("the version-conflict response is missing X-Inertia-Location")]
    MissingLocation,
}
