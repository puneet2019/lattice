//! JSON export for Lattice workbooks.
//!
//! Exports a workbook (or a specific range) as a JSON string for MCP data
//! exchange.

use serde::Serialize;

use lattice_core::{CellValue, Workbook};

use crate::{IoError, Result};

/// A serializable representation of a workbook for JSON export.
#[derive(Debug, Serialize)]
struct JsonWorkbook {
    sheets: Vec<JsonSheet>,
}

/// A serializable sheet.
#[derive(Debug, Serialize)]
struct JsonSheet {
    name: String,
    rows: Vec<Vec<JsonCell>>,
    used_range: UsedRange,
}

/// A serializable cell value.
#[derive(Debug, Serialize)]
struct JsonCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula: Option<String>,
}

/// Describes the used range of a sheet.
#[derive(Debug, Serialize)]
struct UsedRange {
    rows: u32,
    cols: u32,
}

/// Export a workbook as a pretty-printed JSON string.
///
/// The output format is:
/// ```json
/// {
///   "sheets": [
///     {
///       "name": "Sheet1",
///       "used_range": { "rows": 3, "cols": 2 },
///       "rows": [
///         [{"value": 42}, {"value": "hello"}],
///         [{"value": true}, null]
///       ]
///     }
///   ]
/// }
/// ```
pub fn export_json(workbook: &Workbook) -> Result<String> {
    let mut sheets = Vec::new();

    for sheet_name in workbook.sheet_names() {
        let sheet = workbook.get_sheet(&sheet_name).map_err(IoError::Core)?;

        let (max_row, max_col) = sheet.used_range();

        // Build the row data.
        let mut rows = Vec::new();
        if !sheet.cells().is_empty() {
            for row in 0..=max_row {
                let mut row_data = Vec::new();
                for col in 0..=max_col {
                    match sheet.get_cell(row, col) {
                        Some(cell) => {
                            let json_value = cell_value_to_json(&cell.value);
                            row_data.push(JsonCell {
                                value: json_value,
                                formula: cell.formula.clone(),
                            });
                        }
                        None => {
                            row_data.push(JsonCell {
                                value: None,
                                formula: None,
                            });
                        }
                    }
                }
                rows.push(row_data);
            }
        }

        sheets.push(JsonSheet {
            name: sheet_name,
            used_range: UsedRange {
                rows: if sheet.cells().is_empty() {
                    0
                } else {
                    max_row + 1
                },
                cols: if sheet.cells().is_empty() {
                    0
                } else {
                    max_col + 1
                },
            },
            rows,
        });
    }

    let json_wb = JsonWorkbook { sheets };
    let json = serde_json::to_string_pretty(&json_wb)?;
    Ok(json)
}

/// Export a specific range from a sheet as a JSON array of arrays.
///
/// Returns a JSON string like `[[1, "hello"], [2, "world"]]`.
/// Useful for MCP range-specific data exchange.
pub fn export_range_json(
    workbook: &Workbook,
    sheet_name: &str,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
) -> Result<String> {
    let sheet = workbook.get_sheet(sheet_name).map_err(IoError::Core)?;

    let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();

    for row in start_row..=end_row {
        let mut row_data = Vec::new();
        for col in start_col..=end_col {
            let json_val = match sheet.get_cell(row, col) {
                Some(cell) => cell_value_to_json(&cell.value).unwrap_or(serde_json::Value::Null),
                None => serde_json::Value::Null,
            };
            row_data.push(json_val);
        }
        rows.push(row_data);
    }

    let json = serde_json::to_string_pretty(&rows)?;
    Ok(json)
}

/// Convert a `CellValue` to a `serde_json::Value`.
fn cell_value_to_json(value: &CellValue) -> Option<serde_json::Value> {
    match value {
        CellValue::Empty => None,
        CellValue::Text(s) => Some(serde_json::Value::String(s.clone())),
        CellValue::Number(n) => serde_json::Number::from_f64(*n).map(serde_json::Value::Number),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => Some(serde_json::Value::Bool(*b)),
        CellValue::Error(e) => Some(serde_json::Value::String(e.to_string())),
        CellValue::Date(s) => Some(serde_json::Value::String(s.clone())),
        CellValue::Array(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| {
                    serde_json::Value::Array(
                        row.iter()
                            .map(|v| cell_value_to_json(v).unwrap_or(serde_json::Value::Null))
                            .collect(),
                    )
                })
                .collect();
            Some(serde_json::Value::Array(arr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_cell_value_to_json() {
        assert_eq!(cell_value_to_json(&CellValue::Empty), None);
        assert_eq!(
            cell_value_to_json(&CellValue::Text("hi".into())),
            Some(serde_json::json!("hi"))
        );
        assert_eq!(
            cell_value_to_json(&CellValue::Number(42.0)),
            Some(serde_json::json!(42.0))
        );
        assert_eq!(
            cell_value_to_json(&CellValue::Boolean(true)),
            Some(serde_json::json!(true))
        );
    }

    #[test]
    fn test_export_json_basic() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Text("hello".into()))
            .unwrap();

        let json = export_json(&wb).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["sheets"][0]["name"], "Sheet1");
        assert_eq!(parsed["sheets"][0]["rows"][0][0]["value"], 1.0);
        assert_eq!(parsed["sheets"][0]["rows"][0][1]["value"], "hello");
    }

    #[test]
    fn test_export_range_json() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(4.0)).unwrap();

        // Export only the first row.
        let json = export_range_json(&wb, "Sheet1", 0, 0, 0, 1).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0][0], 1.0);
        assert_eq!(parsed[0][1], 2.0);
        // Should only have one row.
        assert!(parsed.as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_export_json_empty_workbook() {
        let wb = Workbook::new();
        let json = export_json(&wb).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["sheets"][0]["used_range"]["rows"], 0);
        assert_eq!(parsed["sheets"][0]["used_range"]["cols"], 0);
    }

    #[test]
    fn test_export_json_with_formula() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let cell = lattice_core::Cell {
            value: CellValue::Number(3.0),
            formula: Some("SUM(A1:A2)".to_string()),
            format: Default::default(),
            style_id: 0,
            comment: None,
            hyperlink: None,
        };
        sheet.set_cell(0, 0, cell);

        let json = export_json(&wb).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["sheets"][0]["rows"][0][0]["formula"], "SUM(A1:A2)");
        assert_eq!(parsed["sheets"][0]["rows"][0][0]["value"], 3.0);
    }
}
