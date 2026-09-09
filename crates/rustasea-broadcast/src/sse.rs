//! Server-sent events: `event_stream` SSE with bounded buffering and Axum
//! `Response::event_stream` wiring.

use std::pin::Pin;
use std::task::{Context, Poll};

use axum::response::sse::{Event as AxumSseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_core::Stream;
use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::error::{BroadcastError, Result};

/// Capacity of the bounded SSE channel (backpressure cap).
pub const SSE_BUFFER: usize = 64;

/// Build a `Response::event_stream`-compatible SSE channel.
///
/// Returns the caller side (`EventSender`) plus the stream that Axum's
/// [`axum::response::sse::Sse`] can wrap. Contract (FS-M6-01): the response
/// must carry `Content-Type: text/event-stream` and chunks must stream in
/// order; a dropped receiver surfaces as a closed stream, not a silent
/// success.
pub fn event_stream() -> (EventSender, EventStream) {
    let (tx, rx) = mpsc::channel::<SseEvent>(SSE_BUFFER);
    (EventSender { tx }, EventStream::new(rx))
}

/// Render any ordered stream of strings as an SSE HTTP response.
///
/// This is the `Response::eventStream(stream)` contract (FS-M6-01): the
/// response carries `Content-Type: text/event-stream` and each input item is
/// streamed as one `data:` frame. Frames are emitted in input order; a stream
/// error terminates the response.
pub fn event_stream_response<S, E>(stream: S) -> Response
where
    S: Stream<Item = std::result::Result<SseEvent, E>> + Send + 'static,
    E: std::error::Error + Send + 'static,
{
    let sse = Sse::new(stream.map(|item| match item {
        Ok(frame) => {
            let mut event = AxumSseEvent::default().data(frame.data);
            if !frame.event.is_empty() {
                event = event.event(frame.event);
            }
            Ok(event)
        }
        Err(err) => Err(err.to_string()),
    }))
    .keep_alive(KeepAlive::default());
    sse.into_response()
}

impl EventSender {
    /// Send one frame; errors when the subscriber disconnected (Lagged-safe).
    pub async fn send(&self, event: SseEvent) -> Result<()> {
        self.tx
            .send(event)
            .await
            .map_err(|_| BroadcastError::Transport("sse receiver dropped".to_string()))
    }
}

/// One SSE frame (Laravel `event:` framing parity).
#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    /// Event name (`token`, `message`, …).
    pub event: String,
    /// Serialized event payload.
    pub data: String,
}

impl SseEvent {
    /// Create an SSE frame.
    pub fn new(event: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            event: event.into(),
            data: data.into(),
        }
    }

    /// Render the wire format (`event: X\ndata: Y\n\n`).
    pub fn to_wire(&self) -> String {
        format!("event: {}\ndata: {}\n\n", self.event, self.data)
    }
}

/// Sender half of the bounded SSE channel.
#[derive(Debug, Clone)]
pub struct EventSender {
    tx: mpsc::Sender<SseEvent>,
}

/// Stream half of the bounded SSE channel.
#[derive(Debug)]
pub struct EventStream {
    rx: mpsc::Receiver<SseEvent>,
    done: bool,
}

impl EventStream {
    fn new(rx: mpsc::Receiver<SseEvent>) -> Self {
        Self { rx, done: false }
    }
}

impl Stream for EventStream {
    type Item = Result<SseEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(event)) => Poll::Ready(Some(Ok(event))),
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn sse_frame_wire_format() {
        let frame = SseEvent::new("token", "{\"t\":\"hi\"}");
        assert_eq!(frame.to_wire(), "event: token\ndata: {\"t\":\"hi\"}\n\n");
    }

    #[tokio::test]
    async fn stream_yields_frames_in_order() {
        let (tx, mut stream) = event_stream();
        tx.send(SseEvent::new("a", "1")).await.unwrap();
        tx.send(SseEvent::new("b", "2")).await.unwrap();
        drop(tx);

        let mut out = Vec::new();
        while let Some(Ok(frame)) = stream.next().await {
            out.push(frame.event);
        }
        assert_eq!(out, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn event_stream_response_sets_content_type() {
        use futures_core::Stream;
        let stream = futures_util::stream::iter(std::iter::empty::<
            std::result::Result<SseEvent, std::io::Error>,
        >());
        let response = event_stream_response(stream);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .map(|v| v.to_str().unwrap_or_default()),
            Some("text/event-stream")
        );
    }
}
