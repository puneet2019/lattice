//! MCP resource endpoints.
//!
//! Resources provide read-only data access to the workbook via URI patterns.

use serde_json::{Value, json};

use lattice_core::{CellValue, Workbook, col_to_letter};

/// Handle the `resources/list` method.
///
/// Returns the list of available resource URIs and templates.
pub fn handle_list_resources() -> Result<Value, (i32, String)> {
    Ok(json!({
        "resources": [
            {
                "uri": "lattice://workbook/info",
                "name": "Workbook Info",
                "description": "Workbook metadata (sheets, cell counts, active sheet)",
                "mimeType": "application/json",
            },
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "lattice://sheet/{name}/data",
                "name": "Sheet Data",
                "description": "Full sheet data as JSON (rows of column-keyed objects)",
                "mimeType": "application/json",
            },
            {
                "uriTemplate": "lattice://sheet/{name}/range/{range}",
                "name": "Sheet Range",
                "description": "Specific range data as a 2D JSON array",
                "mimeType": "application/json",
            },
            {
                "uriTemplate": "lattice://sheet/{name}/summary",
                "name": "Sheet Summary",
                "description": "Auto-generated data summary (row/col counts, data types breakdown)",
                "mimeType": "application/json",
            },
            {
                "uriTemplate": "lattice://sheet/{name}/formulas",
                "name": "Sheet Formulas",
                "description": "List all cells with formulas in the sheet",
                "mimeType": "application/json",
            },
        ],
    }))
}

/// Handle the `resources/read` method.
///
/// Reads a resource by URI, using the workbook for data.
pub fn handle_read_resource(params: Value, workbook: &Workbook) -> Result<Value, (i32, String)> {
    let uri = params["uri"]
        .as_str()
        .ok_or((-32602, "Missing 'uri' parameter".to_string()))?;

    // Static resources.
    if uri == "lattice://workbook/info" {
        return read_workbook_info(uri, workbook);
    }

    // Template resources: lattice://sheet/{name}/...
    if let Some(rest) = uri.strip_prefix("lattice://sheet/") {
        // Parse: {name}/data, {name}/range/{range}, {name}/summary, {name}/formulas
        if let Some((name, suffix)) = rest.split_once('/') {
            let name = urllike_decode(name);

            if suffix == "data" {
                return read_sheet_data(uri, workbook, &name);
            }
            if suffix == "summary" {
                return read_sheet_summary(uri, workbook, &name);
            }
            if suffix == "formulas" {
                return read_sheet_formulas(uri, workbook, &name);
            }
            if let Some(range) = suffix.strip_prefix("range/") {
                return read_sheet_range(uri, workbook, &name, range);
            }
        }
    }

    Err((-32002, format!("Resource not found: {}", uri)))
}

// ── Resource Handlers ────────────────────────────────────────────────────────

/// `lattice://workbook/info` — workbook metadata.
fn read_workbook_info(uri: &str, workbook: &Workbook) -> Result<Value, (i32, String)> {
    let names = workbook.sheet_names();
    let mut sheets = Vec::new();
    let mut total_cells = 0usize;

    for name in &names {
        if let Ok(sheet) = workbook.get_sheet(name) {
            let count = sheet.cells().len();
            total_cells += count;
            let (max_row, max_col) = sheet.used_range();
            sheets.push(json!({
                "name": name,
                "cell_count": count,
                "used_range": {
                    "rows": if count == 0 { 0 } else { max_row + 1 },
                    "cols": if count == 0 { 0 } else { max_col + 1 },
                },
            }));
        }
    }

    let info = json!({
        "sheet_count": names.len(),
        "total_cells": total_cells,
        "active_sheet": workbook.active_sheet,
        "sheets": sheets,
    });

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&info).unwrap(),
        }],
    }))
}

