//! Twelve provider adapters behind the `AiProvider` trait.
//!
//! Each adapter (`openai` … `openai_compatible`) is feature-flag isolated in
//! M6-full via per-provider SDKs; this crate ships in-process adapters that
//! satisfy the trait contract and capability matrix without pulling SDK
//! dependencies (NFR-Sca-02). `ProviderCall` provides the
//! `Ai::provider(name).text(prompt).send()` builder chain.
//!
//! Capability matrix highlights (ai-sdk.md): `ollama` lacks `reranking`,
//! `groq` lacks `files` — every other cell is provider-typical.

use crate::error::{AiError, Result};
use crate::provider::{AiProvider, Capability, TextRequest};
use crate::types::{
    AiResponse, AudioResponse, ContentPart, EmbeddingResponse, ImageResponse, RerankResponse,
    TextResponse, TokenUsage,
};

/// Number of providers in the M6 catalogue.
pub const PROVIDER_COUNT: usize = 12;

/// Stable ordered list of provider names (TC-M6-16 invariant).
pub const PROVIDERS: [&str; PROVIDER_COUNT] = [
    "openai",
    "anthropic",
    "gemini",
    "azure",
    "bedrock",
    "groq",
    "xai",
    "deepseek",
    "mistral",
    "ollama",
    "openrouter",
    "openai_compatible",
];

/// Embedding dimension advertised by the in-process text-embedding adapters.
pub const EMBEDDING_DIM: usize = 1536;

/// Return the ordered catalogue of provider names.
pub fn catalogue() -> Vec<&'static str> {
    PROVIDERS.to_vec()
}

/// In-process provider adapter.
///
/// Emits deterministic placeholder outputs; real network calls are supplied
/// by M6-full per-provider SDK adapters behind the same trait.
pub struct InProcessProvider {
    name: &'static str,
    capabilities: Vec<Capability>,
}

impl InProcessProvider {
    /// Create an adapter with the given capability set.
    pub fn new(name: &'static str, capabilities: Vec<Capability>) -> Self {
        Self { name, capabilities }
    }

    /// Whether reranking is advertised.
    fn supports(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}

impl InProcessProvider {
    /// Adapter with all seven capabilities (openai).
    pub fn openai() -> Self {
        Self::new(
            "openai",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Audio,
                Capability::Embeddings,
                Capability::Reranking,
                Capability::Files,
                Capability::VectorStores,
            ],
        )
    }

    /// Anthropic adapter (text, image, files, vector stores).
    pub fn anthropic() -> Self {
        Self::new(
            "anthropic",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Files,
                Capability::VectorStores,
            ],
        )
    }

    /// Gemini adapter (text, image, audio, embeddings, files).
    pub fn gemini() -> Self {
        Self::new(
            "gemini",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Audio,
                Capability::Embeddings,
                Capability::Files,
            ],
        )
    }

    /// Azure OpenAI adapter.
    pub fn azure() -> Self {
        Self::new(
            "azure",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Embeddings,
                Capability::Reranking,
                Capability::Files,
                Capability::VectorStores,
            ],
        )
    }

    /// Bedrock adapter.
    pub fn bedrock() -> Self {
        Self::new(
            "bedrock",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Embeddings,
                Capability::Reranking,
                Capability::VectorStores,
            ],
        )
    }

    /// Groq adapter — no `files` capability (docs AC).
    pub fn groq() -> Self {
        Self::new(
            "groq",
            vec![
                Capability::Text,
                Capability::Audio,
                Capability::Embeddings,
                Capability::Reranking,
            ],
        )
    }

    /// xAI adapter.
    pub fn xai() -> Self {
        Self::new("xai", vec![Capability::Text, Capability::Embeddings])
    }

    /// DeepSeek adapter.
    pub fn deepseek() -> Self {
        Self::new("deepseek", vec![Capability::Text, Capability::Embeddings])
    }

    /// Mistral adapter.
    pub fn mistral() -> Self {
        Self::new(
            "mistral",
            vec![
                Capability::Text,
                Capability::Embeddings,
                Capability::Reranking,
                Capability::Files,
                Capability::VectorStores,
            ],
        )
    }

    /// Ollama adapter — no `reranking` capability (docs AC).
    pub fn ollama() -> Self {
        Self::new(
            "ollama",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Audio,
                Capability::Embeddings,
                Capability::VectorStores,
            ],
        )
    }

    /// OpenRouter aggregator adapter.
    pub fn openrouter() -> Self {
        Self::new(
            "openrouter",
            vec![
                Capability::Text,
                Capability::Image,
                Capability::Audio,
                Capability::Embeddings,
            ],
        )
    }

    /// OpenAI-compatible endpoint adapter (self-hosted vLLM etc.).
    pub fn openai_compatible() -> Self {
        Self::new(
            "openai_compatible",
            vec![Capability::Text, Capability::Embeddings, Capability::Files],
        )
    }
}

/// Deterministic 1536-dim placeholder embedding for an input.
pub fn stub_embedding(text: &str) -> Vec<f32> {
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    for byte in text.bytes() {
        seed = seed
            .wrapping_mul(1_099_511_628_211)
            .wrapping_add(u64::from(byte));
    }
    (0..EMBEDDING_DIM)
        .map(|i| {
            let mut x = seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            x ^= x >> 30;
            x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x ^= x >> 27;
            x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^= x >> 31;
            (x as f32 / u64::MAX as f32) - 0.5
        })
        .collect()
}

