//! Rustavel AI SDK — provider-agnostic AI over 12 providers.
//!
//! Sprint 07 (M6) scope: the `AiProvider` trait + registry, `Agent`/`Tool`
//! contracts, streaming/broadcasting/queueing stubs, sub-agents, middleware,
//! deferred loaders (`SimilaritySearch`/`FileStorage`/`ToolSearch`), MCP
//! discovery (feature-gated), and `make:agent`/`make:tool` scaffolds. The
//! crate is opt-in: core `cargo check` pulls no provider SDKs (NFR-Sca-02).

pub mod adapters;
pub mod agent;
pub mod ai;
pub mod embed;
pub mod error;
pub mod loaders;
pub mod mcp;
pub mod middleware;
pub mod provider;
pub mod scaffold;
pub mod streaming;
pub mod types;

pub use adapters::{
    catalogue, stub_embedding, EmbeddingsCall, FilesCall, InProcessProvider, ProviderCall,
    RerankCall, TextCall, EMBEDDING_DIM, PROVIDERS, PROVIDER_COUNT,
};
pub use agent::{Agent, AgentError, AgentMiddleware, AgentRun, Tool, ToolCall, ToolRegistry};
pub use ai::Ai;
pub use error::{AiError, Result};
pub use loaders::{FileStorage, SimilaritySearch, ToolSearch};
pub use middleware::{LoggingMiddleware, TimingMiddleware};
pub use provider::{AiProvider, Capability, StreamingProvider, TextRequest};
pub use scaffold::{generate_scaffold, ScaffoldFile, ScaffoldKind};
pub use streaming::{
    queue_run, token_channel, DeferredLoader, StreamEvent, StreamReceiver, StreamSender, SubAgent,
};
pub use types::{
    AiChunk, AiResponse, AudioResponse, ContentPart, EmbeddingResponse, ImageResponse,
    RerankResponse, TextResponse, TokenUsage,
};

pub use async_trait::async_trait;
pub use embed::StrToEmbeddings;
