//! File operation tool handlers: get_workbook_info, export_json, export_csv.
//!
//! These tools provide workbook metadata and export capabilities.
//! Full file I/O (open_file, save_file, import_csv) will require the lattice-io crate.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellValue, Workbook, col_to_letter};

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for file operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "get_workbook_info".to_string(),
            description: "Get workbook metadata: sheet count, cell counts per sheet, and summary"
                .to_string(),
            input_schema: object_schema(&[], &[]),
        },
        ToolDef {
            name: "export_json".to_string(),
            description: "Export a sheet or the entire workbook as JSON".to_string(),
            input_schema: object_schema(
                &[(
                    "sheet",
                    string_prop("Sheet to export (omit for entire workbook)"),
                )],
                &[],
            ),
        },
        ToolDef {
            name: "export_csv".to_string(),
            description: "Export a sheet as a CSV string".to_string(),
            input_schema: object_schema(
                &[("sheet", string_prop("Sheet to export as CSV"))],
                &["sheet"],
            ),
        },
    ]
}

/// Handle the `get_workbook_info` tool call.
pub fn handle_get_workbook_info(workbook: &Workbook) -> Result<Value, String> {
    let names = workbook.sheet_names();
    let mut sheets_info = Vec::new();
    let mut total_cells = 0usize;

    for name in &names {
        let sheet = workbook.get_sheet(name).map_err(|e| e.to_string())?;
        let cell_count = sheet.cells().len();
        let (max_row, max_col) = sheet.used_range();
        total_cells += cell_count;

        sheets_info.push(json!({
            "name": name,
            "cell_count": cell_count,
            "used_range": {
                "rows": if cell_count == 0 { 0 } else { max_row + 1 },
                "cols": if cell_count == 0 { 0 } else { max_col + 1 },
            },
        }));
    }

    Ok(json!({
        "sheet_count": names.len(),
        "total_cells": total_cells,
        "active_sheet": workbook.active_sheet,
        "sheets": sheets_info,
    }))
}

/// Arguments for export_json.
#[derive(Debug, Deserialize)]
pub struct ExportJsonArgs {
    pub sheet: Option<String>,
}

/// Handle the `export_json` tool call.
pub fn handle_export_json(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: ExportJsonArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    match args.sheet {
        Some(sheet_name) => {
            let sheet_data = export_sheet_to_json(workbook, &sheet_name)?;
            Ok(json!({
                "format": "json",
                "sheet": sheet_name,
                "data": sheet_data,
            }))
        }
        None => {
            let mut workbook_data = json!({});
            for name in workbook.sheet_names() {
                let sheet_data = export_sheet_to_json(workbook, &name)?;
                workbook_data[&name] = sheet_data;
            }
            Ok(json!({
                "format": "json",
                "sheet": null,
                "data": workbook_data,
            }))
        }
    }
}

/// Arguments for export_csv.
#[derive(Debug, Deserialize)]
pub struct ExportCsvArgs {
    pub sheet: String,
}

