//! MCP client transport — JSON-RPC 2.0 over stdio and streamable HTTP.
//!
//! Implements the MCP handshake and the two core methods the SDK needs:
//! `tools/list` (discovery) and `tools/call` (invocation). stdio frames are
//! newline-delimited JSON-RPC objects; HTTP responses are accepted as either a
//! single JSON body or an SSE stream. Transport failures map to
//! [`crate::error::AiError::McpServerUnreachable`] and malformed frames to
//! [`crate::error::AiError::McpProtocol`].

use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::error::{AiError, Result};
use crate::mcp::{McpServerConfig, McpTool};

/// MCP protocol revision this client advertises during `initialize`.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Default timeout applied to a single MCP round-trip.
pub const DEFAULT_MCP_TIMEOUT: Duration = Duration::from_secs(30);

/// Which transport a connected client speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTransport {
    /// Newline-delimited JSON-RPC over a child process's stdio.
    Stdio,
    /// Streamable HTTP JSON-RPC (JSON or SSE responses).
    Http,
}

/// A connected MCP server client.
///
/// Construct one with [`McpClient::connect`], which performs the JSON-RPC
/// handshake before returning; then call [`McpClient::list_tools`] and
/// [`McpClient::call_tool`].
pub struct McpClient {
    /// Server name used in error messages.
    server: String,
    /// Active transport.
    transport: McpTransport,
    /// Round-trip timeout.
    timeout: Duration,
    /// Next JSON-RPC request id.
    next_id: u64,
    /// stdio connection state (present for [`McpTransport::Stdio`]).
    stdio: Option<StdioConnection>,
    /// HTTP connection state (present for [`McpTransport::Http`]).
    http: Option<HttpConnection>,
}

/// Live stdio child-process connection.
struct StdioConnection {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Live HTTP connection.
struct HttpConnection {
    client: reqwest::Client,
    url: String,
}

impl Drop for StdioConnection {
    /// Terminate the child process so a dropped client leaves no zombie.
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

impl McpClient {
    /// Connect to `config` and complete the MCP handshake.
    ///
    /// Spawns a stdio child (or dials the HTTP endpoint), sends `initialize`,
    /// and follows with the `notifications/initialized` notification. Returns
    /// [`AiError::McpServerUnreachable`] when the server cannot be reached.
    pub async fn connect(config: &McpServerConfig) -> Result<Self> {
        if config.is_http() {
            Self::connect_http(config).await
        } else {
            Self::connect_stdio(config).await
        }
    }

    /// Override the per-round-trip timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Transport this client is using.
    pub fn transport(&self) -> McpTransport {
        self.transport
    }

