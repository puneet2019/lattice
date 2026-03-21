//! MCP transport layer — abstracts message I/O from protocol logic.

pub mod http;
pub mod stdio;

use std::future::Future;
use std::pin::Pin;

/// Trait for MCP message transports (stdio, HTTP, etc.).
///
/// Transports read and write newline-delimited JSON-RPC 2.0 messages.
pub trait Transport: Send + Sync {
    /// Read the next message from the transport.
    ///
    /// Returns `None` if the transport is closed (EOF).
    fn read_message(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Option<String>>> + Send + '_>>;

    /// Write a message to the transport.
    fn write_message(
        &mut self,
        message: &str,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + '_>>;
}
