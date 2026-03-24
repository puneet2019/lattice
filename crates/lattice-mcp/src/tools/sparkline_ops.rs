//! Sparkline MCP tool handlers: add_sparkline, remove_sparkline, list_sparklines.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellRef, SparklineConfig, SparklineType, Workbook, col_to_letter};

use super::ToolDef;
use crate::schema::{bool_prop, number_prop, object_schema, string_prop};

/// Return tool definitions for sparkline operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "add_sparkline".to_string(),
            description:
                "Add an inline sparkline chart to a cell. Supports line, bar, and win_loss types."
                    .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    (
                        "cell_ref",
                        string_prop("Cell to place the sparkline in (A1 notation)"),
                    ),
                    (
                        "spark_type",
                        string_prop("Sparkline type: 'line', 'bar', or 'win_loss'"),
                    ),
                    (
                        "data_range",
                        string_prop("Data range for the sparkline in A1:B2 notation"),
                    ),
                    (
                        "color",
                        string_prop("Primary color (CSS hex, e.g. '#4e79a7')"),
                    ),
                    (
                        "high_color",
                        string_prop("Highlight color for maximum value"),
                    ),
                    (
                        "low_color",
                        string_prop("Highlight color for minimum value"),
                    ),
                    (
                        "negative_color",
                        string_prop("Color for negative values (bar/win_loss)"),
                    ),
                    (
                        "show_markers",
                        bool_prop("Show point markers (line type only)"),
                    ),
                    (
                        "line_width",
                        number_prop("Stroke width for line sparklines"),
                    ),
                ],
                &["sheet", "cell_ref", "spark_type", "data_range"],
            ),
        },
        ToolDef {
            name: "remove_sparkline".to_string(),
            description: "Remove the sparkline from a cell".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    (
                        "cell_ref",
                        string_prop("Cell containing the sparkline (A1 notation)"),
                    ),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "list_sparklines".to_string(),
            description: "List all sparklines in a sheet".to_string(),
            input_schema: object_schema(&[("sheet", string_prop("Sheet name"))], &["sheet"]),
        },
    ]
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Arguments for add_sparkline.
#[derive(Debug, Deserialize)]
pub struct AddSparklineArgs {
    pub sheet: String,
    pub cell_ref: String,
    pub spark_type: String,
    pub data_range: String,
    pub color: Option<String>,
    pub high_color: Option<String>,
    pub low_color: Option<String>,
    pub negative_color: Option<String>,
    pub show_markers: Option<bool>,
    pub line_width: Option<f32>,
}

/// Handle the `add_sparkline` tool call.
pub fn handle_add_sparkline(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: AddSparklineArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let (start, end) = parse_range(&args.data_range)?;

    let spark_type = match args.spark_type.to_lowercase().as_str() {
        "line" => SparklineType::Line,
        "bar" => SparklineType::Bar,
        "win_loss" | "winloss" => SparklineType::WinLoss,
        _ => {
            return Err(format!(
                "Invalid sparkline type '{}': expected 'line', 'bar', or 'win_loss'",
                args.spark_type
            ));
        }
    };

    let config = SparklineConfig {
        spark_type,
        data_range: lattice_core::Range {
            start: lattice_core::CellRef {
                row: start.row,
                col: start.col,
            },
            end: lattice_core::CellRef {
                row: end.row,
                col: end.col,
            },
        },
        color: args.color,
        high_color: args.high_color,
        low_color: args.low_color,
        negative_color: args.negative_color,
        show_markers: args.show_markers.unwrap_or(false),
        line_width: args.line_width.unwrap_or(1.5),
    };

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    sheet.sparklines.add(cell.row, cell.col, config);

    Ok(json!({
        "success": true,
        "cell_ref": args.cell_ref,
        "spark_type": args.spark_type,
        "data_range": args.data_range,
    }))
}

/// Arguments for remove_sparkline.
#[derive(Debug, Deserialize)]
pub struct RemoveSparklineArgs {
    pub sheet: String,
    pub cell_ref: String,
}

