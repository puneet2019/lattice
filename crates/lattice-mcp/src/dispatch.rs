//! Sync JSON-RPC 2.0 dispatcher for the MCP protocol.
//!
//! This module is the heart of the MCP server, independent of any
//! transport or async runtime. It can be driven directly by:
//!
//! - The native `McpServer` (which acquires `tokio::RwLock` guards
//!   and then calls into here), and
//! - A WASM host such as `lattice-wasm`, which lives in a
//!   single-threaded JS context and needs a sync entry point.
//!
//! No tokio, no `Arc`, no locks. Callers own the workbook and pass it
//! in as `&mut Workbook` via [`McpState`].

use serde_json::{Value, json};

use lattice_core::{ConditionalFormatStore, Workbook};

use crate::tools::ToolRegistry;
use crate::tools::{
    analysis, cell_ops, chart_ops, conditional_format_ops, data_ops, file_ops, filter_view_ops,
    find_replace_ops, format_ops, formula_ops, named_function_ops, named_range_ops, sheet_ops,
    sparkline_ops, validation_ops,
};

/// The MCP protocol version we implement.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// The server name reported during initialization.
pub const SERVER_NAME: &str = "lattice";

/// The server version reported during initialization.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Mutable state borrowed by [`handle_request`].
///
/// Holds the workbook, the conditional-format store, the
/// initialization flag, and the tool registry. All of these are
/// borrowed for the duration of a single request, which is fine
/// because dispatch is sync.
pub struct McpState<'a> {
    /// The workbook being operated on.
    pub workbook: &'a mut Workbook,
    /// Conditional formatting store (kept outside the workbook, same as
    /// the desktop Tauri layer).
    pub conditional_formats: &'a mut ConditionalFormatStore,
    /// Whether the server has been initialized (set by `initialize`).
    pub initialized: &'a mut bool,
    /// Registry used to validate tool names and produce `tools/list`.
    pub tool_registry: &'a ToolRegistry,
}

