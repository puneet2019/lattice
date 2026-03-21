//! Cell operation tool handlers: read_cell, write_cell, read_range, write_range.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellRef, CellValue, Workbook};

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for cell operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "read_cell".to_string(),
            description: "Read the value and formula of a single cell".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    (
                        "cell_ref",
                        string_prop("Cell reference in A1 notation (e.g. 'B3')"),
                    ),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "write_cell".to_string(),
            description: "Write a value to a single cell".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation")),
                    (
                        "value",
                        json!({"description": "Value to write (string, number, or boolean)"}),
                    ),
                    (
                        "formula",
                        string_prop("Optional formula (without leading '=')"),
                    ),
                ],
                &["sheet", "cell_ref", "value"],
            ),
        },
        ToolDef {
            name: "read_range".to_string(),
            description: "Read a rectangular range of cells as a 2D array".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range in A1:B2 notation")),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "write_range".to_string(),
            description: "Write a 2D array of values to a range".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("start_cell", string_prop("Top-left cell in A1 notation")),
                    (
                        "values",
                        json!({
                            "type": "array",
                            "description": "2D array of values (rows of columns)",
                            "items": { "type": "array" }
                        }),
                    ),
                ],
                &["sheet", "start_cell", "values"],
            ),
        },
    ]
}

/// Arguments for read_cell.
#[derive(Debug, Deserialize)]
pub struct ReadCellArgs {
    pub sheet: String,
    pub cell_ref: String,
}

/// Handle the `read_cell` tool call.
pub fn handle_read_cell(workbook: &Workbook, args: Value) -> std::result::Result<Value, String> {
    let args: ReadCellArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell_ref =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let cell = workbook
        .get_cell(&args.sheet, cell_ref.row, cell_ref.col)
        .map_err(|e| e.to_string())?;

    match cell {
        Some(c) => Ok(json!({
            "value": cell_value_to_json(&c.value),
            "formula": c.formula,
            "cell_ref": args.cell_ref,
        })),
        None => Ok(json!({
            "value": null,
            "formula": null,
            "cell_ref": args.cell_ref,
        })),
    }
}

/// Arguments for write_cell.
#[derive(Debug, Deserialize)]
pub struct WriteCellArgs {
    pub sheet: String,
    pub cell_ref: String,
    pub value: Value,
    pub formula: Option<String>,
}

/// Handle the `write_cell` tool call.
pub fn handle_write_cell(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: WriteCellArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell_ref =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let value = json_to_cell_value(&args.value);

    workbook
        .set_cell(&args.sheet, cell_ref.row, cell_ref.col, value)
        .map_err(|e| e.to_string())?;

    // If a formula was provided, set it on the cell.
    if let Some(formula) = args.formula {
        let sheet = workbook
            .get_sheet_mut(&args.sheet)
            .map_err(|e| e.to_string())?;
        if let Some(cell) = sheet.get_cell(cell_ref.row, cell_ref.col) {
            let mut new_cell = cell.clone();
            new_cell.formula = Some(formula);
            sheet.set_cell(cell_ref.row, cell_ref.col, new_cell);
        }
    }

    Ok(json!({
        "success": true,
        "cell_ref": args.cell_ref,
    }))
}

/// Arguments for read_range.
#[derive(Debug, Deserialize)]
pub struct ReadRangeArgs {
    pub sheet: String,
    pub range: String,
}

/// Handle the `read_range` tool call.
pub fn handle_read_range(workbook: &Workbook, args: Value) -> std::result::Result<Value, String> {
    let args: ReadRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;

    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    for row in start.row..=end.row {
        let mut row_data = Vec::new();
        for col in start.col..=end.col {
            match sheet.get_cell(row, col) {
                Some(cell) => row_data.push(cell_value_to_json(&cell.value)),
                None => row_data.push(Value::Null),
            }
        }
        rows.push(Value::Array(row_data));
    }

    Ok(json!({
        "range": args.range,
        "data": rows,
    }))
}

/// Arguments for write_range.
#[derive(Debug, Deserialize)]
pub struct WriteRangeArgs {
    pub sheet: String,
    pub start_cell: String,
    pub values: Vec<Vec<Value>>,
}

/// Handle the `write_range` tool call.
pub fn handle_write_range(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: WriteRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let start =
        CellRef::parse(&args.start_cell).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let mut cells_written = 0u32;
    for (row_offset, row) in args.values.iter().enumerate() {
        for (col_offset, val) in row.iter().enumerate() {
            let value = json_to_cell_value(val);
            workbook
                .set_cell(
                    &args.sheet,
                    start.row + row_offset as u32,
                    start.col + col_offset as u32,
                    value,
                )
                .map_err(|e| e.to_string())?;
            cells_written += 1;
        }
    }

    Ok(json!({
        "success": true,
        "cells_written": cells_written,
    }))
}

/// Convert a CellValue to a serde_json::Value for responses.
fn cell_value_to_json(cv: &CellValue) -> Value {
    match cv {
        CellValue::Empty => Value::Null,
        CellValue::Text(s) => Value::String(s.clone()),
        CellValue::Number(n) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        CellValue::Boolean(b) => Value::Bool(*b),
        CellValue::Error(e) => Value::String(e.to_string()),
        CellValue::Date(s) => Value::String(s.clone()),
    }
}

/// Convert a JSON value to a CellValue.
fn json_to_cell_value(v: &Value) -> CellValue {
    match v {
        Value::Null => CellValue::Empty,
        Value::String(s) => CellValue::Text(s.clone()),
        Value::Number(n) => CellValue::Number(n.as_f64().unwrap_or(0.0)),
        Value::Bool(b) => CellValue::Boolean(*b),
        _ => CellValue::Text(v.to_string()),
    }
}

/// Parse a range string like "A1:C3" into two CellRefs.
fn parse_range(range: &str) -> std::result::Result<(CellRef, CellRef), String> {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid range format '{}': expected 'A1:B2'",
            range
        ));
    }
    let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
    let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_read_cell() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();

        let result = handle_read_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();

        assert_eq!(result["value"], 42.0);
    }

    #[test]
    fn test_handle_write_cell() {
        let mut wb = Workbook::new();

        let result = handle_write_cell(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "B2", "value": "hello"}),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        let cell = wb.get_cell("Sheet1", 1, 1).unwrap().unwrap();
        assert_eq!(cell.value, CellValue::Text("hello".into()));
    }

    #[test]
    fn test_handle_read_range() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(4.0)).unwrap();

        let result = handle_read_range(&wb, json!({"sheet": "Sheet1", "range": "A1:B2"})).unwrap();

        assert_eq!(result["data"][0][0], 1.0);
        assert_eq!(result["data"][0][1], 2.0);
        assert_eq!(result["data"][1][0], 3.0);
        assert_eq!(result["data"][1][1], 4.0);
    }

    #[test]
    fn test_handle_write_range() {
        let mut wb = Workbook::new();

        let result = handle_write_range(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "start_cell": "A1",
                "values": [[1, 2], [3, 4]],
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["cells_written"], 4);
    }
}
