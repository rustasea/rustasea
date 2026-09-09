//! Runtime entry point: `Ai` facade with provider registry and agent factory.

use std::collections::HashMap;

use crate::adapters::ProviderCall;
use crate::agent::Agent;
use crate::error::{AiError, Result};
use crate::provider::{AiProvider, Capability};

/// AI SDK facade — provider registry + anonymous agent factory.
///
/// Mirrors the Laravel `Ai` facade: `Ai::provider("anthropic")` returns a
/// call builder over the registered provider, and `Ai::agent(...)` builds an
/// anonymous agent (FS-M6-07 contract).
#[derive(Default)]
pub struct Ai {
    /// Provider name → adapter.
    registry: HashMap<&'static str, Box<dyn AiProvider>>,
}

impl Ai {
    /// Create an empty facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider adapter under its `provider_name()`.
    pub fn register<P: AiProvider>(&mut self, provider: P) -> &mut Self {
        self.registry
            .insert(provider.provider_name(), Box::new(provider));
        self
    }

    /// Resolve a registered provider into a call builder.
    ///
    /// Chain: `Ai::provider("openai").text("hello").send().await` or
    /// `.embeddings(...).send().await` / `.rerank(...).send().await`.
    pub fn provider(&self, name: &str) -> Result<ProviderCall<'_>> {
        Ok(ProviderCall::new(self.provider_ref(name)?))
    }

    /// Raw accessor for a registered provider (capability inspection etc.).
    pub fn provider_ref(&self, name: &str) -> Result<&dyn AiProvider> {
        self.registry
            .get(name)
            .map(|p| p.as_ref())
            .ok_or_else(|| AiError::UnknownProvider(name.to_string()))
    }

    /// Named constructor used by framework code needing the raw provider.
    pub fn resolve(&self, name: &str) -> Result<&dyn AiProvider> {
        self.provider_ref(name)
    }

    /// Whether a provider supports a capability.
    pub fn supports(&self, name: &str, capability: Capability) -> Result<bool> {
        Ok(self
            .provider_ref(name)?
            .capabilities()
            .contains(&capability))
    }

    /// Number of registered providers.
    pub fn len(&self) -> usize {
        self.registry.len()
    }

    /// Whether no provider is registered.
    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Build an anonymous agent over a provider (Laravel `Ai::agent` parity).
    ///
    /// The closure receives the resolved provider for tool wiring and returns
    /// the configured agent; the agent is pinned to the requested provider.
    pub fn agent(
        &self,
        provider_name: &'static str,
        build: impl FnOnce(&dyn AiProvider) -> Agent + Send + 'static,
    ) -> Result<Agent> {
        let provider = self.provider_ref(provider_name)?;
        let mut agent = build(provider);
        agent.provider = provider_name;
        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::TextRequest;
    use crate::types::TextResponse;

    struct EchoProvider;

    #[async_trait::async_trait]
    impl AiProvider for EchoProvider {
        fn provider_name(&self) -> &'static str {
            "echo"
        }

        fn capabilities(&self) -> Vec<Capability> {
            vec![Capability::Text]
        }

        async fn text(&self, request: &TextRequest) -> Result<TextResponse> {
            Ok(TextResponse {
                text: request.messages.join(" "),
                model: "echo-1".to_string(),
                usage: None,
            })
        }
    }

    #[tokio::test]
    async fn resolve_registered_provider() {
        let mut ai = Ai::new();
        ai.register(EchoProvider);
        let provider = ai.provider_ref("echo").unwrap();
        let response = provider
            .text(&TextRequest::prompt("hello world"))
            .await
            .unwrap();
        assert_eq!(response.text, "hello world");
    }

    #[test]
    fn unknown_provider_errors() {
        let ai = Ai::new();
        assert!(matches!(
            ai.provider_ref("nope"),
            Err(AiError::UnknownProvider(_))
        ));
        assert!(ai.provider("nope").is_err());
    }

    #[test]
    fn agent_builds_anonymous_over_provider() {
        let mut ai = Ai::new();
        ai.register(EchoProvider);
        let agent = ai
            .agent("echo", |_provider| {
                Agent::new("ignored").middleware(NoopMiddleware)
            })
            .unwrap();
        // The closure output is pinned to the requested provider.
        assert_eq!(agent.provider, "echo");
    }

    #[test]
    fn len_tracks_registered_providers() {
        let mut ai = Ai::new();
        assert!(ai.is_empty());
        ai.register(EchoProvider);
        assert_eq!(ai.len(), 1);
    }

    struct NoopMiddleware;

    #[async_trait::async_trait]
    impl crate::agent::AgentMiddleware for NoopMiddleware {
        async fn before_run(&self, _prompt: &str) {}
        async fn after_run(&self, _prompt: &str, _run: &crate::agent::AgentRun) {}
    }
}
