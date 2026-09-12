//! Shared HTTP plumbing for the real provider adapters.
//!
//! Owns endpoint/key/model resolution from the environment, credential
//! placement, typed error mapping, and the SSE `data:` parser used by streaming
//! adapters. Provider-specific request/response shaping lives in the sibling
//! `openai` and `anthropic` modules.

use std::time::Duration;

use crate::error::{AiError, Result};

/// Request timeout applied when a provider does not override it.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Wire style a provider speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStyle {
    /// [OI]-compatible `/v1/chat/completions` + `/v1/embeddings`.
    OpenAi,
    /// Anthropic `/v1/messages`.
    Anthropic,
}

/// How the API key is attached to outgoing requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStyle {
    /// `Authorization: Bearer <key>`.
    Bearer,
    /// `x-api-key: <key>` (Anthropic).
    XApiKey,
    /// Custom header carrying the key (Azure `api-key`).
    Header(&'static str),
    /// No credential (local `ollama`).
    None,
}

/// Resolved connection settings for one provider adapter.
#[derive(Debug, Clone)]
pub struct HttpProviderConfig {
    /// Provider name used in errors.
    pub provider: &'static str,
    /// Base URL without a trailing slash (`https://api.openai.com/v1`).
    pub base_url: String,
    /// API key when the provider requires or accepts one.
    pub api_key: Option<String>,
    /// Model used when a request does not override it.
    pub default_model: String,
    /// Wire style spoken by the provider.
    pub style: ProviderStyle,
    /// Credential placement.
    pub auth: AuthStyle,
    /// Optional `api-version` query parameter (Azure).
    pub api_version: Option<String>,
    /// Per-request timeout.
    pub timeout: Duration,
    /// Anthropic `max_tokens` (ignored by [OI] style).
    pub max_tokens: u32,
}

/// Static environment mapping for one provider.
struct EnvSpec {
    /// Env var holding the API key (`None` = unauthenticated provider).
    key_env: Option<&'static str>,
    /// Whether a missing key is a hard configuration error.
    key_required: bool,
    /// Env var overriding the base URL.
    base_env: Option<&'static str>,
    /// Base URL used when the override is unset.
    default_base: &'static str,
    /// Env var overriding the default model.
    model_env: &'static str,
    /// Model used when the override is unset.
    default_model: &'static str,
    /// Credential placement.
    auth: AuthStyle,
    /// Wire style spoken by the provider.
    style: ProviderStyle,
}

impl HttpProviderConfig {
    /// Resolve provider settings from the process environment.
    pub fn from_env(provider: &str) -> Result<Self> {
        Self::from_env_with(provider, |key| std::env::var(key).ok())
    }

