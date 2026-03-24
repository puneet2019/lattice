//! Filter view MCP tool handlers.
//!
//! Provides tools to save, list, apply, and delete named filter views
//! via the workbook's `FilterViewStore`.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::Workbook;

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for filter view operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "save_filter_view".to_string(),
            description: "Save a named filter view with column filter criteria. Each column maps to a list of allowed values.".to_string(),
            input_schema: object_schema(
                &[
                    ("name", string_prop("Name for the filter view")),
                    ("column_filters", json!({
                        "type": "object",
                        "description": "Map of column index (as string) to array of allowed values",
                        "additionalProperties": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    })),
                ],
                &["name", "column_filters"],
            ),
        },
        ToolDef {
            name: "list_filter_views".to_string(),
            description: "List all saved filter views in the workbook.".to_string(),
            input_schema: object_schema(&[], &[]),
        },
        ToolDef {
            name: "apply_filter_view".to_string(),
            description: "Apply a saved filter view to a sheet, hiding rows that do not match.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name to apply the filter to")),
                    ("name", string_prop("Name of the filter view to apply")),
                ],
                &["sheet", "name"],
            ),
        },
        ToolDef {
            name: "delete_filter_view".to_string(),
            description: "Delete a saved filter view by name.".to_string(),
            input_schema: object_schema(
                &[("name", string_prop("Name of the filter view to delete"))],
                &["name"],
            ),
        },
    ]
}

#[derive(Debug, Deserialize)]
struct SaveFilterViewArgs {
    name: String,
    column_filters: HashMap<String, Vec<String>>,
}

/// Handle the `save_filter_view` tool call.
pub fn handle_save_filter_view(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: SaveFilterViewArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    // Convert string keys to u32 column indices.
    let mut filters: HashMap<u32, Vec<String>> = HashMap::new();
    for (key, values) in args.column_filters {
        let col: u32 = key
            .parse()
            .map_err(|_| format!("Invalid column index: '{key}'"))?;
        filters.insert(col, values);
    }

    workbook
        .filter_views
        .add(&args.name, filters)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
    }))
}

/// Handle the `list_filter_views` tool call.
pub fn handle_list_filter_views(workbook: &Workbook) -> Result<Value, String> {
    let views: Vec<Value> = workbook
        .filter_views
        .list()
        .iter()
        .map(|v| {
            let col_filters: HashMap<String, &Vec<String>> = v
                .column_filters
                .iter()
                .map(|(k, vals)| (k.to_string(), vals))
                .collect();
            json!({
                "name": v.name,
                "column_filters": col_filters,
            })
        })
        .collect();

    Ok(json!({
        "count": views.len(),
        "filter_views": views,
    }))
}

#[derive(Debug, Deserialize)]
struct ApplyFilterViewArgs {
    sheet: String,
    name: String,
}

/// Handle the `apply_filter_view` tool call.
pub fn handle_apply_filter_view(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: ApplyFilterViewArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    // Clone the view first to avoid borrowing issues.
    let view = workbook
        .filter_views
        .get(&args.name)
        .cloned()
        .ok_or_else(|| format!("filter view '{}' not found", args.name))?;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    let hidden =
        lattice_core::filter_view::apply_filter_view(sheet, &view).map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
        "sheet": args.sheet,
        "rows_hidden": hidden,
    }))
}

#[derive(Debug, Deserialize)]
struct DeleteFilterViewArgs {
    name: String,
}

/// Handle the `delete_filter_view` tool call.
pub fn handle_delete_filter_view(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: DeleteFilterViewArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    workbook
        .filter_views
        .remove(&args.name)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_save_and_list_filter_views() {
        let mut wb = Workbook::new();
        let result = handle_save_filter_view(
            &mut wb,
            json!({"name": "MyView", "column_filters": {"0": ["apple", "banana"]}}),
        )
        .unwrap();
        assert_eq!(result["success"], true);

        let list = handle_list_filter_views(&wb).unwrap();
        assert_eq!(list["count"], 1);
        assert_eq!(list["filter_views"][0]["name"], "MyView");
    }

    #[test]
    fn test_apply_filter_view() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Name".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("apple".into()))
            .unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("banana".into()))
            .unwrap();

        handle_save_filter_view(
            &mut wb,
            json!({"name": "Apples", "column_filters": {"0": ["apple"]}}),
        )
        .unwrap();

        let result =
            handle_apply_filter_view(&mut wb, json!({"sheet": "Sheet1", "name": "Apples"}))
                .unwrap();
        assert_eq!(result["rows_hidden"], 1);
    }

    #[test]
    fn test_delete_filter_view() {
        let mut wb = Workbook::new();
        handle_save_filter_view(&mut wb, json!({"name": "Test", "column_filters": {}})).unwrap();

        let result = handle_delete_filter_view(&mut wb, json!({"name": "Test"})).unwrap();
        assert_eq!(result["success"], true);

        let list = handle_list_filter_views(&wb).unwrap();
        assert_eq!(list["count"], 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut wb = Workbook::new();
        let result = handle_delete_filter_view(&mut wb, json!({"name": "nope"}));
        assert!(result.is_err());
    }
}
