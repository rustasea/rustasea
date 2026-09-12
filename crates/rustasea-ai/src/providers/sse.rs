//! Server-Sent Events parser shared by streaming provider adapters.
//!
//! Turns a `reqwest` byte stream into a stream of `data:` payloads, splitting
//! events on blank lines (`\n\n` / `\r\n\r\n`). The terminator token
//! (`[DONE]` for [OI], `message_stop` for Anthropic) is passed through so each
//! adapter decides where to stop.

use futures_util::StreamExt;

use crate::error::Result;
use crate::providers::client::map_transport;

/// Adapt a streaming response into a stream of SSE `data:` payloads.
pub(crate) fn sse_data_stream(
    response: reqwest::Response,
    provider: &'static str,
) -> futures_core::stream::BoxStream<'static, Result<String>> {
    let bytes = response.bytes_stream();
    let state = (
        bytes,
        Vec::<u8>::new(),
        std::collections::VecDeque::<String>::new(),
        false,
    );
    Box::pin(futures_util::stream::unfold(
        state,
        move |(mut bytes, mut buffer, mut pending, mut eof)| async move {
            loop {
                if let Some(data) = pending.pop_front() {
                    return Some((Ok(data), (bytes, buffer, pending, eof)));
                }
                if eof {
                    return None;
                }
                match bytes.next().await {
                    Some(Ok(chunk)) => {
                        buffer.extend_from_slice(&chunk);
                        drain_events(&mut buffer, &mut pending);
                    }
                    Some(Err(error)) => {
                        eof = true;
                        return Some((
                            Err(map_transport(provider, error)),
                            (bytes, buffer, pending, eof),
                        ));
                    }
                    None => {
                        drain_events(&mut buffer, &mut pending);
                        if !buffer.is_empty() {
                            let tail = String::from_utf8_lossy(&buffer).to_string();
                            buffer.clear();
                            if let Some(data) = event_data(&tail) {
                                pending.push_back(data);
                            }
                        }
                        eof = true;
                    }
                }
            }
        },
    ))
}

/// Drain complete SSE events from `buffer` into `pending` data payloads.
fn drain_events(buffer: &mut Vec<u8>, pending: &mut std::collections::VecDeque<String>) {
    while let Some((index, len)) = find_event_boundary(buffer) {
        let event: Vec<u8> = buffer.drain(..index + len).collect();
        let event = String::from_utf8_lossy(&event[..index]).to_string();
        if let Some(data) = event_data(&event) {
            pending.push_back(data);
        }
    }
}

/// Locate the next blank-line event boundary (`\n\n` or `\r\n\r\n`).
fn find_event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    for (index, byte) in buffer.iter().enumerate() {
        if *byte == b'\n' && buffer.get(index + 1) == Some(&b'\n') {
            return Some((index, 2));
        }
        if *byte == b'\r'
            && buffer.get(index + 1) == Some(&b'\n')
            && buffer.get(index + 2) == Some(&b'\r')
            && buffer.get(index + 3) == Some(&b'\n')
        {
            return Some((index, 4));
        }
    }
    None
}

/// Extract concatenated `data:` field values from one SSE event block.
fn event_data(event: &str) -> Option<String> {
    let mut data = Vec::new();
    for line in event.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(rest) = line.strip_prefix("data:") {
            data.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
        }
    }
    if data.is_empty() {
        None
    } else {
        Some(data.join("\n"))
    }
}
