//! Real HTTP-backed provider adapters.
//!
//! Replaces the deterministic [`crate::adapters::InProcessProvider`] stubs for
//! providers that expose an [OI]-compatible or Anthropic HTTP surface. The
//! in-process provider stays available as the deterministic test/fallback
//! backend; these adapters perform real network I/O and require credentials.

mod anthropic;
mod client;
mod openai;
mod sse;

pub use anthropic::AnthropicProvider;
pub use client::{AuthStyle, HttpProviderConfig, ProviderStyle, DEFAULT_TIMEOUT};
pub use openai::OpenAiProvider;

use crate::error::Result;
use crate::provider::AiProvider;

/// Build a real HTTP provider adapter from environment credentials.
///
/// Returns [`crate::error::AiError::ConfigurationError`] when a required
/// credential is missing (never a silent stub) and
/// [`crate::error::AiError::UnknownProvider`] for names without an HTTP
/// adapter (`gemini`, `bedrock`). Callers wanting deterministic offline
/// behaviour should construct [`crate::adapters::InProcessProvider`] instead.
pub fn provider_from_env(name: &str) -> Result<Box<dyn AiProvider>> {
    let config = HttpProviderConfig::from_env(name)?;
    match config.style {
        ProviderStyle::OpenAi => Ok(Box::new(OpenAiProvider::new(config)?)),
        ProviderStyle::Anthropic => Ok(Box::new(AnthropicProvider::new(config)?)),
    }
}
