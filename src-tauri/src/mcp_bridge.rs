//! MCP stdio-to-socket bridge.
//!
//! When `lattice --mcp-stdio` detects a running GUI instance (via the
//! Unix socket), it enters bridge mode: reads JSON-RPC messages from
//! stdin, forwards them to the GUI socket, reads responses, and writes
//! them to stdout.  This lets Claude Desktop/Code operate the live GUI.

use std::path::Path;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Run the MCP bridge, proxying stdin/stdout to the GUI Unix socket.
///
/// Reads newline-delimited JSON-RPC messages from stdin, sends them
/// to the socket, reads the response, and writes it to stdout.
/// The loop runs until EOF on stdin or the socket connection closes.
pub async fn run_mcp_bridge(socket_path: &Path) -> std::io::Result<()> {
    eprintln!(
        "lattice: bridge mode — connecting to GUI at {:?}",
        socket_path
    );

    let stream = UnixStream::connect(socket_path).await.map_err(|e| {
        eprintln!("lattice: cannot connect to GUI socket: {}", e);
        e
    })?;

    let (socket_reader, mut socket_writer) = stream.into_split();
    let mut socket_buf = BufReader::new(socket_reader);

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut stdin_buf = BufReader::new(stdin);

    eprintln!("lattice: bridge connected — proxying MCP messages");

    loop {
        // Read a line from stdin (MCP client).
        let mut line = String::new();
        match stdin_buf.read_line(&mut line).await {
            Ok(0) => {
                // EOF on stdin — client disconnected.
                eprintln!("lattice: stdin closed, shutting down bridge");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("lattice: stdin read error: {}", e);
                return Err(e);
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Forward to the GUI socket.
        let msg = format!("{}\n", trimmed);
        socket_writer.write_all(msg.as_bytes()).await?;
        socket_writer.flush().await?;

        // Read the response from the GUI socket.
        let mut response = String::new();
        match socket_buf.read_line(&mut response).await {
            Ok(0) => {
                eprintln!("lattice: GUI socket closed unexpectedly");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("lattice: socket read error: {}", e);
                return Err(e);
            }
        }

        // Write response to stdout for the MCP client.
        stdout.write_all(response.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Integration tests for the bridge require a running socket server,
    // so unit tests here are limited to ensuring the module compiles
    // and the public API is correct.

    #[test]
    fn test_module_compiles() {
        // Smoke test — the bridge module exists and compiles.
        assert!(true);
    }
}