/// Handle the `export_csv` tool call.
pub fn handle_export_csv(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: ExportCsvArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    let (max_row, max_col) = sheet.used_range();
    if sheet.cells().is_empty() {
        return Ok(json!({
            "format": "csv",
            "sheet": args.sheet,
            "csv": "",
            "rows": 0,
        }));
    }

    let mut csv_lines = Vec::new();
    for row in 0..=max_row {
        let mut row_values = Vec::new();
        for col in 0..=max_col {
            let text = match sheet.get_cell(row, col) {
                Some(cell) => cell_value_to_csv_field(&cell.value),
                None => String::new(),
            };
            row_values.push(text);
        }
        csv_lines.push(row_values.join(","));
    }

    let csv_str = csv_lines.join("\n");

    Ok(json!({
        "format": "csv",
        "sheet": args.sheet,
        "csv": csv_str,
        "rows": max_row + 1,
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Export a single sheet to a JSON value (array of row objects or 2D array).
fn export_sheet_to_json(workbook: &Workbook, sheet_name: &str) -> Result<Value, String> {
    let sheet = workbook.get_sheet(sheet_name).map_err(|e| e.to_string())?;

    let (max_row, max_col) = sheet.used_range();
    if sheet.cells().is_empty() {
        return Ok(json!([]));
    }

    let mut rows = Vec::new();
    for row in 0..=max_row {
        let mut row_data = serde_json::Map::new();
        for col in 0..=max_col {
            let col_name = col_to_letter(col);
            match sheet.get_cell(row, col) {
                Some(cell) => {
                    row_data.insert(col_name, cell_value_to_json(&cell.value));
                }
                None => {
                    row_data.insert(col_name, Value::Null);
                }
            }
        }
        rows.push(Value::Object(row_data));
    }

    Ok(Value::Array(rows))
}

/// Convert a CellValue to a serde_json::Value.
fn cell_value_to_json(cv: &CellValue) -> Value {
    match cv {
        CellValue::Empty => Value::Null,
        CellValue::Text(s) => Value::String(s.clone()),
        CellValue::Number(n) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => Value::Bool(*b),
        CellValue::Error(e) => Value::String(e.to_string()),
        CellValue::Date(s) => Value::String(s.clone()),
        CellValue::Array(rows) => {
            let arr: Vec<Value> = rows
                .iter()
                .map(|row| Value::Array(row.iter().map(|v| cell_value_to_json(v)).collect()))
                .collect();
            Value::Array(arr)
        }
    }
}

/// Convert a CellValue to a CSV-safe string field.
fn cell_value_to_csv_field(cv: &CellValue) -> String {
    match cv {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => {
            // Quote text that contains commas, newlines, or quotes.
            if s.contains(',') || s.contains('\n') || s.contains('"') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => b.to_string().to_uppercase(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_workbook_info() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();
        wb.add_sheet("Data").unwrap();

        let result = handle_get_workbook_info(&wb).unwrap();

        assert_eq!(result["sheet_count"], 2);
        assert_eq!(result["total_cells"], 1);
        assert_eq!(result["sheets"][0]["name"], "Sheet1");
        assert_eq!(result["sheets"][0]["cell_count"], 1);
        assert_eq!(result["sheets"][1]["name"], "Data");
        assert_eq!(result["sheets"][1]["cell_count"], 0);
    }

    #[test]
    fn test_export_json_single_sheet() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Text("hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(2.0)).unwrap();

        let result = handle_export_json(&wb, json!({"sheet": "Sheet1"})).unwrap();

        assert_eq!(result["format"], "json");
        assert_eq!(result["sheet"], "Sheet1");
        let data = result["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["A"], 1.0);
        assert_eq!(data[0]["B"], "hello");
    }

    #[test]
    fn test_export_json_entire_workbook() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();
        wb.add_sheet("Sheet2").unwrap();
        wb.set_cell("Sheet2", 0, 0, CellValue::Text("hi".into()))
            .unwrap();

        let result = handle_export_json(&wb, json!({})).unwrap();

        assert!(result["data"]["Sheet1"].is_array());
        assert!(result["data"]["Sheet2"].is_array());
    }

    #[test]
    fn test_export_csv() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Text("hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Text("world".into()))
            .unwrap();

        let result = handle_export_csv(&wb, json!({"sheet": "Sheet1"})).unwrap();

        let csv = result["csv"].as_str().unwrap();
        assert!(csv.contains("1,hello"));
        assert!(csv.contains("2,world"));
    }

    #[test]
    fn test_export_csv_empty_sheet() {
        let wb = Workbook::new();
        let result = handle_export_csv(&wb, json!({"sheet": "Sheet1"})).unwrap();
        assert_eq!(result["csv"], "");
        assert_eq!(result["rows"], 0);
    }

    #[test]
    fn test_export_csv_with_commas() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello, world".into()))
            .unwrap();

        let result = handle_export_csv(&wb, json!({"sheet": "Sheet1"})).unwrap();
        let csv = result["csv"].as_str().unwrap();
        assert_eq!(csv, "\"hello, world\"");
    }

    #[test]
    fn test_cell_value_to_csv_field() {
        assert_eq!(cell_value_to_csv_field(&CellValue::Empty), "");
        assert_eq!(cell_value_to_csv_field(&CellValue::Number(42.0)), "42");
        assert_eq!(cell_value_to_csv_field(&CellValue::Boolean(true)), "TRUE");
        assert_eq!(
            cell_value_to_csv_field(&CellValue::Text("a,b".into())),
            "\"a,b\""
        );
    }
}
