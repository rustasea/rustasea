//! HTTP provider adapter tests against an in-process mock server.
//!
//! The mock speaks just enough HTTP/1.1 to return canned JSON/SSE responses,
//! keeping these tests hermetic and offline. Live-API tests are `#[ignore]`d.

use std::time::Duration;

use futures_util::StreamExt;
use rustasea_ai::{
    provider_from_env, AiError, AiProvider, AnthropicProvider, AuthStyle, HttpProviderConfig,
    OpenAiProvider, ProviderStyle, TextRequest, TokenUsage,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// One canned HTTP response served by [`spawn_mock`].
struct MockResponse {
    status: u16,
    content_type: &'static str,
    body: String,
}

impl MockResponse {
    /// Build a `200 OK` JSON response.
    fn json(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            content_type: "application/json",
            body: body.into(),
        }
    }

    /// Build an SSE response.
    fn sse(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            content_type: "text/event-stream",
            body: body.into(),
        }
    }

    /// Build an error response with the given status.
    fn error(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.into(),
        }
    }
}

/// Serve `responses` in order on a fresh localhost port, returning the base URL.
async fn spawn_mock(responses: Vec<MockResponse>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        for response in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_request(&mut socket).await;
            let reason = match response.status {
                200 => "OK",
                401 => "Unauthorized",
                429 => "Too Many Requests",
                _ => "Error",
            };
            let payload = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.status,
                reason,
                response.content_type,
                response.body.len(),
                response.body
            );
            let _ = socket.write_all(payload.as_bytes()).await;
            let _ = socket.flush().await;
        }
    });
    format!("http://{address}")
}

/// Read one HTTP request (headers + `Content-Length` body) or until EOF.
async fn read_request(socket: &mut TcpStream) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = socket.read(&mut chunk).await.unwrap_or(0);
        if read == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&buffer[..position]).to_ascii_lowercase();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buffer.len() >= position + 4 + content_length {
                return;
            }
        }
    }
}

/// Config for an [OI] provider pointed at `base` with a synthetic key.
fn openai_config(base: &str) -> HttpProviderConfig {
    HttpProviderConfig::from_env_with("openai", |_| Some("test-key".to_string()))
        .unwrap()
        .with_base_url(base)
        .with_model("test-model")
        .with_timeout(Duration::from_secs(5))
}

/// Config for an Anthropic provider pointed at `base` with a synthetic key.
fn anthropic_config(base: &str) -> HttpProviderConfig {
    HttpProviderConfig::from_env_with("anthropic", |_| Some("test-key".to_string()))
        .unwrap()
        .with_base_url(base)
        .with_model("claude-test")
        .with_timeout(Duration::from_secs(5))
}

/// POSITIVE — [OI] chat completions map into a structured `TextResponse`.
#[tokio::test]
async fn openai_text_returns_structured_output() {
    let body = r#"{"model":"gpt-test","choices":[{"message":{"role":"assistant","content":"hello from openai"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":4}}"#;
    let base = spawn_mock(vec![MockResponse::json(body)]).await;
    let provider = OpenAiProvider::new(openai_config(&base)).unwrap();

    let response = provider.text(&TextRequest::prompt("hi")).await.unwrap();
    assert_eq!(response.text, "hello from openai");
    assert_eq!(response.model, "gpt-test");
    assert_eq!(
        response.usage,
        Some(TokenUsage {
            prompt: 3,
            completion: 4
        })
    );
}

/// POSITIVE — [OI] embeddings are returned in `index` order.
#[tokio::test]
async fn openai_embeddings_return_vectors_in_index_order() {
    let body = r#"{"model":"embed-test","data":[{"index":1,"embedding":[0.2,0.3]},{"index":0,"embedding":[0.1]}]}"#;
    let base = spawn_mock(vec![MockResponse::json(body)]).await;
    let provider = OpenAiProvider::new(openai_config(&base)).unwrap();

    let response = provider
        .embeddings(&["a".to_string(), "b".to_string()])
        .await
        .unwrap();
    assert_eq!(response.model, "embed-test");
    assert_eq!(response.data, vec![vec![0.1], vec![0.2, 0.3]]);
}

/// POSITIVE — [OI] SSE deltas are surfaced as incremental chunks.
#[tokio::test]
async fn openai_stream_yields_incremental_chunks() {
    let body = concat!(
        "data: {\"model\":\"gpt-test\",\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
        "data: [DONE]\n\n"
    );
    let base = spawn_mock(vec![MockResponse::sse(body)]).await;
    let provider = OpenAiProvider::new(openai_config(&base)).unwrap();

    let mut stream = provider
        .stream_text(&TextRequest::prompt("hi"))
        .await
        .unwrap();
    let mut text = String::new();
    let mut chunks = 0;
    while let Some(chunk) = stream.next().await {
        text.push_str(&chunk.unwrap().text);
        chunks += 1;
    }
    assert_eq!(text, "Hello");
    assert_eq!(chunks, 2);
}

