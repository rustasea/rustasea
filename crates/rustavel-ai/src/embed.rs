//! Embedding glue: `Str::to_embeddings` over an `AiProvider`.
//!
//! Docs contract (ai-agents.md US-M6-07): `Str::to_embeddings("hello",
//! provider: "openai")` → `Vec<f32>` dimension 1536 by calling
//! `AiProvider::embeddings`. The search crate owns the `Str` trait (engine
//! agnostic); this extension trait, available with the `search` feature,
//! resolves the provider call and maps the response onto the same contract.

use crate::error::Result;
use crate::provider::AiProvider;

/// `to_embeddings` extension trait bridging `rustavel-search::Str` text with
/// an `AiProvider::embeddings` call (FS-M6-07 / US-M6-07 contract).
///
/// Implemented for `str` and `String`; the provider must advertise the
/// `Embeddings` capability or a typed [`crate::error::AiError`] is returned.
#[async_trait::async_trait]
pub trait StrToEmbeddings: Send + Sync {
    /// Embed `self` via `provider`, returning the first embedding vector.
    async fn to_embeddings(&self, provider: &dyn AiProvider) -> Result<Vec<f32>>;

    /// Embed `self` via the provider resolved by `resolve`.
    ///
    /// `resolve` maps a provider name to its adapter — the app normally wires
    /// this to its `Ai` registry: `|name| ai.provider_ref(name)`.
    async fn to_embeddings_with<R>(&self, provider_name: &str, resolve: R) -> Result<Vec<f32>>
    where
        R: Fn(&str) -> Result<&dyn AiProvider> + Send + Sync,
    {
        self.to_embeddings(resolve(provider_name)?).await
    }
}

#[async_trait::async_trait]
impl StrToEmbeddings for str {
    async fn to_embeddings(&self, provider: &dyn AiProvider) -> Result<Vec<f32>> {
        let response = provider.embeddings(&[self.to_string()]).await?;
        response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::AiError::Provider {
                provider: provider.provider_name().to_string(),
                message: "embeddings response contained no vectors".to_string(),
            })
    }
}

#[async_trait::async_trait]
impl StrToEmbeddings for String {
    async fn to_embeddings(&self, provider: &dyn AiProvider) -> Result<Vec<f32>> {
        self.as_str().to_embeddings(provider).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InProcessProvider;
    use crate::types::EmbeddingResponse;

    struct EmbeddingEchoProvider;

    #[async_trait::async_trait]
    impl AiProvider for EmbeddingEchoProvider {
        fn provider_name(&self) -> &'static str {
            "embed-echo"
        }

        fn capabilities(&self) -> Vec<crate::provider::Capability> {
            vec![crate::provider::Capability::Embeddings]
        }

        async fn embeddings(&self, inputs: &[String]) -> Result<EmbeddingResponse> {
            Ok(EmbeddingResponse {
                data: inputs
                    .iter()
                    .map(|input| vec![input.len() as f32])
                    .collect(),
                model: "embed-echo".to_string(),
            })
        }

        async fn text(
            &self,
            _request: &crate::provider::TextRequest,
        ) -> Result<crate::types::TextResponse> {
            Err(crate::error::AiError::unsupported(
                self.provider_name(),
                "text",
            ))
        }
    }

    #[tokio::test]
    async fn to_embeddings_calls_provider() {
        let provider = EmbeddingEchoProvider;
        let vector = "hello".to_embeddings(&provider).await.unwrap();
        assert_eq!(vector, vec![5.0]);
    }

    #[tokio::test]
    async fn to_embeddings_openai_dimension_is_1536() {
        let provider = InProcessProvider::openai();
        let vector = "hello".to_embeddings(&provider).await.unwrap();
        assert_eq!(vector.len(), crate::adapters::EMBEDDING_DIM);
        assert_eq!(vector.len(), 1536);
    }
}
