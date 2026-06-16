//! Native MCP server — async wrapper around the sync [`crate::dispatch`]
//! pipeline.
//!
//! This file is `native`-only. The protocol dispatch itself is sync (see
//! [`crate::dispatch::handle_request`]); `McpServer` exists so the
//! desktop build can keep its `Arc<RwLock<Workbook>>` sharing model (so
//! the GUI and the MCP server can mutate the same workbook concurrently)
//! and so the stdio and HTTP transports continue to work unchanged.

use std::sync::Arc;

use tokio::sync::RwLock;

use lattice_core::{ConditionalFormatStore, Workbook};

use crate::dispatch::{self, McpState};
use crate::tools::ToolRegistry;

/// The MCP protocol version we implement.
pub const PROTOCOL_VERSION: &str = dispatch::PROTOCOL_VERSION;

/// The server name reported during initialization.
pub const SERVER_NAME: &str = dispatch::SERVER_NAME;

/// The server version reported during initialization.
pub const SERVER_VERSION: &str = dispatch::SERVER_VERSION;

/// MCP server that wraps a workbook and handles JSON-RPC 2.0 messages.
pub struct McpServer {
    /// The workbook being operated on, shared with potential GUI.
    workbook: Arc<RwLock<Workbook>>,
    /// Conditional formatting store (kept separate from workbook, same as Tauri).
    conditional_formats: Arc<RwLock<ConditionalFormatStore>>,
    /// Registry of available tools.
    tool_registry: ToolRegistry,
    /// Whether the server has been initialized.
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server wrapping the given workbook.
    pub fn new(workbook: Arc<RwLock<Workbook>>) -> Self {
        Self {
            workbook,
            conditional_formats: Arc::new(RwLock::new(ConditionalFormatStore::new())),
            tool_registry: ToolRegistry::default_registry(),
            initialized: false,
        }
    }

    /// Create a new MCP server with a default empty workbook.
    pub fn new_default() -> Self {
        Self::new(Arc::new(RwLock::new(Workbook::new())))
    }

    /// Run the MCP server over stdio (stdin/stdout).
    ///
    /// Reads newline-delimited JSON-RPC 2.0 messages from stdin, processes
    /// each one, and writes responses to stdout. Logs go to stderr.
    /// The loop runs until EOF on stdin.
    pub async fn run_stdio(&mut self) -> std::io::Result<()> {
        use crate::transport::Transport;
        use crate::transport::stdio::StdioTransport;

        let mut transport = StdioTransport::new();

        eprintln!("lattice: MCP server starting on stdio");

        loop {
            let message = match transport.read_message().await? {
                Some(msg) => msg,
                None => {
                    // EOF — client disconnected.
                    eprintln!("lattice: stdin closed, shutting down MCP server");
                    break;
                }
            };

            // Skip empty lines.
            if message.is_empty() {
                continue;
            }

            if let Some(response) = self.handle_message(&message).await {
                transport.write_message(&response).await?;
            }
        }

        Ok(())
    }

    /// Run the MCP server as a Streamable HTTP service.
    ///
    /// Listens on `localhost:{port}` and handles JSON-RPC 2.0 messages via
    /// `POST /mcp`, SSE notifications via `GET /mcp/sse`, and health checks
    /// via `GET /health`.
    pub async fn run_http(self, port: u16) -> std::io::Result<()> {
        crate::transport::http::run_http(self, port).await
    }