/// Estimate placeholder token counts from text length.
fn token_count(text: &str) -> u32 {
    (text.chars().count() / 4).max(1) as u32
}

#[async_trait::async_trait]
impl AiProvider for InProcessProvider {
    fn provider_name(&self) -> &'static str {
        self.name
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    async fn text(&self, request: &TextRequest) -> Result<TextResponse> {
        let joined = request.messages.join(" ");
        let text = format!("[{} in-process] {joined}", self.name);
        Ok(TextResponse {
            usage: Some(TokenUsage {
                prompt: token_count(&joined),
                completion: token_count(&text),
            }),
            text,
            model: self.name.to_string(),
        })
    }

    async fn image(&self, prompt: &str) -> Result<ImageResponse> {
        if !self.supports(Capability::Image) {
            return Err(AiError::unsupported(self.name, "image"));
        }
        Ok(ImageResponse {
            images: vec![format!("[{}-image] {prompt}", self.name)],
            model: self.name.to_string(),
        })
    }

    async fn audio(&self, prompt: &str) -> Result<AudioResponse> {
        if !self.supports(Capability::Audio) {
            return Err(AiError::unsupported(self.name, "audio"));
        }
        Ok(AudioResponse {
            audio: vec![format!("[{}-audio] {prompt}", self.name)],
            model: self.name.to_string(),
        })
    }

    async fn embeddings(&self, inputs: &[String]) -> Result<EmbeddingResponse> {
        if !self.supports(Capability::Embeddings) {
            return Err(AiError::unsupported(self.name, "embeddings"));
        }
        Ok(EmbeddingResponse {
            data: inputs.iter().map(|input| stub_embedding(input)).collect(),
            model: self.name.to_string(),
        })
    }

    async fn rerank(&self, query: &str, documents: &[String]) -> Result<RerankResponse> {
        if !self.supports(Capability::Reranking) {
            return Err(AiError::unsupported(self.name, "reranking"));
        }
        let indices: Vec<usize> = (0..documents.len()).collect();
        let scores: Vec<f32> = (0..documents.len())
            .map(|i| 1.0 - (i as f32 / documents.len().max(1) as f32))
            .collect();
        Ok(RerankResponse {
            indices,
            scores,
            model: format!("{}:{query}", self.name),
        })
    }

    async fn files(&self, _parts: Vec<ContentPart>) -> Result<()> {
        if !self.supports(Capability::Files) {
            return Err(AiError::unsupported(self.name, "files"));
        }
        Ok(())
    }
}

/// Builder facade over one provider (`Ai::provider(name).text(...).send()`).
#[derive(Clone, Copy)]
pub struct ProviderCall<'a> {
    provider: &'a dyn AiProvider,
}

impl<'a> ProviderCall<'a> {
    /// Wrap a resolved provider into the call builder.
    pub fn new(provider: &'a dyn AiProvider) -> Self {
        Self { provider }
    }

    /// Begin a text-completion call.
    pub fn text(self, prompt: impl Into<String>) -> TextCall<'a> {
        TextCall {
            provider: self.provider,
            prompt: prompt.into(),
        }
    }

    /// Begin an embeddings call.
    pub fn embeddings(self, inputs: Vec<String>) -> EmbeddingsCall<'a> {
        EmbeddingsCall {
            provider: self.provider,
            inputs,
        }
    }

    /// Begin a reranking call.
    pub fn rerank(self, query: impl Into<String>, documents: Vec<String>) -> RerankCall<'a> {
        RerankCall {
            provider: self.provider,
            query: query.into(),
            documents,
        }
    }

    /// Begin a files-ingestion call.
    pub fn files(self, parts: Vec<ContentPart>) -> FilesCall<'a> {
        FilesCall {
            provider: self.provider,
            parts,
        }
    }
}

/// In-flight text call; resolves on [`TextCall::send`].
pub struct TextCall<'a> {
    provider: &'a dyn AiProvider,
    prompt: String,
}

impl<'a> TextCall<'a> {
    /// Send the prompt and await the uniform response.
    pub async fn send(self) -> Result<AiResponse> {
        let response = self
            .provider
            .text(&TextRequest::prompt(self.prompt))
            .await?;
        Ok(response.into())
    }
}

/// In-flight embeddings call; resolves on [`EmbeddingsCall::send`].
pub struct EmbeddingsCall<'a> {
    provider: &'a dyn AiProvider,
    inputs: Vec<String>,
}

impl<'a> EmbeddingsCall<'a> {
    /// Send the inputs and await the embedding vectors.
    pub async fn send(self) -> Result<EmbeddingResponse> {
        self.provider.embeddings(&self.inputs).await
    }
}

/// In-flight rerank call; resolves on [`RerankCall::send`].
pub struct RerankCall<'a> {
    provider: &'a dyn AiProvider,
    query: String,
    documents: Vec<String>,
}

impl<'a> RerankCall<'a> {
    /// Send the query/docs pair and await relevance ordering.
    pub async fn send(self) -> Result<RerankResponse> {
        self.provider.rerank(&self.query, &self.documents).await
    }
}

/// In-flight files-ingestion call; resolves on [`FilesCall::send`].
pub struct FilesCall<'a> {
    provider: &'a dyn AiProvider,
    parts: Vec<ContentPart>,
}

impl<'a> FilesCall<'a> {
    /// Send the content parts for ingestion.
    pub async fn send(self) -> Result<()> {
        self.provider.files(self.parts).await
    }
}
