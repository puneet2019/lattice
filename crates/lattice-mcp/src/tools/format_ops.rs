//! Format operation tool handlers: get_cell_format, set_cell_format, merge_cells, unmerge_cells.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellFormat, CellRef, HAlign, VAlign, Workbook};

use super::ToolDef;
use crate::schema::{bool_prop, number_prop, object_schema, string_prop};

/// Return tool definitions for format operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "get_cell_format".to_string(),
            description: "Get the visual format of a cell (bold, italic, font_size, colors, alignment)".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation (e.g. 'B3')")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "set_cell_format".to_string(),
            description: "Set visual format properties on a cell or range. Only provided properties are changed; others are preserved.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation, or range like 'A1:C3'")),
                    ("bold", bool_prop("Whether the text should be bold")),
                    ("italic", bool_prop("Whether the text should be italic")),
                    ("font_size", number_prop("Font size in points (e.g. 11.0)")),
                    ("font_color", string_prop("Font color as hex string (e.g. '#FF0000')")),
                    ("bg_color", string_prop("Background color as hex string (e.g. '#FFFF00'), or null to clear")),
                    ("h_align", string_prop("Horizontal alignment: 'left', 'center', or 'right'")),
                    ("v_align", string_prop("Vertical alignment: 'top', 'middle', or 'bottom'")),
                    ("number_format", string_prop("Number format pattern (e.g. '#,##0.00'), or null to clear")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "merge_cells".to_string(),
            description: "Merge a rectangular range of cells. The value of the top-left cell is preserved; other cells are cleared.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range to merge in A1:B2 notation")),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "unmerge_cells".to_string(),
            description: "Unmerge a previously merged region that contains the given cell".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Any cell within the merged region, in A1 notation")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
    ]
}

/// Arguments for get_cell_format.
#[derive(Debug, Deserialize)]
pub struct GetCellFormatArgs {
    pub sheet: String,
    pub cell_ref: String,
}

/// Handle the `get_cell_format` tool call.
pub fn handle_get_cell_format(
    workbook: &Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: GetCellFormatArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell_ref =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let cell = workbook
        .get_cell(&args.sheet, cell_ref.row, cell_ref.col)
        .map_err(|e| e.to_string())?;

    let format = match cell {
        Some(c) => format_to_json(&c.format),
        None => format_to_json(&CellFormat::default()),
    };

    Ok(json!({
        "cell_ref": args.cell_ref,
        "format": format,
    }))
}

/// Arguments for set_cell_format.
#[derive(Debug, Deserialize)]
pub struct SetCellFormatArgs {
    pub sheet: String,
    pub cell_ref: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font_size: Option<f64>,
    pub font_color: Option<String>,
    pub bg_color: Option<Value>,
    pub h_align: Option<String>,
    pub v_align: Option<String>,
    pub number_format: Option<Value>,
}

