//! MCP resource endpoints — stub implementation.
//!
//! Resources provide read-only data access to the workbook.

use serde_json::{Value, json};

/// Handle the `resources/list` method.
///
/// Returns the list of available resource URIs.
pub fn handle_list_resources() -> Result<Value, (i32, String)> {
    Ok(json!({
        "resources": [
            {
                "uri": "lattice://workbook/info",
                "name": "Workbook Info",
                "description": "Workbook metadata (filename, sheets, modified, size)",
                "mimeType": "application/json",
            },
            {
                "uri": "lattice://charts",
                "name": "Charts",
                "description": "List of all charts in the workbook",
                "mimeType": "application/json",
            },
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "lattice://sheet/{name}/data",
                "name": "Sheet Data",
                "description": "Full sheet data as JSON",
                "mimeType": "application/json",
            },
            {
                "uriTemplate": "lattice://sheet/{name}/range/{range}",
                "name": "Sheet Range",
                "description": "Specific range data",
                "mimeType": "application/json",
            },
            {
                "uriTemplate": "lattice://sheet/{name}/summary",
                "name": "Sheet Summary",
                "description": "Auto-generated data summary",
                "mimeType": "application/json",
            },
        ],
    }))
}

/// Handle the `resources/read` method.
///
/// Reads a resource by URI. Currently returns stubs.
pub fn handle_read_resource(params: Value) -> Result<Value, (i32, String)> {
    let uri = params["uri"]
        .as_str()
        .ok_or((-32602, "Missing 'uri' parameter".to_string()))?;

    match uri {
        "lattice://workbook/info" => Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&json!({
                    "filename": null,
                    "sheets": [],
                    "modified": false,
                })).unwrap(),
            }],
        })),
        _ => Err((-32002, format!("Resource not found: {}", uri))),
    }
}
