//! Anthropic provider adapter (`/v1/messages`).
//!
//! Anthropic's wire format differs from [OI]: a top-level `system` field, a
//! `content` block array in the response, and `x-api-key` authentication.

use futures_util::StreamExt;
use serde_json::{json, Value};

use crate::error::{AiError, Result};
use crate::provider::{AiProvider, Capability, TextRequest};
use crate::providers::client::{
    authorize, build_client, map_error_response, map_transport, HttpProviderConfig,
};
use crate::providers::sse::sse_data_stream;
use crate::types::{AiChunk, TextResponse, TokenUsage};

/// HTTP adapter for the Anthropic Messages API.
pub struct AnthropicProvider {
    config: HttpProviderConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Build an adapter from resolved connection settings.
    pub fn new(config: HttpProviderConfig) -> Result<Self> {
        let client = build_client(config.timeout)?;
        Ok(Self { config, client })
    }

    /// Build an adapter by resolving credentials from the environment.
    pub fn from_env(provider: &str) -> Result<Self> {
        Self::new(HttpProviderConfig::from_env(provider)?)
    }

    /// Resolved connection settings (diagnostics/tests).
    pub fn config(&self) -> &HttpProviderConfig {
        &self.config
    }

    /// Compose the absolute endpoint URL for `path`.
    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.config.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Build an authorized POST with the pinned Anthropic API version.
    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        authorize(self.client.post(self.endpoint(path)), &self.config)
            .header("anthropic-version", "2023-06-01")
    }

    /// Build the `/v1/messages` request body.
    fn messages_body(&self, request: &TextRequest, stream: bool) -> Value {
        let model = self.model_for(request);
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|message| json!({"role": "user", "content": message}))
            .collect();
        let mut body = json!({
            "model": model,
            "max_tokens": self.config.max_tokens,
            "messages": messages,
            "temperature": request.temperature,
            "stream": stream,
        });
        if let Some(system) = &request.system {
            body["system"] = json!(system);
        }
        body
    }

    /// Model to request, falling back to the configured default.
    fn model_for(&self, request: &TextRequest) -> String {
        request
            .model
            .clone()
            .unwrap_or_else(|| self.config.default_model.clone())
    }
}

#[async_trait::async_trait]
impl AiProvider for AnthropicProvider {
    fn provider_name(&self) -> &'static str {
        self.config.provider
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Text]
    }

    async fn text(&self, request: &TextRequest) -> Result<TextResponse> {
        let body = self.messages_body(request, false);
        let request = self.post("messages");
        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|error| map_transport(self.config.provider, error))?;
        if !response.status().is_success() {
            return Err(map_error_response(self.config.provider, response).await);
        }
        let value: Value = response
            .json()
            .await
            .map_err(|error| map_transport(self.config.provider, error))?;
        parse_messages_response(self.config.provider, &value)
    }

    async fn stream_text(
        &self,
        request: &TextRequest,
    ) -> Result<futures_core::stream::BoxStream<'static, Result<AiChunk>>> {
        let model = self.model_for(request);
        let body = self.messages_body(request, true);
        let request = self.post("messages");
        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|error| map_transport(self.config.provider, error))?;
        if !response.status().is_success() {
            return Err(map_error_response(self.config.provider, response).await);
        }
        let provider = self.config.provider;
        let stream = sse_data_stream(response, provider)
            .take_while(|item| {
                futures_util::future::ready(
                    !matches!(item, Ok(data) if data.contains("message_stop")),
                )
            })
            .filter_map(move |item| {
                let model = model.clone();
                futures_util::future::ready(match item {
                    Err(error) => Some(Err(error)),
                    Ok(data) => match parse_anthropic_delta(provider, &data) {
                        Ok(Some(text)) if !text.is_empty() => Some(Ok(AiChunk::token(model, text))),
                        Ok(_) => None,
                        Err(error) => Some(Err(error)),
                    },
                })
            });
        Ok(Box::pin(stream))
    }
}

/// Parse a `/v1/messages` response into the uniform text response.
fn parse_messages_response(provider: &str, value: &Value) -> Result<TextResponse> {
    let text = value
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(provider)
        .to_string();
    Ok(TextResponse {
        text,
        model,
        usage: parse_usage(value),
    })
}

/// Parse Anthropic `usage` (`input_tokens`/`output_tokens`) into token counts.
fn parse_usage(value: &Value) -> Option<TokenUsage> {
    let prompt = value.pointer("/usage/input_tokens").and_then(Value::as_u64);
    let completion = value
        .pointer("/usage/output_tokens")
        .and_then(Value::as_u64);
    match (prompt, completion) {
        (None, None) => None,
        (prompt, completion) => Some(TokenUsage {
            prompt: prompt.unwrap_or(0) as u32,
            completion: completion.unwrap_or(0) as u32,
        }),
    }
}

/// Extract incremental text from one Anthropic streaming `data:` payload.
fn parse_anthropic_delta(provider: &str, data: &str) -> Result<Option<String>> {
    let value: Value = serde_json::from_str(data).map_err(|error| AiError::Provider {
        provider: provider.to_string(),
        message: format!("invalid streaming JSON: {error}"),
    })?;
    if value.get("type").and_then(Value::as_str) != Some("content_block_delta") {
        return Ok(None);
    }
    Ok(value
        .pointer("/delta/text")
        .and_then(Value::as_str)
        .map(str::to_string))
}
