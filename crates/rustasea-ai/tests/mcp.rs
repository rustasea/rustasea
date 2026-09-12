//! MCP client tests against in-process mock servers (feature `mcp`).
//!
//! Positive: a mock JSON-RPC server exposes a tool that the registry discovers
//! and invokes. Negative: an unreachable server surfaces a typed error rather
//! than an empty discovery. The HTTP mock speaks just enough HTTP/1.1 to serve
//! canned JSON-RPC frames, keeping the suite hermetic and offline.
#![cfg(feature = "mcp")]

use std::time::Duration;

use rustasea_ai::mcp::{McpClient, McpRegistry, McpServerConfig};
use rustasea_ai::AiError;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Read one HTTP request (headers + `Content-Length` body) or until EOF.
async fn read_request(socket: &mut TcpStream) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = socket.read(&mut chunk).await.unwrap_or(0);
        if read == 0 {
            return buffer;
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
                return buffer;
            }
        }
    }
}

/// Extract the JSON-RPC body from a raw HTTP request.
fn request_body(raw: &[u8]) -> Value {
    let position = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("request has header terminator");
    serde_json::from_slice(&raw[position + 4..]).unwrap_or(Value::Null)
}

/// Build a JSON-RPC response frame for `id` carrying `result`.
fn rpc_result(id: u64, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

/// Serve MCP JSON-RPC responses derived from each request's `method`.
///
/// Handles `initialize`, `notifications/initialized` (202), `tools/list`, and
/// `tools/call`. Returns the bound base URL.
async fn spawn_mcp_http_mock() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let raw = read_request(&mut socket).await;
            let body = request_body(&raw);
            let method = body
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let id = body.get("id").and_then(Value::as_u64);
            let payload = match (method, id) {
                ("initialize", Some(id)) => rpc_result(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "mock", "version": "0.1" }
                    }),
                ),
                ("tools/list", Some(id)) => rpc_result(
                    id,
                    json!({
                        "tools": [{
                            "name": "echo",
                            "description": "Echoes the provided text",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "text": { "type": "string" } }
                            }
                        }]
                    }),
                ),
                ("tools/call", Some(id)) => rpc_result(
                    id,
                    json!({
                        "content": [{ "type": "text", "text": "hi" }],
                        "isError": false
                    }),
                ),
                // Notifications carry no `id`; acknowledge with an empty 202.
                _ => String::new(),
            };
            let (status, reason, content_type) = if payload.is_empty() {
                (202, "Accepted", "application/json")
            } else {
                (200, "OK", "application/json")
            };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;
        }
    });
    format!("http://{address}")
}

/// POSITIVE — discovery returns the tool the mock advertises.
#[tokio::test]
async fn http_discovery_returns_mock_tool() {
    let base = spawn_mcp_http_mock().await;
    let mut registry = McpRegistry::new();
    registry.add_server(McpServerConfig::http("mock", base));

    let tools = registry.discover_tools().await.expect("discover");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].server, "mock");
    assert_eq!(tools[0].name, "echo");
    assert_eq!(tools[0].description, "Echoes the provided text");
    assert_eq!(tools[0].input_schema["type"], "object");
}

/// POSITIVE — the discovered tool can be invoked through the registry.
#[tokio::test]
async fn http_invocation_returns_mock_result() {
    let base = spawn_mcp_http_mock().await;
    let mut registry = McpRegistry::new();
    registry.add_server(McpServerConfig::http("mock", base));

    let tools = registry.discover_tools().await.expect("discover");
    let result = registry
        .call_tool(&tools[0], json!({ "text": "hi" }))
        .await
        .expect("call");
    assert_eq!(result["content"][0]["text"], "hi");
    assert_eq!(result["isError"], false);
}

/// POSITIVE — a stdio server is discovered through the child-process transport.
#[tokio::test]
async fn stdio_discovery_returns_mock_tool() {
    // A tiny POSIX shell JSON-RPC server: answers `initialize` and
    // `tools/list` (by request id), ignores the id-less notification.
    let script = concat!(
        "while IFS= read -r line; do\n",
        "  id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\\([0-9]*\\).*/\\1/p')\n",
        "  case \"$id\" in\n",
        "    1) printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{},\"serverInfo\":{\"name\":\"mock\",\"version\":\"0\"}}}';;\n",
        "    2) printf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\",\"description\":\"Echoes text\",\"inputSchema\":{\"type\":\"object\"}}]}}';;\n",
        "    *) : ;;\n",
        "  esac\n",
        "done\n"
    );
    let config = McpServerConfig::stdio_with_args("mock-stdio", "sh", ["-c", script]);
    let mut client = McpClient::connect(&config).await.expect("connect");
    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
}

/// NEGATIVE — an unreachable HTTP server yields a typed error, not empty tools.
#[tokio::test]
async fn unreachable_http_server_is_typed_error() {
    // Bind then drop the listener so the port refuses connections.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    let mut registry = McpRegistry::new();
    registry.add_server(McpServerConfig::http("down", format!("http://{address}")));
    let error = registry.discover_tools().await.unwrap_err();
    assert!(
        matches!(error, AiError::McpServerUnreachable { .. }),
        "unexpected error: {error}"
    );
}

/// NEGATIVE — a missing stdio binary yields a typed unreachable error.
#[tokio::test]
async fn unreachable_stdio_server_is_typed_error() {
    let mut registry = McpRegistry::new();
    registry.add_server(McpServerConfig::stdio(
        "missing",
        "/nonexistent/rustasea-mcp-binary-xyz",
    ));
    let error = registry.discover_tools().await.unwrap_err();
    assert!(
        matches!(error, AiError::McpServerUnreachable { .. }),
        "unexpected error: {error}"
    );
}

/// NEGATIVE — invoking an unconfigured server is a typed unreachable error.
#[tokio::test]
async fn invoking_unconfigured_server_is_typed_error() {
    let registry = McpRegistry::new();
    let tool = rustasea_ai::mcp::McpTool {
        server: "ghost".to_string(),
        name: "echo".to_string(),
        description: String::new(),
        input_schema: json!({ "type": "object" }),
    };
    let error = registry.call_tool(&tool, json!({})).await.unwrap_err();
    assert!(matches!(error, AiError::McpServerUnreachable { .. }));
}

/// The client honours a custom round-trip timeout without panicking.
#[tokio::test]
async fn client_timeout_is_configurable() {
    let base = spawn_mcp_http_mock().await;
    let config = McpServerConfig::http("mock", base);
    let client = McpClient::connect(&config)
        .await
        .expect("connect")
        .with_timeout(Duration::from_secs(5));
    assert_eq!(client.transport(), rustasea_ai::mcp::McpTransport::Http);
}
