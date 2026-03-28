//! Unix domain socket server for GUI-MCP bridge.
//!
//! When the Lattice GUI is running, it listens on a Unix socket at
//! `~/Library/Application Support/Lattice/lattice.sock`.  An MCP stdio
//! process can connect to this socket to proxy JSON-RPC commands to the
//! live GUI workbook, so changes appear in real time.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::RwLock;

use lattice_core::{ConditionalFormatStore, Workbook};
use lattice_mcp::McpServer;

/// Return the canonical socket path for the GUI instance.
///
/// On macOS this is `~/Library/Application Support/Lattice/lattice.sock`.
/// Falls back to the current directory if the system data directory
/// cannot be determined.
pub fn socket_path() -> PathBuf {
    let support = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    support.join("Lattice").join("lattice.sock")
}

/// Remove the socket file if it exists (cleanup on shutdown or before bind).
pub fn remove_socket() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
}

/// Start the Unix socket server that accepts MCP connections.
///
/// Each connection is handled in a separate tokio task.  Incoming
/// newline-delimited JSON-RPC messages are processed by an `McpServer`
/// instance that shares the GUI workbook.  After any write command the
/// `workbook-changed` Tauri event is emitted so the frontend repaints.
pub async fn start_socket_server(
    workbook: Arc<RwLock<Workbook>>,
    conditional_formats: Arc<RwLock<ConditionalFormatStore>>,
    app_handle: tauri::AppHandle,
) {
    let path = socket_path();

    // Remove stale socket from a previous run.
    let _ = std::fs::remove_file(&path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("lattice: failed to bind socket at {:?}: {}", path, e);
            return;
        }
    };

    eprintln!("lattice: socket server listening at {:?}", path);

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let wb = Arc::clone(&workbook);
                let cf = Arc::clone(&conditional_formats);
                let handle = app_handle.clone();
                tokio::spawn(async move {
                    handle_connection(stream, wb, cf, handle).await;
                });
            }
            Err(e) => {
                eprintln!("lattice: socket accept error: {}", e);
            }
        }
    }
}

/// Handle a single socket connection.
///
/// Reads newline-delimited JSON-RPC messages, dispatches them through
/// an `McpServer`, and writes responses back.  Write operations trigger
/// a Tauri event to refresh the frontend.
async fn handle_connection(
    stream: tokio::net::UnixStream,
    workbook: Arc<RwLock<Workbook>>,
    _conditional_formats: Arc<RwLock<ConditionalFormatStore>>,
    app_handle: tauri::AppHandle,
) {
    use tauri::Emitter;

    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut mcp_server = McpServer::new(workbook);

    loop {
        let mut line = String::new();
        match buf_reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF — client disconnected.
                eprintln!("lattice: socket client disconnected");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Detect if this is a write/mutating operation so we
                // can emit a frontend refresh event afterwards.
                let is_write = is_mutating_request(trimmed);

                if let Some(response) = mcp_server.handle_message(trimmed).await {
                    let msg = format!("{}\n", response);
                    if let Err(e) = writer.write_all(msg.as_bytes()).await {
                        eprintln!("lattice: socket write error: {}", e);
                        break;
                    }
                    if let Err(e) = writer.flush().await {
                        eprintln!("lattice: socket flush error: {}", e);
                        break;
                    }
                }

                // After a successful write operation, notify the frontend.
                if is_write {
                    let _ = app_handle.emit("workbook-changed", ());
                }
            }
            Err(e) => {
                eprintln!("lattice: socket read error: {}", e);
                break;
            }
        }
    }
}

/// Check whether a JSON-RPC request is a mutating (write) operation.
///
/// We parse just enough of the JSON to extract the tool name from
/// `tools/call` requests and check it against a list of known write
/// operations.  This is best-effort — if parsing fails we assume
/// non-mutating.
fn is_mutating_request(json_str: &str) -> bool {
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let method = parsed["method"].as_str().unwrap_or("");
    if method != "tools/call" {
        return false;
    }

    let tool_name = parsed["params"]["name"].as_str().unwrap_or("");

    matches!(
        tool_name,
        "write_cell"
            | "write_range"
            | "clear_range"
            | "find_replace"
            | "sort_range"
            | "deduplicate"
            | "transpose"
            | "auto_fill"
            | "remove_duplicates"
            | "text_to_columns"
            | "replace_in_workbook"
            | "create_sheet"
            | "rename_sheet"
            | "delete_sheet"
            | "hide_rows"
            | "unhide_rows"
            | "hide_cols"
            | "unhide_cols"
            | "protect_sheet"
            | "unprotect_sheet"
            | "set_sheet_tab_color"
            | "set_cell_format"
            | "merge_cells"
            | "unmerge_cells"
            | "insert_formula"
            | "bulk_formula"
            | "set_validation"
            | "remove_validation"
            | "add_conditional_format"
            | "remove_conditional_format"
            | "add_named_range"
            | "remove_named_range"
            | "add_named_function"
            | "remove_named_function"
            | "add_sparkline"
            | "remove_sparkline"
            | "save_filter_view"
            | "apply_filter_view"
            | "delete_filter_view"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_is_absolute() {
        let path = socket_path();
        assert!(
            path.is_absolute() || path.starts_with("."),
            "socket path should be absolute or fallback"
        );
        assert!(path.ends_with("lattice.sock"));
    }

    #[test]
    fn test_is_mutating_request_write_cell() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"write_cell","arguments":{"sheet":"Sheet1","cell_ref":"A1","value":"hello"}}}"#;
        assert!(is_mutating_request(req));
    }

    #[test]
    fn test_is_mutating_request_read_cell() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_cell","arguments":{"sheet":"Sheet1","cell_ref":"A1"}}}"#;
        assert!(!is_mutating_request(req));
    }

    #[test]
    fn test_is_mutating_request_initialize() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        assert!(!is_mutating_request(req));
    }

    #[test]
    fn test_is_mutating_request_invalid_json() {
        assert!(!is_mutating_request("not valid json"));
    }

    #[test]
    fn test_is_mutating_request_sort_range() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"sort_range","arguments":{}}}"#;
        assert!(is_mutating_request(req));
    }

    #[test]
    fn test_is_mutating_request_list_sheets() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_sheets","arguments":{}}}"#;
        assert!(!is_mutating_request(req));
    }
}
