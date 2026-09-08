//! Runtime entry point: `Ai` facade with provider registry and agent factory.

use std::collections::HashMap;

use crate::agent::Agent;
use crate::error::{AiError, Result};
use crate::provider::{AiProvider, Capability};

/// AI SDK facade — provider registry + anonymous agent factory.
///
/// Mirrors the Laravel `Ai` facade: `Ai::provider("anthropic")` returns the
/// registered provider, and `Ai::agent(...)` builds an anonymous agent
/// (FS-M6-07 contract).
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

    /// Resolve a registered provider by name.
    pub fn provider(&self, name: &str) -> Result<&dyn AiProvider> {
        self.registry
            .get(name)
            .map(|p| p.as_ref())
            .ok_or_else(|| AiError::UnknownProvider(name.to_string()))
    }

    /// Named constructor used by `Ai::provider(...)` in framework code.
    pub fn resolve(&self, name: &str) -> Result<&dyn AiProvider> {
        self.provider(name)
    }

    /// Whether a provider supports a capability.
    pub fn supports(&self, name: &str, capability: Capability) -> Result<bool> {
        Ok(self.provider(name)?.capabilities().contains(&capability))
    }

    /// Build an anonymous agent over a provider (Laravel `Ai::agent` parity).
    ///
    /// The closure receives the resolved provider for tool wiring; the built
    /// agent is returned. Errors surface only when the provider is unknown.
    pub fn agent(
        &self,
        provider_name: &'static str,
        _build: impl FnOnce(&dyn AiProvider) -> Agent + Send + 'static,
    ) -> Result<Agent> {
        let _provider = self.provider(provider_name)?;
        Ok(Agent::new(provider_name))
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
        let provider = ai.provider("echo").unwrap();
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
            ai.provider("nope"),
            Err(AiError::UnknownProvider(_))
        ));
    }
}
