//! Provider-agnostic AI trait over the M6 provider catalogue.

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{
    AudioResponse, ContentPart, EmbeddingResponse, ImageResponse, RerankResponse, TextResponse,
};

/// Text-completion request.
#[derive(Debug, Clone)]
pub struct TextRequest {
    /// System prompt.
    pub system: Option<String>,
    /// User turn(s).
    pub messages: Vec<String>,
    /// Model override (`None` = provider default).
    pub model: Option<String>,
    /// Sampling temperature.
    pub temperature: f32,
}

impl TextRequest {
    /// Build a single-turn request from a prompt.
    pub fn prompt(prompt: impl Into<String>) -> Self {
        Self {
            system: None,
            messages: vec![prompt.into()],
            model: None,
            temperature: 0.7,
        }
    }
}

/// Capabilities a provider may implement (opt-in per provider).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Text generation (`text`).
    Text,
    /// Image generation (`image`).
    Image,
    /// Audio generation (`audio`).
    Audio,
    /// Embeddings (`embeddings`).
    Embeddings,
    /// Reranking (`reranking`).
    Reranking,
    /// File ingestion (`files`).
    Files,
    /// Vector store management (`vector_stores`).
    VectorStores,
}

impl Capability {
    /// Stable capability name for errors/registry.
    pub fn name(&self) -> &'static str {
        match self {
            Capability::Text => "text",
            Capability::Image => "image",
            Capability::Audio => "audio",
            Capability::Embeddings => "embeddings",
            Capability::Reranking => "reranking",
            Capability::Files => "files",
            Capability::VectorStores => "vector_stores",
        }
    }
}

/// Provider-agnostic AI interface over 12 providers.
///
/// Every adapter (`openai`, `anthropic`, `gemini`, `azure`, `bedrock`,
/// `groq`, `xai`, `deepseek`, `mistral`, `ollama`, `openrouter`,
/// `openai_compatible`) implements this single trait; unsupported
/// capabilities return [`crate::error::AiError::UnsupportedCapability`]
/// rather than panicking (versioned trait + semver per adapter).
#[async_trait]
pub trait AiProvider: Send + Sync + 'static {
    /// Provider name (`anthropic`, `ollama`, …).
    fn provider_name(&self) -> &'static str;

    /// Which capabilities this provider implements.
    fn capabilities(&self) -> Vec<Capability>;

    /// Convenience guard: error when `capability` is unsupported.
    fn require(&self, capability: Capability) -> Result<()> {
        if self.capabilities().contains(&capability) {
            Ok(())
        } else {
            Err(crate::error::AiError::unsupported(
                self.provider_name(),
                capability.name(),
            ))
        }
    }

    /// Generate text from a request.
    async fn text(&self, request: &TextRequest) -> Result<TextResponse>;

    /// Stream text tokens for a request.
    ///
    /// The default implementation returns a single-chunk stream stub derived
    /// from the non-streaming response; adapters with real streaming override
    /// it with their own `futures_core::Stream`.
    async fn stream_text(
        &self,
        request: &TextRequest,
    ) -> Result<futures_core::stream::BoxStream<'static, Result<crate::types::AiChunk>>> {
        let response = self.text(request).await?;
        let stream = crate::streaming::single_chunk_stream(response.model, response.text);
        Ok(Box::pin(stream))
    }

    /// Generate an image.
    async fn image(&self, _prompt: &str) -> Result<ImageResponse> {
        self.require(Capability::Image)?;
        Err(crate::error::AiError::unsupported(
            self.provider_name(),
            "image",
        ))
    }

    /// Generate audio.
    async fn audio(&self, _prompt: &str) -> Result<AudioResponse> {
        self.require(Capability::Audio)?;
        Err(crate::error::AiError::unsupported(
            self.provider_name(),
            "audio",
        ))
    }

    /// Embed text inputs.
    async fn embeddings(&self, _inputs: &[String]) -> Result<EmbeddingResponse> {
        self.require(Capability::Embeddings)?;
        Err(crate::error::AiError::unsupported(
            self.provider_name(),
            "embeddings",
        ))
    }

    /// Rerank documents against a query.
    async fn rerank(&self, _query: &str, _documents: &[String]) -> Result<RerankResponse> {
        self.require(Capability::Reranking)?;
        Err(crate::error::AiError::unsupported(
            self.provider_name(),
            "reranking",
        ))
    }

    /// Ingest a file by content parts.
    async fn files(&self, _parts: Vec<ContentPart>) -> Result<()> {
        self.require(Capability::Files)?;
        Err(crate::error::AiError::unsupported(
            self.provider_name(),
            "files",
        ))
    }
}

/// Marker for streaming-capable providers.
pub trait StreamingProvider: AiProvider {
    /// True when the adapter streams token chunks.
    fn supports_streaming(&self) -> bool {
        true
    }
}
