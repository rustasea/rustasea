/// JSON:API error types.
use thiserror::Error;

/// Alias for results produced by resource rendering.
pub type Result<T> = std::result::Result<T, JsonApiError>;

/// Top-level JSON:API error type.
#[derive(Debug, Error)]
pub enum JsonApiError {
    /// A resource type was rendered but carries no id.
    #[error("resource of type {0} rendered without an id")]
    MissingId(&'static str),

    /// A relationship was requested (via `include`) but never loaded.
    #[error("relationship {relationship} was not loaded on resource type {resource_type}")]
    RelationNotLoaded {
        /// Relationship name requested in `include`.
        relationship: String,
        /// Resource type that should have loaded it.
        resource_type: String,
    },

    /// Field filtering produced an empty selection for a type.
    #[error("sparse fieldsets for {0} excluded every attribute")]
    EmptySparseFieldset(String),
}
