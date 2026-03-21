//! stdio transport — reads JSON-RPC messages from stdin, writes to stdout.

use std::future::Future;
use std::pin::Pin;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::Transport;

/// Transport that reads from stdin and writes to stdout.
///
/// Messages are newline-delimited JSON strings.
/// Logs and diagnostics go to stderr (never write non-JSON to stdout).
pub struct StdioTransport {
    reader: BufReader<io::Stdin>,
    writer: io::Stdout,
}

impl StdioTransport {
    /// Create a new stdio transport.
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
            writer: io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for StdioTransport {
    fn read_message(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<Option<String>>> + Send + '_>> {
        Box::pin(async move {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                // EOF
                return Ok(None);
            }
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                // Skip empty lines and read the next one.
                return Ok(Some(String::new()));
            }
            Ok(Some(trimmed))
        })
    }

    fn write_message(
        &mut self,
        message: &str,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + '_>> {
        let msg = format!("{}\n", message);
        Box::pin(async move {
            self.writer.write_all(msg.as_bytes()).await?;
            self.writer.flush().await?;
            Ok(())
        })
    }
}
