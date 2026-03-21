//! MCP server — handles JSON-RPC 2.0 messages and dispatches to tools.

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::RwLock;

use lattice_core::Workbook;

use crate::tools::ToolRegistry;
use crate::tools::{
    analysis, cell_ops, chart_ops, data_ops, file_ops, sheet_ops,
};

/// The MCP protocol version we implement.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// The server name reported during initialization.
const SERVER_NAME: &str = "lattice";

/// The server version reported during initialization.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// MCP server that wraps a workbook and handles JSON-RPC 2.0 messages.
pub struct McpServer {
    /// The workbook being operated on, shared with potential GUI.
    workbook: Arc<RwLock<Workbook>>,
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
            tool_registry: ToolRegistry::default_registry(),
            initialized: false,
        }
    }

    /// Create a new MCP server with a default empty workbook.
    pub fn new_default() -> Self {
        Self::new(Arc::new(RwLock::new(Workbook::new())))
    }

    /// Handle an incoming JSON-RPC 2.0 message and return a response.
    ///
    /// Parses the method, dispatches to the appropriate handler,
    /// and wraps the result in a JSON-RPC response envelope.
    pub async fn handle_message(&mut self, message: &str) -> Option<String> {
        let request: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(e) => {
                return Some(
                    json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e),
                        },
                        "id": null,
                    })
                    .to_string(),
                );
            }
        };

        let method = request["method"].as_str().unwrap_or("");
        let id = request.get("id").cloned();
        let params = request.get("params").cloned().unwrap_or(json!({}));

        // Notifications (no id) don't get responses.
        let is_notification = id.is_none();

        let result = match method {
            "initialize" => self.handle_initialize(params),
            "initialized" => {
                // Notification — no response needed.
                return None;
            }
            "ping" => Ok(json!({})),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(params).await,
            "resources/list" => crate::resources::handle_list_resources(),
            "resources/read" => {
                let wb = self.workbook.read().await;
                crate::resources::handle_read_resource(params, &wb)
            }
            "prompts/list" => crate::prompts::handle_list_prompts(),
            "prompts/get" => crate::prompts::handle_get_prompt(params),
            "" => Err((-32600, "Invalid Request: missing method".to_string())),
            _ => Err((-32601, format!("Method not found: {}", method))),
        };

        if is_notification {
            return None;
        }

        let response = match result {
            Ok(result_value) => {
                json!({
                    "jsonrpc": "2.0",
                    "result": result_value,
                    "id": id,
                })
            }
            Err((code, message)) => {
                json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": code,
                        "message": message,
                    },
                    "id": id,
                })
            }
        };

        Some(response.to_string())
    }

    /// Handle the `initialize` method.
    fn handle_initialize(&mut self, _params: Value) -> Result<Value, (i32, String)> {
        self.initialized = true;

        Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false },
                "prompts": { "listChanged": false },
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            },
        }))
    }

    /// Handle the `tools/list` method.
    fn handle_tools_list(&self) -> Result<Value, (i32, String)> {
        let tools: Vec<Value> = self
            .tool_registry
            .list()
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();

        Ok(json!({ "tools": tools }))
    }

    /// Handle the `tools/call` method.
    async fn handle_tools_call(&self, params: Value) -> Result<Value, (i32, String)> {
        let name = params["name"]
            .as_str()
            .ok_or((-32602, "Missing tool name".to_string()))?;

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        // Check that the tool exists.
        if self.tool_registry.get(name).is_none() {
            return Err((-32602, format!("Unknown tool: {}", name)));
        }

        // Dispatch to the appropriate handler.
        let result = match name {
            // ── Cell operations ──────────────────────────────────────────
            "read_cell" => {
                let wb = self.workbook.read().await;
                cell_ops::handle_read_cell(&wb, arguments)
            }
            "write_cell" => {
                let mut wb = self.workbook.write().await;
                cell_ops::handle_write_cell(&mut wb, arguments)
            }
            "read_range" => {
                let wb = self.workbook.read().await;
                cell_ops::handle_read_range(&wb, arguments)
            }
            "write_range" => {
                let mut wb = self.workbook.write().await;
                cell_ops::handle_write_range(&mut wb, arguments)
            }

            // ── Sheet operations ─────────────────────────────────────────
            "list_sheets" => {
                let wb = self.workbook.read().await;
                sheet_ops::handle_list_sheets(&wb)
            }
            "create_sheet" => {
                let mut wb = self.workbook.write().await;
                sheet_ops::handle_create_sheet(&mut wb, arguments)
            }
            "rename_sheet" => {
                let mut wb = self.workbook.write().await;
                sheet_ops::handle_rename_sheet(&mut wb, arguments)
            }
            "delete_sheet" => {
                let mut wb = self.workbook.write().await;
                sheet_ops::handle_delete_sheet(&mut wb, arguments)
            }

            // ── Data operations ──────────────────────────────────────────
            "clear_range" => {
                let mut wb = self.workbook.write().await;
                data_ops::handle_clear_range(&mut wb, arguments)
            }
            "find_replace" => {
                let mut wb = self.workbook.write().await;
                data_ops::handle_find_replace(&mut wb, arguments)
            }
            "sort_range" => {
                let mut wb = self.workbook.write().await;
                data_ops::handle_sort_range(&mut wb, arguments)
            }
            "deduplicate" => {
                let mut wb = self.workbook.write().await;
                data_ops::handle_deduplicate(&mut wb, arguments)
            }
            "transpose" => {
                let mut wb = self.workbook.write().await;
                data_ops::handle_transpose(&mut wb, arguments)
            }


            // ── Analysis operations ──────────────────────────────────────
            "describe_data" => {
                let wb = self.workbook.read().await;
                analysis::handle_describe_data(&wb, arguments)
            }
            "correlate" => {
                let wb = self.workbook.read().await;
                analysis::handle_correlate(&wb, arguments)
            }
            "trend_analysis" => {
                let wb = self.workbook.read().await;
                analysis::handle_trend_analysis(&wb, arguments)
            }

            // ── Chart operations ─────────────────────────────────────────
            "create_chart" => chart_ops::handle_create_chart(arguments),
            "list_charts" => chart_ops::handle_list_charts(arguments),
            "delete_chart" => chart_ops::handle_delete_chart(arguments),

            // ── File operations ──────────────────────────────────────────
            "get_workbook_info" => {
                let wb = self.workbook.read().await;
                file_ops::handle_get_workbook_info(&wb)
            }
            "export_json" => {
                let wb = self.workbook.read().await;
                file_ops::handle_export_json(&wb, arguments)
            }
            "export_csv" => {
                let wb = self.workbook.read().await;
                file_ops::handle_export_csv(&wb, arguments)
            }

            // Catch-all for registered but unimplemented tools.
            _ => Err(format!("Tool '{}' is not yet implemented", name)),
        };

        match result {
            Ok(value) => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value)
                        .unwrap_or_else(|_| value.to_string()),
                }],
                "isError": false,
            })),
            Err(msg) => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": msg,
                }],
                "isError": true,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // We should have 20+ tools (format_ops and formula_ops not yet implemented).
        assert!(
            tools.len() >= 20,
            "Expected at least 20 tools, got {}",
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
        // format_ops and formula_ops tools not yet implemented
        assert!(tool_names.contains(&"describe_data"));
        assert!(tool_names.contains(&"correlate"));
        assert!(tool_names.contains(&"trend_analysis"));
        assert!(tool_names.contains(&"create_chart"));
        assert!(tool_names.contains(&"list_charts"));
        assert!(tool_names.contains(&"delete_chart"));
        assert!(tool_names.contains(&"get_workbook_info"));
        assert!(tool_names.contains(&"export_json"));
        assert!(tool_names.contains(&"export_csv"));
        // merge_cells and unmerge_cells are in format_ops (not yet implemented)
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
        // evaluate_formula tool is not yet implemented (formula_ops module pending).
        // Calling it should return a JSON-RPC error since it's not in the registry.
        let mut server = McpServer::new_default();

        let response = server
            .handle_message(
                r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"evaluate_formula","arguments":{"formula":"SUM(A1:A2)"}}}"#,
            )
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["error"]["code"], -32602);
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
        assert_eq!(prompts.len(), 5);
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