    /// Resolve provider settings from a caller-supplied environment lookup.
    ///
    /// `lookup` returns `None` for an unset variable; separating it from
    /// `std::env` keeps credential tests hermetic and side-effect free.
    pub fn from_env_with<F>(provider: &str, lookup: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let lookup = |key: &str| lookup(key).filter(|value| !value.trim().is_empty());
        match provider {
            "openai" => resolve_env(
                "openai",
                EnvSpec {
                    key_env: Some("OPENAI_API_KEY"),
                    key_required: true,
                    base_env: Some("OPENAI_BASE_URL"),
                    default_base: "https://api.openai.com/v1",
                    model_env: "OPENAI_MODEL",
                    default_model: "gpt-4o-mini",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "anthropic" => resolve_env(
                "anthropic",
                EnvSpec {
                    key_env: Some("ANTHROPIC_API_KEY"),
                    key_required: true,
                    base_env: Some("ANTHROPIC_BASE_URL"),
                    default_base: "https://api.anthropic.com/v1",
                    model_env: "ANTHROPIC_MODEL",
                    default_model: "claude-3-5-sonnet-latest",
                    auth: AuthStyle::XApiKey,
                    style: ProviderStyle::Anthropic,
                },
                &lookup,
            ),
            "groq" => resolve_env(
                "groq",
                EnvSpec {
                    key_env: Some("GROQ_API_KEY"),
                    key_required: true,
                    base_env: Some("GROQ_BASE_URL"),
                    default_base: "https://api.groq.com/openai/v1",
                    model_env: "GROQ_MODEL",
                    default_model: "llama-3.3-70b-versatile",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "xai" => resolve_env(
                "xai",
                EnvSpec {
                    key_env: Some("XAI_API_KEY"),
                    key_required: true,
                    base_env: Some("XAI_BASE_URL"),
                    default_base: "https://api.x.ai/v1",
                    model_env: "XAI_MODEL",
                    default_model: "grok-2-latest",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "deepseek" => resolve_env(
                "deepseek",
                EnvSpec {
                    key_env: Some("DEEPSEEK_API_KEY"),
                    key_required: true,
                    base_env: Some("DEEPSEEK_BASE_URL"),
                    default_base: "https://api.deepseek.com/v1",
                    model_env: "DEEPSEEK_MODEL",
                    default_model: "deepseek-chat",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "mistral" => resolve_env(
                "mistral",
                EnvSpec {
                    key_env: Some("MISTRAL_API_KEY"),
                    key_required: true,
                    base_env: Some("MISTRAL_BASE_URL"),
                    default_base: "https://api.mistral.ai/v1",
                    model_env: "MISTRAL_MODEL",
                    default_model: "mistral-large-latest",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "openrouter" => resolve_env(
                "openrouter",
                EnvSpec {
                    key_env: Some("OPENROUTER_API_KEY"),
                    key_required: true,
                    base_env: Some("OPENROUTER_BASE_URL"),
                    default_base: "https://openrouter.ai/api/v1",
                    model_env: "OPENROUTER_MODEL",
                    default_model: "openai/gpt-4o-mini",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "openai_compatible" => resolve_env(
                "openai_compatible",
                EnvSpec {
                    key_env: Some("OPENAI_COMPATIBLE_API_KEY"),
                    key_required: false,
                    base_env: Some("OPENAI_COMPATIBLE_BASE_URL"),
                    default_base: "http://localhost:8000/v1",
                    model_env: "OPENAI_COMPATIBLE_MODEL",
                    default_model: "default",
                    auth: AuthStyle::Bearer,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "ollama" => resolve_env(
                "ollama",
                EnvSpec {
                    key_env: None,
                    key_required: false,
                    base_env: Some("OLLAMA_BASE_URL"),
                    default_base: "http://localhost:11434/v1",
                    model_env: "OLLAMA_MODEL",
                    default_model: "llama3.2",
                    auth: AuthStyle::None,
                    style: ProviderStyle::OpenAi,
                },
                &lookup,
            ),
            "azure" => azure_from_env(&lookup),
            other => Err(AiError::UnknownProvider(other.to_string())),
        }
    }

    /// Override the base URL (endpoint root without trailing slash).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the API key.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Override the default model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Override the per-request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the Anthropic `max_tokens` budget.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// Resolve an env-backed provider using a shared [`EnvSpec`].
fn resolve_env<F>(provider: &'static str, spec: EnvSpec, lookup: &F) -> Result<HttpProviderConfig>
where
    F: Fn(&str) -> Option<String>,
{
    let api_key = match spec.key_env {
        Some(env) => {
            let key = lookup(env);
            if key.is_none() && spec.key_required {
                return Err(AiError::configuration(format!(
                    "{env} is not set (required for provider {provider})"
                )));
            }
            key
        }
        None => None,
    };
    let base_url = spec
        .base_env
        .and_then(lookup)
        .unwrap_or_else(|| spec.default_base.to_string());
    let default_model = lookup(spec.model_env).unwrap_or_else(|| spec.default_model.to_string());
    Ok(HttpProviderConfig {
        provider,
        base_url: base_url.trim_end_matches('/').to_string(),
        api_key,
        default_model,
        style: spec.style,
        auth: spec.auth,
        api_version: None,
        timeout: DEFAULT_TIMEOUT,
        max_tokens: 1024,
    })
}

/// Resolve the Azure [OI] endpoint (deployment-scoped, `api-key` header).
fn azure_from_env<F>(lookup: &F) -> Result<HttpProviderConfig>
where
    F: Fn(&str) -> Option<String>,
{
    let missing = |env: &str| {
        AiError::configuration(format!("{env} is not set (required for provider azure)"))
    };
    let endpoint =
        lookup("AZURE_OPENAI_ENDPOINT").ok_or_else(|| missing("AZURE_OPENAI_ENDPOINT"))?;
    let deployment =
        lookup("AZURE_OPENAI_DEPLOYMENT").ok_or_else(|| missing("AZURE_OPENAI_DEPLOYMENT"))?;
    let api_key = lookup("AZURE_OPENAI_API_KEY").ok_or_else(|| missing("AZURE_OPENAI_API_KEY"))?;
    Ok(HttpProviderConfig {
        provider: "azure",
        base_url: format!(
            "{}/openai/deployments/{}",
            endpoint.trim_end_matches('/'),
            deployment
        ),
        api_key: Some(api_key),
        default_model: deployment,
        style: ProviderStyle::OpenAi,
        auth: AuthStyle::Header("api-key"),
        api_version: Some(
            lookup("AZURE_OPENAI_API_VERSION").unwrap_or_else(|| "2024-10-21".to_string()),
        ),
        timeout: DEFAULT_TIMEOUT,
        max_tokens: 1024,
    })
}

/// Build the shared reqwest client with the configured timeout.
pub(crate) fn build_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| AiError::configuration(format!("failed to build HTTP client: {error}")))
}

/// Apply the configured credential header to a request.
pub(crate) fn authorize(
    request: reqwest::RequestBuilder,
    config: &HttpProviderConfig,
) -> reqwest::RequestBuilder {
    match (config.auth, config.api_key.as_deref()) {
        (AuthStyle::Bearer, Some(key)) => request.bearer_auth(key),
        (AuthStyle::XApiKey, Some(key)) => request.header("x-api-key", key),
        (AuthStyle::Header(name), Some(key)) => request.header(name, key),
        _ => request,
    }
}

/// Append the configured `api-version` query parameter (Azure).
pub(crate) fn with_api_version(
    request: reqwest::RequestBuilder,
    config: &HttpProviderConfig,
) -> reqwest::RequestBuilder {
    match &config.api_version {
        Some(version) => request.query(&[("api-version", version)]),
        None => request,
    }
}

/// Classify a reqwest transport failure into a typed error.
pub(crate) fn map_transport(provider: &str, error: reqwest::Error) -> AiError {
    let message = if error.is_timeout() {
        format!("request timed out: {error}")
    } else if error.is_connect() {
        format!("connection failed: {error}")
    } else {
        error.to_string()
    };
    AiError::transport(provider, message)
}

/// Classify a non-success HTTP response into a typed error.
pub(crate) async fn map_error_response(provider: &str, response: reqwest::Response) -> AiError {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok());
    let body = response.text().await.unwrap_or_default();
    let message = extract_error_message(&body);
    match status.as_u16() {
        401 | 403 => AiError::authentication_failed(provider, message),
        429 => AiError::RateLimited {
            provider: provider.to_string(),
            retry_after,
        },
        _ => AiError::Provider {
            provider: provider.to_string(),
            message: format!("HTTP {status}: {message}"),
        },
    }
}

/// Extract a human-readable message from a provider error body.
fn extract_error_message(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        for pointer in ["/error/message", "/error", "/message"] {
            if let Some(message) = value.pointer(pointer).and_then(|v| v.as_str()) {
                return message.to_string();
            }
        }
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        "empty error body".to_string()
    } else {
        trimmed.chars().take(512).collect()
    }
}