/// Handle the `set_cell_format` tool call.
///
/// We parse `args` directly as a JSON object so we can distinguish between a
/// field being **absent** (no change) and a field being explicitly **null**
/// (clear the value).  Using `Option<Value>` via serde would collapse both
/// cases to `None`.
pub fn handle_set_cell_format(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    // Validate required scalar fields via the typed struct.
    let typed: SetCellFormatArgs =
        serde_json::from_value(args.clone()).map_err(|e| format!("Invalid arguments: {}", e))?;

    let raw = args.as_object().ok_or("arguments must be a JSON object")?;

    // Determine if we're formatting a single cell or a range.
    let cells = if typed.cell_ref.contains(':') {
        parse_range_cells(&typed.cell_ref)?
    } else {
        let cr = CellRef::parse(&typed.cell_ref)
            .map_err(|e| format!("Invalid cell reference: {}", e))?;
        vec![(cr.row, cr.col)]
    };

    let sheet = workbook
        .get_sheet_mut(&typed.sheet)
        .map_err(|e| e.to_string())?;

    let mut cells_formatted = 0u32;
    for (row, col) in &cells {
        // Ensure the cell exists (create default if not).
        let cell = sheet.cells_mut().entry((*row, *col)).or_default();

        // Apply only the properties that are present in the JSON object.
        // Checking the raw map allows null to mean "clear" and absence to mean
        // "leave unchanged".
        if let Some(bold) = typed.bold {
            cell.format.bold = bold;
        }
        if let Some(italic) = typed.italic {
            cell.format.italic = italic;
        }
        if let Some(font_size) = typed.font_size {
            cell.format.font_size = font_size;
        }
        if let Some(ref font_color) = typed.font_color {
            cell.format.font_color = Some(font_color.clone());
        }
        // bg_color: present+null → clear; present+string → set; absent → keep.
        if raw.contains_key("bg_color") {
            cell.format.bg_color = match raw.get("bg_color").unwrap() {
                Value::Null => None,
                Value::String(s) => Some(s.clone()),
                other => Some(other.to_string()),
            };
        }
        if let Some(ref h_align) = typed.h_align {
            cell.format.h_align = parse_h_align(h_align)?;
        }
        if let Some(ref v_align) = typed.v_align {
            cell.format.v_align = parse_v_align(v_align)?;
        }
        // number_format: same null-means-clear semantics as bg_color.
        if raw.contains_key("number_format") {
            cell.format.number_format = match raw.get("number_format").unwrap() {
                Value::Null => None,
                Value::String(s) => Some(s.clone()),
                other => Some(other.to_string()),
            };
        }
        cells_formatted += 1;
    }

    Ok(json!({
        "success": true,
        "cells_formatted": cells_formatted,
        "cell_ref": typed.cell_ref,
    }))
}

/// Arguments for merge_cells.
#[derive(Debug, Deserialize)]
pub struct MergeCellsArgs {
    pub sheet: String,
    pub range: String,
}

/// Handle the `merge_cells` tool call.
pub fn handle_merge_cells(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: MergeCellsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    sheet
        .merge_cells(start.row, start.col, end.row, end.col)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "range": args.range,
    }))
}

/// Arguments for unmerge_cells.
#[derive(Debug, Deserialize)]
pub struct UnmergeCellsArgs {
    pub sheet: String,
    pub cell_ref: String,
}

/// Handle the `unmerge_cells` tool call.
pub fn handle_unmerge_cells(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: UnmergeCellsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell_ref =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    let unmerged = sheet
        .unmerge_cell(cell_ref.row, cell_ref.col)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "was_merged": unmerged,
        "cell_ref": args.cell_ref,
    }))
}

// ── Helper functions ───────────────────────────────────────────────────

/// Convert a CellFormat to a JSON value.
fn format_to_json(fmt: &CellFormat) -> Value {
    json!({
        "bold": fmt.bold,
        "italic": fmt.italic,
        "font_size": fmt.font_size,
        "font_color": fmt.font_color,
        "bg_color": fmt.bg_color,
        "h_align": h_align_str(&fmt.h_align),
        "v_align": v_align_str(&fmt.v_align),
        "number_format": fmt.number_format,
    })
}

/// Convert HAlign to a string.
fn h_align_str(a: &HAlign) -> &'static str {
    match a {
        HAlign::Left => "left",
        HAlign::Center => "center",
        HAlign::Right => "right",
    }
}

/// Convert VAlign to a string.
fn v_align_str(a: &VAlign) -> &'static str {
    match a {
        VAlign::Top => "top",
        VAlign::Middle => "middle",
        VAlign::Bottom => "bottom",
    }
}

/// Parse an h_align string into an HAlign enum.
fn parse_h_align(s: &str) -> std::result::Result<HAlign, String> {
    match s.to_lowercase().as_str() {
        "left" => Ok(HAlign::Left),
        "center" | "centre" => Ok(HAlign::Center),
        "right" => Ok(HAlign::Right),
        _ => Err(format!(
            "Invalid horizontal alignment '{}': expected 'left', 'center', or 'right'",
            s
        )),
    }
}