    /// Handle an incoming JSON-RPC 2.0 message and return a response.
    ///
    /// Acquires write locks on the workbook and conditional-format store
    /// for the duration of the request and delegates to the sync
    /// [`crate::dispatch::handle_request`]. The locks are held for the
    /// whole request — same semantics as before — so concurrent tool
    /// calls serialize through `tokio::RwLock` just as they did when each
    /// arm acquired its own guard.
    pub async fn handle_message(&mut self, message: &str) -> Option<String> {
        let mut workbook = self.workbook.write().await;
        let mut conditional_formats = self.conditional_formats.write().await;

        let mut state = McpState {
            workbook: &mut workbook,
            conditional_formats: &mut conditional_formats,
            initialized: &mut self.initialized,
            tool_registry: &self.tool_registry,
        };

        dispatch::handle_request(&mut state, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[tokio::test]
    async fn test_initialize() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(parsed["result"]["capabilities"]["tools"].is_object());
        assert!(parsed["result"]["capabilities"]["prompts"].is_object());
        assert_eq!(parsed["id"], 1);
    }

    #[tokio::test]
    async fn test_tools_list() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#)
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        let tools = parsed["result"]["tools"].as_array().unwrap();
        // We should have 61+ tools (all tool modules implemented).
        assert!(
            tools.len() >= 61,
            "Expected at least 61 tools, got {}",
            tools.len()
        );

        // Verify key tools are present.
        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tool_names.contains(&"read_cell"));
        assert!(tool_names.contains(&"write_cell"));
        assert!(tool_names.contains(&"clear_range"));
        assert!(tool_names.contains(&"find_replace"));
        assert!(tool_names.contains(&"sort_range"));
        assert!(tool_names.contains(&"deduplicate"));
        assert!(tool_names.contains(&"transpose"));
        // formula_ops tools
        assert!(tool_names.contains(&"evaluate_formula"));
        assert!(tool_names.contains(&"get_formula"));
        assert!(tool_names.contains(&"insert_formula"));
        assert!(tool_names.contains(&"bulk_formula"));
        // format_ops tools
        assert!(tool_names.contains(&"get_cell_format"));
        assert!(tool_names.contains(&"set_cell_format"));
        assert!(tool_names.contains(&"merge_cells"));
        assert!(tool_names.contains(&"unmerge_cells"));
        // find_replace_ops tools
        assert!(tool_names.contains(&"find_in_workbook"));
        assert!(tool_names.contains(&"replace_in_workbook"));
        // named_range_ops tools
        assert!(tool_names.contains(&"add_named_range"));
        assert!(tool_names.contains(&"remove_named_range"));
        assert!(tool_names.contains(&"list_named_ranges"));
        assert!(tool_names.contains(&"resolve_named_range"));
        // named_function_ops tools
        assert!(tool_names.contains(&"add_named_function"));
        assert!(tool_names.contains(&"remove_named_function"));
        assert!(tool_names.contains(&"list_named_functions"));
        // validation_ops tools
        assert!(tool_names.contains(&"set_validation"));
        assert!(tool_names.contains(&"get_validation"));
        assert!(tool_names.contains(&"remove_validation"));
        assert!(tool_names.contains(&"validate_cell"));
        assert!(tool_names.contains(&"describe_data"));
        assert!(tool_names.contains(&"correlate"));
        assert!(tool_names.contains(&"trend_analysis"));
        assert!(tool_names.contains(&"create_chart"));
        assert!(tool_names.contains(&"list_charts"));
        assert!(tool_names.contains(&"delete_chart"));
        assert!(tool_names.contains(&"get_workbook_info"));
        assert!(tool_names.contains(&"export_json"));
        assert!(tool_names.contains(&"export_csv"));
        // Data tools added for MCP coverage audit
        assert!(tool_names.contains(&"remove_duplicates"));
        assert!(tool_names.contains(&"text_to_columns"));
        assert!(tool_names.contains(&"auto_fill"));
        assert!(tool_names.contains(&"generate_pivot"));
        // Sheet management tools
        assert!(tool_names.contains(&"hide_rows"));
        assert!(tool_names.contains(&"unhide_rows"));
        assert!(tool_names.contains(&"hide_cols"));
        assert!(tool_names.contains(&"unhide_cols"));
        assert!(tool_names.contains(&"protect_sheet"));
        assert!(tool_names.contains(&"unprotect_sheet"));
        assert!(tool_names.contains(&"set_sheet_tab_color"));
        // Conditional format tools
        assert!(tool_names.contains(&"add_conditional_format"));
        assert!(tool_names.contains(&"list_conditional_formats"));
        assert!(tool_names.contains(&"remove_conditional_format"));
        // Sparkline tools
        assert!(tool_names.contains(&"add_sparkline"));
        assert!(tool_names.contains(&"remove_sparkline"));
        assert!(tool_names.contains(&"list_sparklines"));
    }

    #[tokio::test]
    async fn test_tools_call_read_cell() {
        let mut server = McpServer::new_default();

        // Write a cell first.
        {
            let mut wb = server.workbook.write().await;
            wb.set_cell("Sheet1", 0, 0, lattice_core::CellValue::Number(42.0))
                .unwrap();
        }

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_cell","arguments":{"sheet":"Sheet1","cell_ref":"A1"}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], false);
    }

    #[tokio::test]
    async fn test_tools_call_clear_range() {
        let mut server = McpServer::new_default();

        {
            let mut wb = server.workbook.write().await;
            wb.set_cell("Sheet1", 0, 0, lattice_core::CellValue::Number(1.0))
                .unwrap();
            wb.set_cell("Sheet1", 0, 1, lattice_core::CellValue::Number(2.0))
                .unwrap();
        }

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"clear_range","arguments":{"sheet":"Sheet1","range":"A1:B1"}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], false);
    }

    #[tokio::test]
    async fn test_tools_call_describe_data() {
        let mut server = McpServer::new_default();

        {
            let mut wb = server.workbook.write().await;
            for i in 0..5 {
                wb.set_cell(
                    "Sheet1",
                    i,
                    0,
                    lattice_core::CellValue::Number((i + 1) as f64),
                )
                .unwrap();
            }
        }

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"describe_data","arguments":{"sheet":"Sheet1","range":"A1:A5"}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], false);
    }

    #[tokio::test]
    async fn test_tools_call_get_workbook_info() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_workbook_info","arguments":{}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], false);
    }

    #[tokio::test]
    async fn test_tools_call_evaluate_formula() {
        let mut server = McpServer::new_default();

        // Set up data for the formula to reference.
        {
            let mut wb = server.workbook.write().await;
            wb.set_cell("Sheet1", 0, 0, lattice_core::CellValue::Number(10.0))
                .unwrap();
            wb.set_cell("Sheet1", 1, 0, lattice_core::CellValue::Number(20.0))
                .unwrap();
        }

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"evaluate_formula","arguments":{"sheet":"Sheet1","formula":"SUM(A1:A2)"}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["result"]["isError"], false);

        // Parse the text content to verify the result.
        let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
        let result_val: Value = serde_json::from_str(text).unwrap();
        assert_eq!(result_val["result"], 30.0);
        assert_eq!(result_val["result_type"], "number");
    }

    #[tokio::test]
    async fn test_resources_read_workbook_info() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"lattice://workbook/info"}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert!(parsed["result"]["contents"].is_array());
    }

    #[tokio::test]
    async fn test_resources_read_sheet_data() {
        let mut server = McpServer::new_default();

        {
            let mut wb = server.workbook.write().await;
            wb.set_cell("Sheet1", 0, 0, lattice_core::CellValue::Number(42.0))
                .unwrap();
        }

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"lattice://sheet/Sheet1/data"}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert!(parsed["result"]["contents"].is_array());
    }

    #[tokio::test]
    async fn test_resources_read_sheet_summary() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":10,"method":"resources/read","params":{"uri":"lattice://sheet/Sheet1/summary"}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert!(parsed["result"]["contents"].is_array());
    }

    #[tokio::test]
    async fn test_prompts_list() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(r#"{"jsonrpc":"2.0","id":11,"method":"prompts/list","params":{}}"#)
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        let prompts = parsed["result"]["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 6);
    }

    #[tokio::test]
    async fn test_prompts_get() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":12,"method":"prompts/get","params":{"name":"analyze-portfolio"}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert!(parsed["result"]["messages"].is_array());
    }

    #[tokio::test]
    async fn test_prompts_get_unknown() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":13,"method":"prompts/get","params":{"name":"nonexistent"}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert!(parsed["error"].is_object());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(r#"{"jsonrpc":"2.0","id":14,"method":"nonexistent","params":{}}"#)
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_parse_error() {
        let mut server = McpServer::new_default();

        let response = server.handle_message("not valid json").await.unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn test_notification_no_response() {
        let mut server = McpServer::new_default();

        // initialized is a notification (no id).
        let response = server
            .handle_message(r#"{"jsonrpc":"2.0","method":"initialized"}"#)
            .await;

        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["error"]["code"], -32602);
    }
}