/// Dispatch a single JSON-RPC 2.0 request and return the response, or
/// `None` for notifications (requests with no `id`).
///
/// This is the sync core of the MCP server — it is safe to call from a
/// wasm32 single-threaded context, and the native `McpServer` delegates
/// to it after acquiring its `tokio::RwLock` guards.
pub fn handle_request(state: &mut McpState<'_>, message: &str) -> Option<String> {
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
        "initialize" => handle_initialize(state, params),
        "initialized" => {
            // Notification — no response needed.
            return None;
        }
        "ping" => Ok(json!({})),
        "tools/list" => handle_tools_list(state),
        "tools/call" => handle_tools_call(state, params),
        "resources/list" => crate::resources::handle_list_resources(),
        "resources/read" => crate::resources::handle_read_resource(params, state.workbook),
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

/// Handle `initialize`. Flips the `initialized` flag and returns the
/// server capabilities.
fn handle_initialize(state: &mut McpState<'_>, _params: Value) -> Result<Value, (i32, String)> {
    *state.initialized = true;

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

/// Handle `tools/list` by rendering the registry as a JSON array.
fn handle_tools_list(state: &McpState<'_>) -> Result<Value, (i32, String)> {
    let tools: Vec<Value> = state
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

/// Handle `tools/call` by routing to the matching tool handler.
///
/// Mirrors the dispatch arms from the original `McpServer::handle_tools_call`,
/// but operates synchronously on borrowed state.
fn handle_tools_call(state: &mut McpState<'_>, params: Value) -> Result<Value, (i32, String)> {
    let name = params["name"]
        .as_str()
        .ok_or((-32602, "Missing tool name".to_string()))?;

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    if state.tool_registry.get(name).is_none() {
        return Err((-32602, format!("Unknown tool: {}", name)));
    }

    // Reborrow each piece of state we mutate from the borrowed `McpState`,
    // so the match arms below can take `&mut Workbook` /
    // `&mut ConditionalFormatStore` independently.
    let workbook: &mut Workbook = &mut *state.workbook;
    let conditional_formats: &mut ConditionalFormatStore = &mut *state.conditional_formats;

    let result = match name {
        // ── Cell operations ──────────────────────────────────────────
        "read_cell" => cell_ops::handle_read_cell(workbook, arguments),
        "write_cell" => cell_ops::handle_write_cell(workbook, arguments),
        "read_range" => cell_ops::handle_read_range(workbook, arguments),
        "write_range" => cell_ops::handle_write_range(workbook, arguments),

        // ── Sheet operations ─────────────────────────────────────────
        "list_sheets" => sheet_ops::handle_list_sheets(workbook),
        "create_sheet" => sheet_ops::handle_create_sheet(workbook, arguments),
        "rename_sheet" => sheet_ops::handle_rename_sheet(workbook, arguments),
        "delete_sheet" => sheet_ops::handle_delete_sheet(workbook, arguments),
        "hide_rows" => sheet_ops::handle_hide_rows(workbook, arguments),
        "unhide_rows" => sheet_ops::handle_unhide_rows(workbook, arguments),
        "hide_cols" => sheet_ops::handle_hide_cols(workbook, arguments),
        "unhide_cols" => sheet_ops::handle_unhide_cols(workbook, arguments),
        "protect_sheet" => sheet_ops::handle_protect_sheet(workbook, arguments),
        "unprotect_sheet" => sheet_ops::handle_unprotect_sheet(workbook, arguments),
        "set_sheet_tab_color" => sheet_ops::handle_set_sheet_tab_color(workbook, arguments),

        // ── Data operations ──────────────────────────────────────────
        "clear_range" => data_ops::handle_clear_range(workbook, arguments),
        "find_replace" => data_ops::handle_find_replace(workbook, arguments),
        "sort_range" => data_ops::handle_sort_range(workbook, arguments),
        "deduplicate" => data_ops::handle_deduplicate(workbook, arguments),
        "transpose" => data_ops::handle_transpose(workbook, arguments),
        "auto_fill" => data_ops::handle_auto_fill(workbook, arguments),
        "generate_pivot" => data_ops::handle_generate_pivot(workbook, arguments),
        "remove_duplicates" => data_ops::handle_remove_duplicates(workbook, arguments),
        "text_to_columns" => data_ops::handle_text_to_columns(workbook, arguments),

        // ── Find/replace operations (core-backed) ───────────────────
        "find_in_workbook" => find_replace_ops::handle_find_in_workbook(workbook, arguments),
        "replace_in_workbook" => find_replace_ops::handle_replace_in_workbook(workbook, arguments),

        // ── Named range operations ───────────────────────────────────
        "add_named_range" => named_range_ops::handle_add_named_range(workbook, arguments),
        "remove_named_range" => named_range_ops::handle_remove_named_range(workbook, arguments),
        "list_named_ranges" => named_range_ops::handle_list_named_ranges(workbook),
        "resolve_named_range" => named_range_ops::handle_resolve_named_range(workbook, arguments),

        // ── Named function operations ────────────────────────────────
        "add_named_function" => named_function_ops::handle_add_named_function(workbook, arguments),
        "remove_named_function" => {
            named_function_ops::handle_remove_named_function(workbook, arguments)
        }
        "list_named_functions" => named_function_ops::handle_list_named_functions(workbook),

        // ── Analysis operations ──────────────────────────────────────
        "describe_data" => analysis::handle_describe_data(workbook, arguments),
        "correlate" => analysis::handle_correlate(workbook, arguments),
        "trend_analysis" => analysis::handle_trend_analysis(workbook, arguments),

        // ── Format operations ────────────────────────────────────────
        "get_cell_format" => format_ops::handle_get_cell_format(workbook, arguments),
        "set_cell_format" => format_ops::handle_set_cell_format(workbook, arguments),
        "merge_cells" => format_ops::handle_merge_cells(workbook, arguments),
        "unmerge_cells" => format_ops::handle_unmerge_cells(workbook, arguments),

        // ── Formula operations ───────────────────────────────────────
        "evaluate_formula" => formula_ops::handle_evaluate_formula(workbook, arguments),
        "get_formula" => formula_ops::handle_get_formula(workbook, arguments),
        "insert_formula" => formula_ops::handle_insert_formula(workbook, arguments),
        "bulk_formula" => formula_ops::handle_bulk_formula(workbook, arguments),
        "import_range" => formula_ops::handle_import_range(arguments),

        // ── Validation operations ────────────────────────────────────
        "set_validation" => validation_ops::handle_set_validation(workbook, arguments),
        "get_validation" => validation_ops::handle_get_validation(workbook, arguments),
        "remove_validation" => validation_ops::handle_remove_validation(workbook, arguments),
        "validate_cell" => validation_ops::handle_validate_cell(workbook, arguments),

        // ── Chart operations ─────────────────────────────────────────
        "create_chart" => chart_ops::handle_create_chart(arguments),
        "list_charts" => chart_ops::handle_list_charts(arguments),
        "delete_chart" => chart_ops::handle_delete_chart(arguments),

        // ── Conditional format operations ────────────────────────────
        "add_conditional_format" => {
            conditional_format_ops::handle_add_conditional_format(conditional_formats, arguments)
        }
        "list_conditional_formats" => {
            conditional_format_ops::handle_list_conditional_formats(conditional_formats, arguments)
        }
        "remove_conditional_format" => {
            conditional_format_ops::handle_remove_conditional_format(conditional_formats, arguments)
        }

        // ── Sparkline operations ─────────────────────────────────────
        "add_sparkline" => sparkline_ops::handle_add_sparkline(workbook, arguments),
        "remove_sparkline" => sparkline_ops::handle_remove_sparkline(workbook, arguments),
        "list_sparklines" => sparkline_ops::handle_list_sparklines(workbook, arguments),

        // ── Filter view operations ───────────────────────────────────
        "save_filter_view" => filter_view_ops::handle_save_filter_view(workbook, arguments),
        "list_filter_views" => filter_view_ops::handle_list_filter_views(workbook),
        "apply_filter_view" => filter_view_ops::handle_apply_filter_view(workbook, arguments),
        "delete_filter_view" => filter_view_ops::handle_delete_filter_view(workbook, arguments),

        // ── File operations ──────────────────────────────────────────
        "get_workbook_info" => file_ops::handle_get_workbook_info(workbook),
        "export_json" => file_ops::handle_export_json(workbook, arguments),
        "export_csv" => file_ops::handle_export_csv(workbook, arguments),

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
