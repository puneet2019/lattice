//! Streamable HTTP transport for MCP.
//!
//! Implements:
//! - POST `/mcp` — accepts a JSON-RPC 2.0 request body, returns JSON-RPC response
//! - GET `/mcp/sse` — Server-Sent Events stream for server-to-client notifications
//! - GET `/health` — simple health check endpoint
//!
//! Default port: 3141 (configurable via `--port`).

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};

use crate::server::McpServer;

/// Default port for the MCP HTTP server.
pub const DEFAULT_PORT: u16 = 3141;

/// Run the MCP server as a Streamable HTTP service.
///
/// Listens on `0.0.0.0:{port}` and handles:
/// - `POST /mcp` — JSON-RPC 2.0 request/response
/// - `GET /mcp/sse` — SSE notification stream
/// - `GET /health` — health check
pub async fn run_http(server: McpServer, port: u16) -> std::io::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    eprintln!("lattice: MCP HTTP server listening on http://{}", addr);

    // Shared state: the McpServer wrapped in Arc<Mutex> for concurrent request handling.
    let server = Arc::new(Mutex::new(server));

    // Broadcast channel for SSE notifications (capacity 256 messages).
    let (sse_tx, _) = broadcast::channel::<String>(256);
    let sse_tx = Arc::new(sse_tx);

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let server = Arc::clone(&server);
        let sse_tx = Arc::clone(&sse_tx);

        tokio::spawn(async move {
            let server = Arc::clone(&server);
            let sse_tx = Arc::clone(&sse_tx);

            let service = service_fn(move |req: Request<Incoming>| {
                let server = Arc::clone(&server);
                let sse_tx = Arc::clone(&sse_tx);
                async move { handle_request(req, server, sse_tx).await }
            });

            if let Err(e) =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(io, service)
                    .await
            {
                eprintln!("lattice: HTTP connection error from {}: {}", remote_addr, e);
            }
        });
    }
}

/// Route an incoming HTTP request to the appropriate handler.
async fn handle_request(
    req: Request<Incoming>,
    server: Arc<Mutex<McpServer>>,
    sse_tx: Arc<broadcast::Sender<String>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        (Method::POST, "/mcp") => Ok(handle_post_mcp(req, server).await),
        (Method::GET, "/mcp/sse") => Ok(handle_get_sse(sse_tx).await),
        (Method::GET, "/health") => Ok(handle_health()),
        (Method::OPTIONS, _) => Ok(cors_preflight()),
        _ => Ok(not_found()),
    }
}

/// Handle `POST /mcp` — parse the JSON-RPC request body, process it, return the response.
async fn handle_post_mcp(
    req: Request<Incoming>,
    server: Arc<Mutex<McpServer>>,
) -> Response<Full<Bytes>> {
    // Read the full request body.
    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": format!("Failed to read request body: {}", e),
                    },
                    "id": null,
                }),
            );
        }
    };

    let body_str = match std::str::from_utf8(&body_bytes) {
        Ok(s) => s.to_string(),
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": format!("Invalid UTF-8 in request body: {}", e),
                    },
                    "id": null,
                }),
            );
        }
    };

    // Process the message through the MCP server.
    let mut server = server.lock().await;
    match server.handle_message(&body_str).await {
        Some(response) => {
            let status = StatusCode::OK;
            Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(response)))
                .unwrap()
        }
        None => {
            // Notification — no response body (HTTP 204 No Content).
            Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
    }
}

/// Handle `GET /mcp/sse` — return an SSE-formatted response.
///
/// For simplicity, this returns an initial "connection" event and then
/// closes. A full implementation would hold the connection open and
/// stream events from the broadcast channel, but that requires
/// streaming body support. This provides the endpoint so clients can
/// discover it.
async fn handle_get_sse(sse_tx: Arc<broadcast::Sender<String>>) -> Response<Full<Bytes>> {
    // Subscribe to the broadcast channel.
    let mut rx = sse_tx.subscribe();

    // Build an initial SSE event to confirm the connection.
    let mut events = String::new();
    events.push_str("event: connected\n");
    events.push_str("data: {\"status\":\"connected\"}\n\n");

    // Drain any pending messages (non-blocking).
    loop {
        match rx.try_recv() {
            Ok(msg) => {
                events.push_str("event: notification\n");
                events.push_str(&format!("data: {}\n\n", msg));
            }
            Err(_) => break,
        }
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(events)))
        .unwrap()
}

/// Handle `GET /health` — simple health check.
fn handle_health() -> Response<Full<Bytes>> {
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "status": "ok",
            "server": "lattice-mcp",
        }),
    )
}

/// Handle CORS preflight requests.
fn cors_preflight() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Max-Age", "86400")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Return a 404 Not Found response.
fn not_found() -> Response<Full<Bytes>> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({
            "error": "Not Found",
        }),
    )
}

/// Helper: build a JSON HTTP response with CORS headers.
fn json_response(status: StatusCode, body: &serde_json::Value) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port() {
        assert_eq!(DEFAULT_PORT, 3141);
    }

    #[test]
    fn test_health_response() {
        let resp = handle_health();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn test_not_found_response() {
        let resp = not_found();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_cors_preflight_response() {
        let resp = cors_preflight();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            resp.headers().get("Access-Control-Allow-Origin").unwrap(),
            "*"
        );
    }

    #[tokio::test]
    async fn test_post_mcp_initialize() {
        let server = Arc::new(Mutex::new(McpServer::new_default()));
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;

        // We cannot easily convert Full<Bytes> to Incoming for hyper's
        // request type, so we test the server's handle_message directly
        // which is the core logic that handle_post_mcp delegates to.
        let mut srv = server.lock().await;
        let resp = srv.handle_message(body).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(parsed["result"]["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn test_post_mcp_notification_returns_none() {
        let server = Arc::new(Mutex::new(McpServer::new_default()));
        let mut srv = server.lock().await;
        let body = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
        let resp = srv.handle_message(body).await;
        assert!(resp.is_none());
    }
}