    /// Spawn a stdio MCP server and complete the handshake.
    async fn connect_stdio(config: &McpServerConfig) -> Result<Self> {
        let mut child = tokio::process::Command::new(&config.command)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                AiError::mcp_server_unreachable(
                    &config.name,
                    format!("failed to spawn '{}': {error}", config.command),
                )
            })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            AiError::mcp_server_unreachable(&config.name, "child stdin unavailable")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AiError::mcp_server_unreachable(&config.name, "child stdout unavailable")
        })?;
        let mut client = Self {
            server: config.name.clone(),
            transport: McpTransport::Stdio,
            timeout: DEFAULT_MCP_TIMEOUT,
            next_id: 1,
            stdio: Some(StdioConnection {
                child,
                stdin,
                stdout: BufReader::new(stdout),
            }),
            http: None,
        };
        client.handshake().await?;
        Ok(client)
    }

    /// Build an HTTP client and complete the handshake.
    async fn connect_http(config: &McpServerConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_MCP_TIMEOUT)
            .build()
            .map_err(|error| {
                AiError::mcp_server_unreachable(&config.name, format!("HTTP client: {error}"))
            })?;
        let mut client = Self {
            server: config.name.clone(),
            transport: McpTransport::Http,
            timeout: DEFAULT_MCP_TIMEOUT,
            next_id: 1,
            stdio: None,
            http: Some(HttpConnection {
                client: http,
                url: config.command.clone(),
            }),
        };
        client.handshake().await?;
        Ok(client)
    }

    /// Perform `initialize` then send the `initialized` notification.
    async fn handshake(&mut self) -> Result<()> {
        let params = json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "rustasea-ai", "version": env!("CARGO_PKG_VERSION") }
        });
        let _ = self.request("initialize", params).await?;
        self.notify("notifications/initialized", json!({})).await
    }

    /// Discover the server's tools via `tools/list`.
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let result = self.request("tools/list", json!({})).await?;
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AiError::mcp_protocol(&self.server, "tools/list response missing `tools` array")
            })?;
        Ok(tools
            .iter()
            .map(|tool| McpTool {
                server: self.server.clone(),
                name: tool
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                description: tool
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                input_schema: tool
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object" })),
            })
            .collect())
    }

    /// Invoke `name` with `arguments` via `tools/call`.
    ///
    /// Returns the raw `result` object; a tool reporting `isError: true` yields
    /// a typed [`AiError::McpProtocol`].
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let params = json!({ "name": name, "arguments": arguments });
        let result = self.request("tools/call", params).await?;
        if result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Err(AiError::mcp_protocol(
                &self.server,
                format!("tool '{name}' reported an error"),
            ));
        }
        Ok(result)
    }

    /// Send a JSON-RPC request and await the matching response's `result`.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let frame = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let response = match self.transport {
            McpTransport::Stdio => self.stdio_round_trip(id, &frame).await?,
            McpTransport::Http => self.http_round_trip(id, &frame).await?,
        };
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown JSON-RPC error");
            return Err(AiError::mcp_protocol(
                &self.server,
                format!("{method}: {message}"),
            ));
        }
        response.get("result").cloned().ok_or_else(|| {
            AiError::mcp_protocol(&self.server, format!("{method}: response missing `result`"))
        })
    }

    /// Send a JSON-RPC notification (no response expected).
    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let frame = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        match self.transport {
            McpTransport::Stdio => {
                let connection = self.stdio.as_mut().expect("stdio connection present");
                write_line(&mut connection.stdin, &frame, &self.server).await
            }
            McpTransport::Http => self.http_notify(&frame).await,
        }
    }

    /// POST a notification; only the transport-level status is checked (an MCP
    /// server may answer a notification with an empty `202 Accepted`).
    async fn http_notify(&self, frame: &Value) -> Result<()> {
        let http = self.http.as_ref().expect("http connection present");
        let response = http
            .client
            .post(&http.url)
            .header("Accept", "application/json, text/event-stream")
            .json(frame)
            .send()
            .await
            .map_err(|error| AiError::mcp_server_unreachable(&self.server, error.to_string()))?;
        if !response.status().is_success() {
            return Err(AiError::mcp_server_unreachable(
                &self.server,
                format!("HTTP {}", response.status()),
            ));
        }
        Ok(())
    }

    /// Write one request to the child and read until its response arrives.
    async fn stdio_round_trip(&mut self, id: u64, frame: &Value) -> Result<Value> {
        let timeout = self.timeout;
        let server = self.server.clone();
        let connection = self.stdio.as_mut().expect("stdio connection present");
        write_line(&mut connection.stdin, frame, &server).await?;
        let read = async {
            loop {
                let mut line = String::new();
                let read = connection
                    .stdout
                    .read_line(&mut line)
                    .await
                    .map_err(|error| {
                        AiError::mcp_server_unreachable(&server, format!("read failed: {error}"))
                    })?;
                if read == 0 {
                    return Err(AiError::mcp_server_unreachable(
                        &server,
                        "server closed the connection during handshake",
                    ));
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let value: Value = serde_json::from_str(trimmed).map_err(|error| {
                    AiError::mcp_protocol(&server, format!("invalid JSON frame: {error}"))
                })?;
                if value.get("id").and_then(Value::as_u64) == Some(id) {
                    return Ok(value);
                }
            }
        };
        tokio::time::timeout(timeout, read).await.map_err(|_| {
            AiError::mcp_server_unreachable(&server, "timed out waiting for a response")
        })?
    }

    /// POST one frame and extract the JSON-RPC response (JSON or SSE body).
    async fn http_round_trip(&mut self, id: u64, frame: &Value) -> Result<Value> {
        let http = self.http.as_ref().expect("http connection present");
        let response = http
            .client
            .post(&http.url)
            .header("Accept", "application/json, text/event-stream")
            .json(frame)
            .send()
            .await
            .map_err(|error| AiError::mcp_server_unreachable(&self.server, error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(AiError::mcp_server_unreachable(
                &self.server,
                format!("HTTP {status}"),
            ));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = response
            .text()
            .await
            .map_err(|error| AiError::mcp_server_unreachable(&self.server, error.to_string()))?;
        if content_type.contains("text/event-stream") {
            parse_sse_response(&body, id, &self.server)
        } else {
            serde_json::from_str(&body)
                .map_err(|error| AiError::mcp_protocol(&self.server, error.to_string()))
        }
    }
}

/// Serialize `frame` as one newline-delimited JSON-RPC line.
async fn write_line(stdin: &mut ChildStdin, frame: &Value, server: &str) -> Result<()> {
    let mut bytes = serde_json::to_vec(frame)
        .map_err(|error| AiError::mcp_protocol(server, error.to_string()))?;
    bytes.push(b'\n');
    stdin.write_all(&bytes).await.map_err(|error| {
        AiError::mcp_server_unreachable(server, format!("write failed: {error}"))
    })?;
    stdin
        .flush()
        .await
        .map_err(|error| AiError::mcp_server_unreachable(server, format!("flush failed: {error}")))
}

/// Extract the JSON-RPC response with `id` from an SSE body.
fn parse_sse_response(body: &str, id: u64, server: &str) -> Result<Value> {
    for block in body.split("\n\n") {
        let mut data = Vec::new();
        for line in block.lines() {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if let Some(rest) = line.strip_prefix("data:") {
                data.push(rest.strip_prefix(' ').unwrap_or(rest));
            }
        }
        if data.is_empty() {
            continue;
        }
        let joined = data.join("\n");
        let Ok(value) = serde_json::from_str::<Value>(&joined) else {
            continue;
        };
        // id 0 marks a notification: the first parsed frame is the acknowledgement.
        if id == 0 || value.get("id").and_then(Value::as_u64) == Some(id) {
            return Ok(value);
        }
    }
    Err(AiError::mcp_protocol(
        server,
        "SSE response contained no matching JSON-RPC frame",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_response_extracts_matching_id() {
        let body =
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{\"ok\":true}}\n\n";
        let value = parse_sse_response(body, 7, "mock").unwrap();
        assert_eq!(value["result"]["ok"], true);
    }

    #[test]
    fn sse_response_without_match_is_protocol_error() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n";
        let error = parse_sse_response(body, 99, "mock").unwrap_err();
        assert!(matches!(error, AiError::McpProtocol { .. }));
    }

    #[tokio::test]
    async fn missing_stdio_binary_is_unreachable() {
        let config = McpServerConfig::stdio("missing", "/nonexistent/rustasea-mcp-binary-xyz");
        let error = McpClient::connect(&config)
            .await
            .err()
            .expect("connect to a missing binary must fail");
        assert!(matches!(error, AiError::McpServerUnreachable { .. }));
    }
}