/// Parse a v_align string into a VAlign enum.
fn parse_v_align(s: &str) -> std::result::Result<VAlign, String> {
    match s.to_lowercase().as_str() {
        "top" => Ok(VAlign::Top),
        "middle" => Ok(VAlign::Middle),
        "bottom" => Ok(VAlign::Bottom),
        _ => Err(format!(
            "Invalid vertical alignment '{}': expected 'top', 'middle', or 'bottom'",
            s
        )),
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

/// Parse a range string into a list of (row, col) pairs for all cells in the range.
fn parse_range_cells(range: &str) -> std::result::Result<Vec<(u32, u32)>, String> {
    let (start, end) = parse_range(range)?;
    let mut cells = Vec::new();
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            cells.push((row, col));
        }
    }
    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_get_cell_format_default() {
        let wb = Workbook::new();
        let result =
            handle_get_cell_format(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();

        assert_eq!(result["format"]["bold"], false);
        assert_eq!(result["format"]["italic"], false);
        assert_eq!(result["format"]["font_size"], 11.0);
        assert_eq!(result["format"]["font_color"], "#000000");
        assert_eq!(result["format"]["h_align"], "left");
        assert_eq!(result["format"]["v_align"], "bottom");
    }

    #[test]
    fn test_get_cell_format_custom() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        if let Some(cell) = sheet.get_cell_mut(0, 0) {
            cell.format.bold = true;
            cell.format.font_size = 14.0;
            cell.format.bg_color = Some("#FFFF00".to_string());
        }

        let result =
            handle_get_cell_format(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();

        assert_eq!(result["format"]["bold"], true);
        assert_eq!(result["format"]["font_size"], 14.0);
        assert_eq!(result["format"]["bg_color"], "#FFFF00");
    }

    #[test]
    fn test_set_cell_format_single() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();

        let result = handle_set_cell_format(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "A1",
                "bold": true,
                "font_size": 16.0,
                "h_align": "center"
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["cells_formatted"], 1);

        let cell = wb.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        assert!(cell.format.bold);
        assert_eq!(cell.format.font_size, 16.0);
        assert_eq!(cell.format.h_align, HAlign::Center);
        // Unchanged properties should remain at defaults.
        assert!(!cell.format.italic);
    }

    #[test]
    fn test_set_cell_format_range() {
        let mut wb = Workbook::new();

        let result = handle_set_cell_format(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "A1:B2",
                "bold": true
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["cells_formatted"], 4);

        // All four cells should be bold.
        for row in 0..=1 {
            for col in 0..=1 {
                let cell = wb.get_cell("Sheet1", row, col).unwrap().unwrap();
                assert!(cell.format.bold);
            }
        }
    }

    #[test]
    fn test_merge_cells() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("main".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Text("cleared".into()))
            .unwrap();

        let result =
            handle_merge_cells(&mut wb, json!({"sheet": "Sheet1", "range": "A1:B2"})).unwrap();

        assert_eq!(result["success"], true);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert_eq!(sheet.merged_regions().len(), 1);
        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Text("main".into())
        );
        assert!(sheet.get_cell(0, 1).is_none());
    }

    #[test]
    fn test_unmerge_cells() {
        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.merge_cells(0, 0, 1, 1).unwrap();
        assert_eq!(sheet.merged_regions().len(), 1);

        let result =
            handle_unmerge_cells(&mut wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["was_merged"], true);
        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert_eq!(sheet.merged_regions().len(), 0);
    }

    #[test]
    fn test_unmerge_not_merged() {
        let mut wb = Workbook::new();

        let result =
            handle_unmerge_cells(&mut wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["was_merged"], false);
    }

    #[test]
    fn test_set_cell_format_invalid_alignment() {
        let mut wb = Workbook::new();

        let result = handle_set_cell_format(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "A1",
                "h_align": "invalid"
            }),
        );

        assert!(result.is_err());
    }
}
