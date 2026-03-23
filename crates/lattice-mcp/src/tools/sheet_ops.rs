//! Sheet operation tool handlers: list_sheets, create_sheet, rename_sheet, delete_sheet,
//! hide_rows, unhide_rows, hide_cols, unhide_cols, protect_sheet, unprotect_sheet,
//! set_sheet_tab_color.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::Workbook;

use super::ToolDef;
use crate::schema::{number_prop, object_schema, string_prop};

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
        ToolDef {
            name: "hide_rows".to_string(),
            description: "Hide a range of rows in a sheet".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("start_row", number_prop("Start row (1-based)")),
                    ("count", number_prop("Number of rows to hide")),
                ],
                &["sheet", "start_row", "count"],
            ),
        },
        ToolDef {
            name: "unhide_rows".to_string(),
            description: "Unhide a range of previously hidden rows".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("start_row", number_prop("Start row (1-based)")),
                    ("count", number_prop("Number of rows to unhide")),
                ],
                &["sheet", "start_row", "count"],
            ),
        },
        ToolDef {
            name: "hide_cols".to_string(),
            description: "Hide a range of columns in a sheet".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("start_col", string_prop("Start column letter (e.g. 'A')")),
                    ("count", number_prop("Number of columns to hide")),
                ],
                &["sheet", "start_col", "count"],
            ),
        },
        ToolDef {
            name: "unhide_cols".to_string(),
            description: "Unhide a range of previously hidden columns".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("start_col", string_prop("Start column letter (e.g. 'A')")),
                    ("count", number_prop("Number of columns to unhide")),
                ],
                &["sheet", "start_col", "count"],
            ),
        },
        ToolDef {
            name: "protect_sheet".to_string(),
            description: "Protect a sheet to prevent editing. Optionally set a password.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("password", string_prop("Optional password to protect the sheet")),
                ],
                &["sheet"],
            ),
        },
        ToolDef {
            name: "unprotect_sheet".to_string(),
            description: "Remove protection from a sheet. Supply the password if it was protected with one.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("password", string_prop("Password used when protecting (if any)")),
                ],
                &["sheet"],
            ),
        },
        ToolDef {
            name: "set_sheet_tab_color".to_string(),
            description: "Set or clear the tab color for a sheet".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("color", string_prop("CSS hex color (e.g. '#FF0000'), or null to clear")),
                ],
                &["sheet"],
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

// ── Hide / Unhide Rows ────────────────────────────────────────────────────

/// Arguments for hide_rows / unhide_rows.
#[derive(Debug, Deserialize)]
pub struct HideRowsArgs {
    pub sheet: String,
    pub start_row: u32,
    pub count: u32,
}

/// Handle the `hide_rows` tool call.
pub fn handle_hide_rows(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: HideRowsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
    if args.start_row == 0 {
        return Err("start_row must be 1-based (minimum 1)".to_string());
    }
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    sheet.hide_rows(args.start_row - 1, args.count);
    Ok(json!({ "success": true, "rows_hidden": args.count }))
}

/// Handle the `unhide_rows` tool call.
pub fn handle_unhide_rows(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: HideRowsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
    if args.start_row == 0 {
        return Err("start_row must be 1-based (minimum 1)".to_string());
    }
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    sheet.unhide_rows(args.start_row - 1, args.count);
    Ok(json!({ "success": true, "rows_unhidden": args.count }))
}

// ── Hide / Unhide Cols ────────────────────────────────────────────────────

/// Arguments for hide_cols / unhide_cols.
#[derive(Debug, Deserialize)]
pub struct HideColsArgs {
    pub sheet: String,
    pub start_col: String,
    pub count: u32,
}

/// Handle the `hide_cols` tool call.
pub fn handle_hide_cols(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: HideColsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
    let col = lattice_core::CellRef::parse(&format!("{}1", args.start_col))
        .map_err(|e| format!("Invalid column '{}': {}", args.start_col, e))?;
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    sheet.hide_cols(col.col, args.count);
    Ok(json!({ "success": true, "cols_hidden": args.count }))
}

/// Handle the `unhide_cols` tool call.
pub fn handle_unhide_cols(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: HideColsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
    let col = lattice_core::CellRef::parse(&format!("{}1", args.start_col))
        .map_err(|e| format!("Invalid column '{}': {}", args.start_col, e))?;
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    sheet.unhide_cols(col.col, args.count);
    Ok(json!({ "success": true, "cols_unhidden": args.count }))
}

// ── Protect / Unprotect ───────────────────────────────────────────────────

/// Arguments for protect_sheet.
#[derive(Debug, Deserialize)]
pub struct ProtectSheetArgs {
    pub sheet: String,
    pub password: Option<String>,
}

/// Handle the `protect_sheet` tool call.
pub fn handle_protect_sheet(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: ProtectSheetArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    sheet.protect(args.password.as_deref());
    Ok(json!({
        "success": true,
        "sheet": args.sheet,
        "has_password": args.password.is_some(),
    }))
}