/// `lattice://sheet/{name}/data` — full sheet data as JSON.
fn read_sheet_data(
    uri: &str,
    workbook: &Workbook,
    sheet_name: &str,
) -> Result<Value, (i32, String)> {
    let sheet = workbook
        .get_sheet(sheet_name)
        .map_err(|e| (-32002, e.to_string()))?;

    let (max_row, max_col) = sheet.used_range();
    let mut rows = Vec::new();

    if !sheet.cells().is_empty() {
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
    }

    let data = Value::Array(rows);

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&data).unwrap(),
        }],
    }))
}

/// `lattice://sheet/{name}/range/{range}` — specific range data as 2D array.
fn read_sheet_range(
    uri: &str,
    workbook: &Workbook,
    sheet_name: &str,
    range: &str,
) -> Result<Value, (i32, String)> {
    let (start, end) = parse_range(range).map_err(|e| (-32602, e))?;

    let sheet = workbook
        .get_sheet(sheet_name)
        .map_err(|e| (-32002, e.to_string()))?;

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

    let data = json!({
        "range": range,
        "data": rows,
    });

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&data).unwrap(),
        }],
    }))
}

/// `lattice://sheet/{name}/summary` — auto-generated data summary.
fn read_sheet_summary(
    uri: &str,
    workbook: &Workbook,
    sheet_name: &str,
) -> Result<Value, (i32, String)> {
    let sheet = workbook
        .get_sheet(sheet_name)
        .map_err(|e| (-32002, e.to_string()))?;

    let (max_row, max_col) = sheet.used_range();
    let cell_count = sheet.cells().len();

    // Data type breakdown.
    let mut number_count = 0u32;
    let mut text_count = 0u32;
    let mut bool_count = 0u32;
    let mut empty_count = 0u32;
    let mut error_count = 0u32;
    let mut date_count = 0u32;
    let mut formula_count = 0u32;

    for cell in sheet.cells().values() {
        match &cell.value {
            CellValue::Number(_) => number_count += 1,
            CellValue::Text(_) => text_count += 1,
            CellValue::Boolean(_) | CellValue::Checkbox(_) => bool_count += 1,
            CellValue::Empty => empty_count += 1,
            CellValue::Error(_) => error_count += 1,
            CellValue::Date(_) => date_count += 1,
            CellValue::Array(_) => number_count += 1,
            CellValue::Lambda { .. } => {}
        }
        if cell.formula.is_some() {
            formula_count += 1;
        }
    }

    let summary = json!({
        "sheet": sheet_name,
        "rows": if cell_count == 0 { 0 } else { max_row + 1 },
        "cols": if cell_count == 0 { 0 } else { max_col + 1 },
        "cell_count": cell_count,
        "data_types": {
            "numbers": number_count,
            "text": text_count,
            "booleans": bool_count,
            "empty": empty_count,
            "errors": error_count,
            "dates": date_count,
        },
        "formula_count": formula_count,
    });

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&summary).unwrap(),
        }],
    }))
}

/// `lattice://sheet/{name}/formulas` — list all cells with formulas.
fn read_sheet_formulas(
    uri: &str,
    workbook: &Workbook,
    sheet_name: &str,
) -> Result<Value, (i32, String)> {
    let sheet = workbook
        .get_sheet(sheet_name)
        .map_err(|e| (-32002, e.to_string()))?;

    let mut formulas = Vec::new();

    // Collect cells with formulas, sorted by position.
    let mut formula_cells: Vec<((u32, u32), &str)> = sheet
        .cells()
        .iter()
        .filter_map(|(&(row, col), cell)| cell.formula.as_deref().map(|f| ((row, col), f)))
        .collect();
    formula_cells.sort_by_key(|&((r, c), _)| (r, c));

    for ((row, col), formula) in formula_cells {
        let cell_label = format!("{}{}", col_to_letter(col), row + 1);
        let value = sheet
            .get_cell(row, col)
            .map(|c| cell_value_to_json(&c.value))
            .unwrap_or(Value::Null);

        formulas.push(json!({
            "cell_ref": cell_label,
            "formula": formula,
            "computed_value": value,
        }));
    }

    let data = json!({
        "sheet": sheet_name,
        "formula_count": formulas.len(),
        "formulas": formulas,
    });

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&data).unwrap(),
        }],
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