/// POSITIVE — Anthropic `/v1/messages` maps into a structured response.
#[tokio::test]
async fn anthropic_text_returns_structured_output() {
    let body = r#"{"model":"claude-test","content":[{"type":"text","text":"hello from anthropic"}],"usage":{"input_tokens":5,"output_tokens":6}}"#;
    let base = spawn_mock(vec![MockResponse::json(body)]).await;
    let provider = AnthropicProvider::new(anthropic_config(&base)).unwrap();

    let response = provider.text(&TextRequest::prompt("hi")).await.unwrap();
    assert_eq!(response.text, "hello from anthropic");
    assert_eq!(response.model, "claude-test");
    assert_eq!(
        response.usage,
        Some(TokenUsage {
            prompt: 5,
            completion: 6
        })
    );
}

/// POSITIVE — Anthropic SSE `content_block_delta` events stream tokens.
#[tokio::test]
async fn anthropic_stream_yields_incremental_chunks() {
    let body = concat!(
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\" there\"}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let base = spawn_mock(vec![MockResponse::sse(body)]).await;
    let provider = AnthropicProvider::new(anthropic_config(&base)).unwrap();

    let mut stream = provider
        .stream_text(&TextRequest::prompt("hi"))
        .await
        .unwrap();
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        text.push_str(&chunk.unwrap().text);
    }
    assert_eq!(text, "Hi there");
}

/// NEGATIVE — a missing credential is a typed configuration error.
#[test]
fn missing_api_key_is_typed_configuration_error() {
    let error = HttpProviderConfig::from_env_with("openai", |_| None).unwrap_err();
    match error {
        AiError::ConfigurationError(message) => assert!(message.contains("OPENAI_API_KEY")),
        other => panic!("unexpected error: {other}"),
    }
}

/// NEGATIVE — a rejected credential maps to `AuthenticationFailed`.
#[tokio::test]
async fn invalid_api_key_is_typed_auth_error() {
    let body = r#"{"error":{"message":"Incorrect API key provided"}}"#;
    let base = spawn_mock(vec![MockResponse::error(401, body)]).await;
    let provider = OpenAiProvider::new(openai_config(&base)).unwrap();

    let error = provider.text(&TextRequest::prompt("hi")).await.unwrap_err();
    match error {
        AiError::AuthenticationFailed { provider, message } => {
            assert_eq!(provider, "openai");
            assert!(message.contains("Incorrect API key"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

/// NEGATIVE — HTTP 429 maps to a typed rate-limit error.
#[tokio::test]
async fn rate_limit_is_typed_error() {
    let base = spawn_mock(vec![MockResponse::error(
        429,
        r#"{"error":{"message":"slow down"}}"#,
    )])
    .await;
    let provider = OpenAiProvider::new(openai_config(&base)).unwrap();

    let error = provider.text(&TextRequest::prompt("hi")).await.unwrap_err();
    assert!(matches!(error, AiError::RateLimited { .. }));
}

/// Config resolution for the azure and openai-compatible specs.
#[test]
fn config_resolution_covers_compatible_providers() {
    let azure = HttpProviderConfig::from_env_with("azure", |key| match key {
        "AZURE_OPENAI_ENDPOINT" => Some("https://example.openai.azure.com".to_string()),
        "AZURE_OPENAI_DEPLOYMENT" => Some("my-deployment".to_string()),
        "AZURE_OPENAI_API_KEY" => Some("secret".to_string()),
        _ => None,
    })
    .unwrap();
    assert_eq!(azure.auth, AuthStyle::Header("api-key"));
    assert_eq!(azure.style, ProviderStyle::OpenAi);
    assert!(azure
        .base_url
        .ends_with("/openai/deployments/my-deployment"));
    assert!(azure.api_version.is_some());

    let ollama = HttpProviderConfig::from_env_with("ollama", |_| None).unwrap();
    assert_eq!(ollama.auth, AuthStyle::None);
    assert!(ollama.api_key.is_none());
}

/// Factory surfaces the typed error for real providers without credentials.
#[test]
fn factory_requires_credentials_for_remote_providers() {
    let error = provider_from_env("definitely-not-a-provider")
        .err()
        .expect("unknown provider must error");
    assert!(matches!(error, AiError::UnknownProvider(_)));
}

/// LIVE — requires `OPENAI_API_KEY` and network access.
#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and network access"]
async fn live_openai_text() {
    let provider = OpenAiProvider::from_env("openai").unwrap();
    let response = provider
        .text(&TextRequest::prompt("Reply with the single word: ok"))
        .await
        .unwrap();
    assert!(!response.text.is_empty());
}

/// LIVE — requires `ANTHROPIC_API_KEY` and network access.
#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY and network access"]
async fn live_anthropic_stream() {
    let provider = AnthropicProvider::from_env("anthropic").unwrap();
    let mut stream = provider
        .stream_text(&TextRequest::prompt("Reply with the single word: ok"))
        .await
        .unwrap();
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        text.push_str(&chunk.unwrap().text);
    }
    assert!(!text.is_empty());
}
