//! Model/response primitives shared across providers.

use serde::{Deserialize, Serialize};

/// One text-completion chunk emitted during streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiChunk {
    /// Chunk kind (`token`, `tool_call`, `done`).
    pub kind: String,
    /// Token text for `token` chunks.
    pub text: String,
    /// Provider/model that produced the chunk.
    pub model: String,
}

impl AiChunk {
    /// Create a text token chunk.
    pub fn token(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            kind: "token".to_string(),
            text: text.into(),
            model: model.into(),
        }
    }
}

/// Content part of a multimodal provider response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content.
    Text(String),
    /// Base64-encoded image bytes.
    Image { mime: String, base64: String },
    /// Base64-encoded audio bytes.
    Audio { mime: String, base64: String },
}

/// Text-completion response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextResponse {
    /// Generated text.
    pub text: String,
    /// Provider/model identifier.
    pub model: String,
    /// Token usage when reported.
    pub usage: Option<TokenUsage>,
}

/// Image-generation response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageResponse {
    /// One generated image (URL or base64 depending on provider).
    pub images: Vec<String>,
    /// Provider/model identifier.
    pub model: String,
}

/// Audio-generation response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioResponse {
    /// One generated audio clip (URL or base64 depending on provider).
    pub audio: Vec<String>,
    /// Provider/model identifier.
    pub model: String,
}

/// Embeddings response (alias over the search crate when enabled).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Flattened embedding vectors, one per input, in order.
    pub data: Vec<Vec<f32>>,
    /// Provider/model identifier.
    pub model: String,
}

/// Reranking response: original indices reordered by relevance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RerankResponse {
    /// Indices into the input document list, most relevant first.
    pub indices: Vec<usize>,
    /// Relevance scores aligned with `indices`.
    pub scores: Vec<f32>,
    /// Provider/model identifier.
    pub model: String,
}

/// Token usage counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt tokens.
    pub prompt: u32,
    /// Completion tokens.
    pub completion: u32,
}