/// Handle the `remove_sparkline` tool call.
pub fn handle_remove_sparkline(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: RemoveSparklineArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    let removed = sheet.sparklines.remove(cell.row, cell.col);

    Ok(json!({
        "success": true,
        "cell_ref": args.cell_ref,
        "was_present": removed,
    }))
}

/// Arguments for list_sparklines.
#[derive(Debug, Deserialize)]
pub struct ListSparklinesArgs {
    pub sheet: String,
}

/// Handle the `list_sparklines` tool call.
pub fn handle_list_sparklines(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: ListSparklinesArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    let sparklines = sheet.sparklines.list();

    let output: Vec<Value> = sparklines
        .iter()
        .map(|((row, col), config)| {
            let type_str = match config.spark_type {
                SparklineType::Line => "line",
                SparklineType::Bar => "bar",
                SparklineType::WinLoss => "win_loss",
            };
            json!({
                "cell_ref": format!("{}{}", col_to_letter(*col), row + 1),
                "spark_type": type_str,
                "data_range": format!(
                    "{}{}:{}{}",
                    col_to_letter(config.data_range.start.col),
                    config.data_range.start.row + 1,
                    col_to_letter(config.data_range.end.col),
                    config.data_range.end.row + 1
                ),
                "color": config.color,
                "high_color": config.high_color,
                "low_color": config.low_color,
                "negative_color": config.negative_color,
                "show_markers": config.show_markers,
                "line_width": config.line_width,
            })
        })
        .collect();

    Ok(json!({
        "sheet": args.sheet,
        "sparklines": output,
        "count": output.len(),
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_sparkline_line() {
        let mut wb = Workbook::new();
        let result = handle_add_sparkline(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "D1",
                "spark_type": "line",
                "data_range": "A1:C1",
                "color": "#4e79a7"
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["spark_type"], "line");

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(sheet.sparklines.get(0, 3).is_some()); // D1 = (0, 3)
    }

    #[test]
    fn test_add_sparkline_bar() {
        let mut wb = Workbook::new();
        handle_add_sparkline(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "E1",
                "spark_type": "bar",
                "data_range": "A1:D1"
            }),
        )
        .unwrap();

        let sheet = wb.get_sheet("Sheet1").unwrap();
        let config = sheet.sparklines.get(0, 4).unwrap();
        assert_eq!(config.spark_type, SparklineType::Bar);
    }

    #[test]
    fn test_add_sparkline_invalid_type() {
        let mut wb = Workbook::new();
        let result = handle_add_sparkline(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "D1",
                "spark_type": "pie",
                "data_range": "A1:C1"
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_sparkline() {
        let mut wb = Workbook::new();
        handle_add_sparkline(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "D1",
                "spark_type": "line",
                "data_range": "A1:C1"
            }),
        )
        .unwrap();

        let result =
            handle_remove_sparkline(&mut wb, json!({"sheet": "Sheet1", "cell_ref": "D1"})).unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["was_present"], true);

        let sheet = wb.get_sheet("Sheet1").unwrap();
        assert!(sheet.sparklines.get(0, 3).is_none());
    }

    #[test]
    fn test_remove_sparkline_not_present() {
        let mut wb = Workbook::new();
        let result =
            handle_remove_sparkline(&mut wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["was_present"], false);
    }

    #[test]
    fn test_list_sparklines() {
        let mut wb = Workbook::new();
        handle_add_sparkline(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "D1",
                "spark_type": "line",
                "data_range": "A1:C1"
            }),
        )
        .unwrap();
        handle_add_sparkline(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "cell_ref": "D2",
                "spark_type": "bar",
                "data_range": "A2:C2"
            }),
        )
        .unwrap();

        let result = handle_list_sparklines(&wb, json!({"sheet": "Sheet1"})).unwrap();

        assert_eq!(result["count"], 2);
    }

    #[test]
    fn test_list_sparklines_empty() {
        let wb = Workbook::new();
        let result = handle_list_sparklines(&wb, json!({"sheet": "Sheet1"})).unwrap();

        assert_eq!(result["count"], 0);
        assert!(result["sparklines"].as_array().unwrap().is_empty());
    }
}
