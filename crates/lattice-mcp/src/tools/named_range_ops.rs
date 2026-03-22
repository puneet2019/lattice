//! Named range MCP tool handlers.
//!
//! Provides tools to add, remove, list, and resolve named ranges
//! via the workbook's `NamedRangeStore`.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellRef, Range, Workbook, col_to_letter};

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for named range operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "add_named_range".to_string(),
            description: "Create a named range mapping a name to a cell range. Names must start with a letter or underscore.".to_string(),
            input_schema: object_schema(
                &[
                    ("name", string_prop("Name for the range (e.g. 'Revenue')")),
                    ("range", string_prop("Cell range in A1:B2 notation")),
                    ("sheet", string_prop("Sheet scope (omit for workbook-scoped)")),
                ],
                &["name", "range"],
            ),
        },
        ToolDef {
            name: "remove_named_range".to_string(),
            description: "Delete a named range by name (case-insensitive).".to_string(),
            input_schema: object_schema(
                &[
                    ("name", string_prop("Name of the range to remove")),
                ],
                &["name"],
            ),
        },
        ToolDef {
            name: "list_named_ranges".to_string(),
            description: "List all named ranges in the workbook.".to_string(),
            input_schema: object_schema(&[], &[]),
        },
        ToolDef {
            name: "resolve_named_range".to_string(),
            description: "Resolve a named range to its sheet and cell range.".to_string(),
            input_schema: object_schema(
                &[
                    ("name", string_prop("Name of the range to resolve")),
                ],
                &["name"],
            ),
        },
    ]
}

/// Parse a range string like "A1:B5" into a `Range`.
fn parse_range(range_str: &str) -> Result<Range, String> {
    let parts: Vec<&str> = range_str.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid range format '{}': expected 'A1:B2'",
            range_str
        ));
    }
    let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
    let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
    Ok(Range { start, end })
}

/// Format a `Range` back to A1:B2 notation.
fn format_range(range: &Range) -> String {
    format!(
        "{}{}:{}{}",
        col_to_letter(range.start.col),
        range.start.row + 1,
        col_to_letter(range.end.col),
        range.end.row + 1,
    )
}

#[derive(Debug, Deserialize)]
struct AddNamedRangeArgs {
    name: String,
    range: String,
    sheet: Option<String>,
}

/// Handle the `add_named_range` tool call.
pub fn handle_add_named_range(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: AddNamedRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let range = parse_range(&args.range)?;

    workbook
        .named_ranges
        .add(&args.name, args.sheet.clone(), range)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
        "range": args.range,
        "sheet": args.sheet,
    }))
}

#[derive(Debug, Deserialize)]
struct RemoveNamedRangeArgs {
    name: String,
}

/// Handle the `remove_named_range` tool call.
pub fn handle_remove_named_range(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: RemoveNamedRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    workbook
        .named_ranges
        .remove(&args.name)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
    }))
}

/// Handle the `list_named_ranges` tool call.
pub fn handle_list_named_ranges(workbook: &Workbook) -> Result<Value, String> {
    let ranges: Vec<Value> = workbook
        .named_ranges
        .list()
        .iter()
        .map(|nr| {
            json!({
                "name": nr.name,
                "range": format_range(&nr.range),
                "sheet": nr.sheet,
            })
        })
        .collect();

    Ok(json!({
        "count": ranges.len(),
        "named_ranges": ranges,
    }))
}

#[derive(Debug, Deserialize)]
struct ResolveNamedRangeArgs {
    name: String,
}

/// Handle the `resolve_named_range` tool call.
pub fn handle_resolve_named_range(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: ResolveNamedRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    match workbook.named_ranges.resolve(&args.name) {
        Some((sheet, range)) => Ok(json!({
            "found": true,
            "name": args.name,
            "sheet": sheet,
            "range": format_range(range),
        })),
        None => Err(format!("Named range '{}' not found", args.name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_named_range() {
        let mut wb = Workbook::new();

        let result = handle_add_named_range(
            &mut wb,
            json!({"name": "Revenue", "range": "A1:A10"}),
        ).unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["name"], "Revenue");
        assert!(wb.named_ranges.get("Revenue").is_some());
    }

    #[test]
    fn test_add_named_range_with_sheet() {
        let mut wb = Workbook::new();

        let result = handle_add_named_range(
            &mut wb,
            json!({"name": "Sales", "range": "B2:D20", "sheet": "Data"}),
        ).unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["sheet"], "Data");
    }

    #[test]
    fn test_add_named_range_duplicate() {
        let mut wb = Workbook::new();

        handle_add_named_range(
            &mut wb,
            json!({"name": "Revenue", "range": "A1:A10"}),
        ).unwrap();

        let result = handle_add_named_range(
            &mut wb,
            json!({"name": "Revenue", "range": "B1:B10"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_add_named_range_invalid_name() {
        let mut wb = Workbook::new();

        let result = handle_add_named_range(
            &mut wb,
            json!({"name": "1bad", "range": "A1:A10"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_named_range() {
        let mut wb = Workbook::new();

        handle_add_named_range(
            &mut wb,
            json!({"name": "Revenue", "range": "A1:A10"}),
        ).unwrap();

        let result = handle_remove_named_range(
            &mut wb,
            json!({"name": "Revenue"}),
        ).unwrap();

        assert_eq!(result["success"], true);
        assert!(wb.named_ranges.get("Revenue").is_none());
    }

    #[test]
    fn test_remove_named_range_not_found() {
        let mut wb = Workbook::new();

        let result = handle_remove_named_range(
            &mut wb,
            json!({"name": "Nothing"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_list_named_ranges() {
        let mut wb = Workbook::new();

        handle_add_named_range(
            &mut wb,
            json!({"name": "Alpha", "range": "A1:A5"}),
        ).unwrap();
        handle_add_named_range(
            &mut wb,
            json!({"name": "Beta", "range": "B1:B10", "sheet": "Sheet1"}),
        ).unwrap();

        let result = handle_list_named_ranges(&wb).unwrap();

        assert_eq!(result["count"], 2);
        let ranges = result["named_ranges"].as_array().unwrap();
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_list_named_ranges_empty() {
        let wb = Workbook::new();
        let result = handle_list_named_ranges(&wb).unwrap();
        assert_eq!(result["count"], 0);
    }

    #[test]
    fn test_resolve_named_range() {
        let mut wb = Workbook::new();

        handle_add_named_range(
            &mut wb,
            json!({"name": "Sales", "range": "C1:C100", "sheet": "Data"}),
        ).unwrap();

        let result = handle_resolve_named_range(
            &wb,
            json!({"name": "sales"}),  // case-insensitive
        ).unwrap();

        assert_eq!(result["found"], true);
        assert_eq!(result["sheet"], "Data");
        assert_eq!(result["range"], "C1:C100");
    }

    #[test]
    fn test_resolve_named_range_not_found() {
        let wb = Workbook::new();

        let result = handle_resolve_named_range(
            &wb,
            json!({"name": "Nothing"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_named_range_workbook_scoped() {
        let mut wb = Workbook::new();

        handle_add_named_range(
            &mut wb,
            json!({"name": "Total", "range": "A1:A1"}),
        ).unwrap();

        let result = handle_resolve_named_range(
            &wb,
            json!({"name": "Total"}),
        ).unwrap();

        assert_eq!(result["found"], true);
        assert!(result["sheet"].is_null());
    }
}
