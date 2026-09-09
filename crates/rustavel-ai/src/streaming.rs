//! Streaming, broadcasting, and queueing stubs for agent output.
//!
//! Real transport wiring (WebSocket fan-out, deadpool-redis queues) is M6
//! integration; these stubs keep the streaming contract compilable and
//! testable: ordered `event: token` chunks over a bounded broadcast channel,
//! a queueing stub, and a deferred tool loader.

use tokio::sync::broadcast;

use crate::error::Result;
use crate::types::AiChunk;

/// Capacity of the token broadcast channel.
pub const STREAM_BUFFER: usize = 256;

/// Build a single-chunk stream from a completed text response.
pub fn single_chunk_stream(
    model: String,
    text: String,
) -> impl futures_core::Stream<Item = Result<AiChunk>> {
    crate::streaming::single::SingleChunk {
        model,
        text,
        done: false,
    }
}

/// Internal single-chunk stream impl (avoid a `futures-util` dependency).
pub(crate) mod single {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use super::AiChunk;

    /// Emits exactly one token chunk, then ends.
    pub struct SingleChunk {
        /// Provider/model name.
        pub model: String,
        /// Chunk text.
        pub text: String,
        /// Whether the chunk was already emitted.
        pub done: bool,
    }

    impl futures_core::Stream for SingleChunk {
        type Item = crate::error::Result<AiChunk>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.done {
                return Poll::Ready(None);
            }
            self.done = true;
            Poll::Ready(Some(Ok(AiChunk::token(&self.model, &self.text))))
        }
    }
}

/// One streamed token event ready for WS/SSE framing.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct StreamEvent {
    /// `event: token` framing name.
    pub event: String,
    /// Token chunk payload.
    pub chunk: AiChunk,
}

/// Sub-agent descriptor for hierarchical agent runs.
#[derive(Debug, Clone)]
pub struct SubAgent {
    /// Agent name.
    pub name: String,
    /// Task handed to the sub-agent.
    pub task: String,
}

impl SubAgent {
    /// Create a sub-agent assignment.
    pub fn new(name: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            task: task.into(),
        }
    }
}

/// Queue an agent run for background execution (queueing stub).
///
/// Returns the assigned job identifier; actual dispatch hooks into the M4
/// queue driver in M6 integration.
pub async fn queue_run(_agent: &str, _prompt: &str) -> Result<String> {
    Ok(format!("queued-{}", uuid_v4_short()))
}

/// Short random id used by the queueing stub.
fn uuid_v4_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{nanos:x}{}", std::process::id())
}

/// Broadcast channel factory for ordered token chunks.
///
/// Subscribers receive [`StreamEvent`]s in emission order; a slow consumer
/// observing a full buffer is flagged `Lagged` (close frame semantics live in
/// the transport layer).
pub fn token_channel() -> (StreamSender, StreamReceiver) {
    let (tx, _rx) = broadcast::channel(STREAM_BUFFER);
    let rx = tx.subscribe();
    (StreamSender { tx }, StreamReceiver { rx })
}

/// Sender half of the token broadcast.
#[derive(Clone)]
pub struct StreamSender {
    tx: broadcast::Sender<StreamEvent>,
}

impl StreamSender {
    /// Publish one chunk as an ordered `token` event.
    pub fn send(&self, chunk: AiChunk) {
        let _ = self.tx.send(StreamEvent {
            event: "token".to_string(),
            chunk,
        });
    }
}

/// Receiver half of the token broadcast.
pub struct StreamReceiver {
    rx: broadcast::Receiver<StreamEvent>,
}

impl StreamReceiver {
    /// Receive the next token event.
    pub async fn recv(&mut self) -> Result<StreamEvent> {
        self.rx
            .recv()
            .await
            .map_err(|e| crate::error::AiError::Provider {
                provider: "stream".to_string(),
                message: format!("broadcast lagged/closed: {e}"),
            })
    }
}

/// Deferred loader contract for lazily-bound agents (FS-M6-07).
///
/// Loaders resolve heavyweight integrations (`SimilaritySearch`,
/// `FileStorage`, `ToolSearch`) only when first used; constructing the loader
/// never performs I/O.
pub trait DeferredLoader {
    /// Loaded resource type (may be a `dyn` trait object).
    type Target: Send + Sync + ?Sized;

    /// Load the target, caching the result for later calls.
    fn load(&self) -> Result<std::sync::Arc<Self::Target>>;

    /// Whether the target has been loaded already.
    fn loaded(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ordered_token_events_flow() {
        let (tx, mut rx) = token_channel();
        tx.send(AiChunk::token("m", "a"));
        tx.send(AiChunk::token("m", "b"));
        let first = rx.recv().await.unwrap();
        assert_eq!(first.event, "token");
        assert_eq!(first.chunk.text, "a");
    }

    #[tokio::test]
    async fn queue_stub_returns_id() {
        let id = queue_run("agent", "hello").await.unwrap();
        assert!(id.starts_with("queued-"));
    }
}
