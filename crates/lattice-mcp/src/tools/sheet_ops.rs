//! Sheet operation tool handlers: list_sheets, create_sheet, rename_sheet, delete_sheet.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::Workbook;

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for sheet operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "list_sheets".to_string(),
            description: "List all sheets in the workbook with their metadata".to_string(),
            input_schema: object_schema(&[], &[]),
        },
        ToolDef {
            name: "create_sheet".to_string(),
            description: "Create a new empty sheet".to_string(),
            input_schema: object_schema(
                &[("name", string_prop("Name for the new sheet"))],
                &["name"],
            ),
        },
        ToolDef {
            name: "rename_sheet".to_string(),
            description: "Rename an existing sheet".to_string(),
            input_schema: object_schema(
                &[
                    ("old_name", string_prop("Current sheet name")),
                    ("new_name", string_prop("New sheet name")),
                ],
                &["old_name", "new_name"],
            ),
        },
        ToolDef {
            name: "delete_sheet".to_string(),
            description: "Delete a sheet from the workbook".to_string(),
            input_schema: object_schema(
                &[("name", string_prop("Sheet name to delete"))],
                &["name"],
            ),
        },
    ]
}

/// Handle the `list_sheets` tool call.
pub fn handle_list_sheets(workbook: &Workbook) -> std::result::Result<Value, String> {
    let names = workbook.sheet_names();
    let mut sheets = Vec::new();

    for name in &names {
        let sheet = workbook.get_sheet(name).map_err(|e| e.to_string())?;
        let (max_row, max_col) = sheet.used_range();
        let cell_count = sheet.cells().len();

        sheets.push(json!({
            "name": name,
            "used_range": {
                "rows": if cell_count == 0 { 0 } else { max_row + 1 },
                "cols": if cell_count == 0 { 0 } else { max_col + 1 },
            },
            "cell_count": cell_count,
            "is_active": *name == workbook.active_sheet,
        }));
    }

    Ok(json!({
        "sheets": sheets,
        "count": names.len(),
        "active_sheet": workbook.active_sheet,
    }))
}

/// Arguments for create_sheet.
#[derive(Debug, Deserialize)]
pub struct CreateSheetArgs {
    pub name: String,
}

/// Handle the `create_sheet` tool call.
pub fn handle_create_sheet(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: CreateSheetArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    workbook.add_sheet(&args.name).map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "sheet_name": args.name,
    }))
}

/// Arguments for rename_sheet.
#[derive(Debug, Deserialize)]
pub struct RenameSheetArgs {
    pub old_name: String,
    pub new_name: String,
}

/// Handle the `rename_sheet` tool call.
pub fn handle_rename_sheet(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: RenameSheetArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    workbook
        .rename_sheet(&args.old_name, &args.new_name)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "old_name": args.old_name,
        "new_name": args.new_name,
    }))
}

/// Arguments for delete_sheet.
#[derive(Debug, Deserialize)]
pub struct DeleteSheetArgs {
    pub name: String,
}

/// Handle the `delete_sheet` tool call.
pub fn handle_delete_sheet(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: DeleteSheetArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    workbook
        .remove_sheet(&args.name)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "deleted_sheet": args.name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_list_sheets() {
        let wb = Workbook::new();
        let result = handle_list_sheets(&wb).unwrap();
        assert_eq!(result["count"], 1);
        assert_eq!(result["sheets"][0]["name"], "Sheet1");
    }

    #[test]
    fn test_handle_create_sheet() {
        let mut wb = Workbook::new();
        let result = handle_create_sheet(&mut wb, json!({"name": "Data"})).unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(wb.sheet_names().len(), 2);
    }

    #[test]
    fn test_handle_rename_sheet() {
        let mut wb = Workbook::new();
        let result = handle_rename_sheet(
            &mut wb,
            json!({"old_name": "Sheet1", "new_name": "Summary"}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(wb.sheet_names(), vec!["Summary"]);
    }

    #[test]
    fn test_handle_delete_sheet() {
        let mut wb = Workbook::new();
        wb.add_sheet("Extra").unwrap();
        let result = handle_delete_sheet(&mut wb, json!({"name": "Extra"})).unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(wb.sheet_names().len(), 1);
    }
}