use lattice_core::CellRef;

/// Parse a range string like "A1:C3" into two CellRefs.
fn parse_range(range: &str) -> Result<(CellRef, CellRef), String> {
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
                .map(|row| Value::Array(row.iter().map(cell_value_to_json).collect()))
                .collect();
            Value::Array(arr)
        }
        CellValue::Lambda { .. } => Value::String("{lambda}".to_string()),
    }
}

/// Simple URL-like decoding for sheet names containing %20, etc.
fn urllike_decode(s: &str) -> String {
    s.replace("%20", " ")
        .replace("%2F", "/")
        .replace("%25", "%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_resources() {
        let result = handle_list_resources().unwrap();
        assert!(result["resources"].is_array());
        assert!(result["resourceTemplates"].is_array());
        assert!(!result["resources"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_read_workbook_info() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();

        let result = handle_read_resource(json!({"uri": "lattice://workbook/info"}), &wb).unwrap();

        let text = result["contents"][0]["text"].as_str().unwrap();
        let info: Value = serde_json::from_str(text).unwrap();
        assert_eq!(info["sheet_count"], 1);
        assert_eq!(info["total_cells"], 1);
    }

    #[test]
    fn test_read_sheet_data() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Text("hello".into()))
            .unwrap();

        let result =
            handle_read_resource(json!({"uri": "lattice://sheet/Sheet1/data"}), &wb).unwrap();

        let text = result["contents"][0]["text"].as_str().unwrap();
        let data: Value = serde_json::from_str(text).unwrap();
        assert!(data.is_array());
        assert_eq!(data[0]["A"], 1.0);
        assert_eq!(data[0]["B"], "hello");
    }

    #[test]
    fn test_read_sheet_range() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(4.0)).unwrap();

        let result =
            handle_read_resource(json!({"uri": "lattice://sheet/Sheet1/range/A1:B2"}), &wb)
                .unwrap();

        let text = result["contents"][0]["text"].as_str().unwrap();
        let data: Value = serde_json::from_str(text).unwrap();
        assert_eq!(data["data"][0][0], 1.0);
        assert_eq!(data["data"][1][1], 4.0);
    }

    #[test]
    fn test_read_sheet_summary() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Boolean(true))
            .unwrap();

        let result =
            handle_read_resource(json!({"uri": "lattice://sheet/Sheet1/summary"}), &wb).unwrap();

        let text = result["contents"][0]["text"].as_str().unwrap();
        let summary: Value = serde_json::from_str(text).unwrap();
        assert_eq!(summary["cell_count"], 3);
        assert_eq!(summary["data_types"]["numbers"], 1);
        assert_eq!(summary["data_types"]["text"], 1);
        assert_eq!(summary["data_types"]["booleans"], 1);
    }

    #[test]
    fn test_read_sheet_formulas() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(15.0))
            .unwrap();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let mut cell = sheet.get_cell(0, 0).unwrap().clone();
        cell.formula = Some("SUM(B1:B5)".to_string());
        sheet.set_cell(0, 0, cell);

        let result =
            handle_read_resource(json!({"uri": "lattice://sheet/Sheet1/formulas"}), &wb).unwrap();

        let text = result["contents"][0]["text"].as_str().unwrap();
        let data: Value = serde_json::from_str(text).unwrap();
        assert_eq!(data["formula_count"], 1);
        assert_eq!(data["formulas"][0]["cell_ref"], "A1");
        assert_eq!(data["formulas"][0]["formula"], "SUM(B1:B5)");
    }

    #[test]
    fn test_read_resource_not_found() {
        let wb = Workbook::new();
        let result = handle_read_resource(json!({"uri": "lattice://nonexistent"}), &wb);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_missing_uri() {
        let wb = Workbook::new();
        let result = handle_read_resource(json!({}), &wb);
        assert!(result.is_err());
    }

    #[test]
    fn test_urllike_decode() {
        assert_eq!(urllike_decode("Sheet%201"), "Sheet 1");
    }
}
