//! [OI]-compatible provider adapter.
//!
//! Speaks `POST {base}/chat/completions` and `POST {base}/embeddings`, the
//! shared OpenAPI surface implemented by `openai`, `azure`, `groq`, `xai`,
//! `deepseek`, `mistral`, `openrouter`, `ollama`, and self-hosted
//! `openai_compatible` endpoints.

use futures_util::StreamExt;
use serde_json::{json, Value};

use crate::error::{AiError, Result};
use crate::provider::{AiProvider, Capability, TextRequest};
use crate::providers::client::{
    authorize, build_client, map_error_response, map_transport, with_api_version,
    HttpProviderConfig,
};
use crate::providers::sse::sse_data_stream;
use crate::types::{AiChunk, EmbeddingResponse, TextResponse, TokenUsage};

/// HTTP adapter for [OI]-compatible chat and embeddings endpoints.
pub struct OpenAiProvider {
    config: HttpProviderConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
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

    /// Build the chat-completions request body.
    fn chat_body(&self, request: &TextRequest, stream: bool) -> Value {
        let model = self.model_for(request);
        let mut messages = Vec::new();
        if let Some(system) = &request.system {
            messages.push(json!({"role": "system", "content": system}));
        }
        for message in &request.messages {
            messages.push(json!({"role": "user", "content": message}));
        }
        json!({
            "model": model,
            "messages": messages,
            "temperature": request.temperature,
            "stream": stream,
        })
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
impl AiProvider for OpenAiProvider {
    fn provider_name(&self) -> &'static str {
        self.config.provider
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Text, Capability::Embeddings]
    }

    async fn text(&self, request: &TextRequest) -> Result<TextResponse> {
        let body = self.chat_body(request, false);
        let request = with_api_version(
            authorize(
                self.client.post(self.endpoint("chat/completions")),
                &self.config,
            ),
            &self.config,
        );
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
        parse_chat_response(self.config.provider, &value)
    }

    async fn stream_text(
        &self,
        request: &TextRequest,
    ) -> Result<futures_core::stream::BoxStream<'static, Result<AiChunk>>> {
        let model = self.model_for(request);
        let body = self.chat_body(request, true);
        let request = with_api_version(
            authorize(
                self.client.post(self.endpoint("chat/completions")),
                &self.config,
            ),
            &self.config,
        );
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
                futures_util::future::ready(!matches!(item, Ok(data) if data.trim() == "[DONE]"))
            })
            .filter_map(move |item| {
                let model = model.clone();
                futures_util::future::ready(match item {
                    Err(error) => Some(Err(error)),
                    Ok(data) => match parse_chat_delta(provider, &data) {
                        Ok(Some(text)) if !text.is_empty() => Some(Ok(AiChunk::token(model, text))),
                        Ok(_) => None,
                        Err(error) => Some(Err(error)),
                    },
                })
            });
        Ok(Box::pin(stream))
    }

    async fn embeddings(&self, inputs: &[String]) -> Result<EmbeddingResponse> {
        let body = json!({
            "model": self.config.default_model,
            "input": inputs,
        });
        let request = with_api_version(
            authorize(self.client.post(self.endpoint("embeddings")), &self.config),
            &self.config,
        );
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
        parse_embeddings_response(self.config.provider, &value)
    }
}

/// Parse an [OI] chat-completions response into the uniform text response.
fn parse_chat_response(provider: &str, value: &Value) -> Result<TextResponse> {
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
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

/// Parse the [OI] `usage` object into token counts.
fn parse_usage(value: &Value) -> Option<TokenUsage> {
    let prompt = value
        .pointer("/usage/prompt_tokens")
        .and_then(Value::as_u64);
    let completion = value
        .pointer("/usage/completion_tokens")
        .and_then(Value::as_u64);
    match (prompt, completion) {
        (None, None) => None,
        (prompt, completion) => Some(TokenUsage {
            prompt: prompt.unwrap_or(0) as u32,
            completion: completion.unwrap_or(0) as u32,
        }),
    }
}

/// Extract incremental text from one [OI] streaming `data:` payload.
fn parse_chat_delta(provider: &str, data: &str) -> Result<Option<String>> {
    let value: Value = serde_json::from_str(data).map_err(|error| AiError::Provider {
        provider: provider.to_string(),
        message: format!("invalid streaming JSON: {error}"),
    })?;
    Ok(value
        .pointer("/choices/0/delta/content")
        .and_then(Value::as_str)
        .map(str::to_string))
}

/// Parse an [OI] embeddings response, ordering vectors by `index`.
fn parse_embeddings_response(provider: &str, value: &Value) -> Result<EmbeddingResponse> {
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(provider)
        .to_string();
    let mut entries: Vec<(u64, Vec<f32>)> = Vec::new();
    if let Some(data) = value.get("data").and_then(Value::as_array) {
        for item in data {
            let index = item
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(entries.len() as u64);
            let vector = item
                .get("embedding")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_f64().map(|v| v as f32))
                        .collect()
                })
                .unwrap_or_default();
            entries.push((index, vector));
        }
    }
    entries.sort_by_key(|(index, _)| *index);
    Ok(EmbeddingResponse {
        data: entries.into_iter().map(|(_, vector)| vector).collect(),
        model,
    })
}