/// Arguments for unprotect_sheet.
#[derive(Debug, Deserialize)]
pub struct UnprotectSheetArgs {
    pub sheet: String,
    pub password: Option<String>,
}

/// Handle the `unprotect_sheet` tool call.
pub fn handle_unprotect_sheet(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: UnprotectSheetArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    sheet
        .unprotect(args.password.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "success": true,
        "sheet": args.sheet,
    }))
}

// ── Tab Color ─────────────────────────────────────────────────────────────

/// Arguments for set_sheet_tab_color.
#[derive(Debug, Deserialize)]
pub struct SetTabColorArgs {
    pub sheet: String,
    pub color: Option<String>,
}

/// Handle the `set_sheet_tab_color` tool call.
pub fn handle_set_sheet_tab_color(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    // Parse manually to support null color (clear).
    let raw = args
        .as_object()
        .ok_or("arguments must be a JSON object")?;
    let sheet_name = raw
        .get("sheet")
        .and_then(|v| v.as_str())
        .ok_or("Missing required 'sheet' argument")?;
    let color = match raw.get("color") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Null) | None => None,
        _ => return Err("'color' must be a string or null".to_string()),
    };
    let sheet = workbook
        .get_sheet_mut(sheet_name)
        .map_err(|e| e.to_string())?;
    sheet.set_tab_color(color.clone());
    Ok(json!({
        "success": true,
        "sheet": sheet_name,
        "tab_color": color,
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

    #[test]
    fn test_hide_and_unhide_rows() {
        let mut wb = Workbook::new();
        let result = handle_hide_rows(
            &mut wb,
            json!({"sheet": "Sheet1", "start_row": 2, "count": 3}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["rows_hidden"], 3);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(sheet.is_row_hidden(1)); // 0-based row 1 = user row 2
        assert!(sheet.is_row_hidden(2));
        assert!(sheet.is_row_hidden(3));
        assert!(!sheet.is_row_hidden(0));

        let result = handle_unhide_rows(
            &mut wb,
            json!({"sheet": "Sheet1", "start_row": 3, "count": 1}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(!sheet.is_row_hidden(2)); // user row 3 unhidden
    }

    #[test]
    fn test_hide_rows_zero_row() {
        let mut wb = Workbook::new();
        let result = handle_hide_rows(
            &mut wb,
            json!({"sheet": "Sheet1", "start_row": 0, "count": 1}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_hide_and_unhide_cols() {
        let mut wb = Workbook::new();
        let result = handle_hide_cols(
            &mut wb,
            json!({"sheet": "Sheet1", "start_col": "B", "count": 2}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["cols_hidden"], 2);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(sheet.is_col_hidden(1)); // B = col 1
        assert!(sheet.is_col_hidden(2)); // C = col 2
        assert!(!sheet.is_col_hidden(0)); // A

        let result = handle_unhide_cols(
            &mut wb,
            json!({"sheet": "Sheet1", "start_col": "B", "count": 1}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(!sheet.is_col_hidden(1)); // B unhidden
        assert!(sheet.is_col_hidden(2)); // C still hidden
    }

    #[test]
    fn test_protect_and_unprotect_sheet() {
        let mut wb = Workbook::new();
        let result = handle_protect_sheet(
            &mut wb,
            json!({"sheet": "Sheet1"}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["has_password"], false);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(sheet.is_protected());

        let result = handle_unprotect_sheet(
            &mut wb,
            json!({"sheet": "Sheet1"}),
        )
        .unwrap();
        assert_eq!(result["success"], true);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(!sheet.is_protected());
    }

    #[test]
    fn test_protect_with_password() {
        let mut wb = Workbook::new();
        handle_protect_sheet(
            &mut wb,
            json!({"sheet": "Sheet1", "password": "secret"}),
        )
        .unwrap();

        // Wrong password should fail.
        let result = handle_unprotect_sheet(
            &mut wb,
            json!({"sheet": "Sheet1", "password": "wrong"}),
        );
        assert!(result.is_err());

        // Correct password should succeed.
        let result = handle_unprotect_sheet(
            &mut wb,
            json!({"sheet": "Sheet1", "password": "secret"}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_set_sheet_tab_color() {
        let mut wb = Workbook::new();
        let result = handle_set_sheet_tab_color(
            &mut wb,
            json!({"sheet": "Sheet1", "color": "#FF0000"}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["tab_color"], "#FF0000");

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert_eq!(sheet.tab_color, Some("#FF0000".to_string()));

        // Clear color.
        let result = handle_set_sheet_tab_color(
            &mut wb,
            json!({"sheet": "Sheet1", "color": null}),
        )
        .unwrap();
        assert_eq!(result["success"], true);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert_eq!(sheet.tab_color, None);
    }
}
