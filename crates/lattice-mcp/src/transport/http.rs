//! HTTP (Streamable HTTP) transport — stub.
//!
//! Will implement POST for JSON-RPC messages and SSE for
//! server-to-client notifications. Default port: 3141.

use std::future::Future;
use std::pin::Pin;

use super::Transport;

/// Streamable HTTP transport for networked MCP clients.
///
/// Not yet implemented — will use hyper or axum.
pub struct HttpTransport {
    /// Port to listen on (default: 3141).
    pub port: u16,
}

impl HttpTransport {
    /// Create a new HTTP transport stub.
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new(3141)
    }
}

impl Transport for HttpTransport {
    fn read_message(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Option<String>>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement HTTP request reading
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "HTTP transport not yet implemented",
            ))
        })
    }

    fn write_message(
        &mut self,
        _message: &str,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + '_>> {
        Box::pin(async move {
            // TODO: implement HTTP response writing
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "HTTP transport not yet implemented",
            ))
        })
    }
}
