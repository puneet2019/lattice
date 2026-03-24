//! Chart operation tool handlers: create_chart, list_charts, delete_chart.
//!
//! Charts are stored in-memory as metadata. Full rendering requires the lattice-charts crate.
//! These handlers manage chart definitions that can be rendered by the frontend.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::{Value, json};

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

// A simple in-process chart store. In production, this would be part of the Workbook model.
// For now, we use a module-level LazyLock store to demonstrate the MCP API contract.

/// In-memory chart metadata.
#[derive(Debug, Clone)]
struct ChartMeta {
    id: String,
    sheet: String,
    chart_type: String,
    data_range: String,
    title: Option<String>,
    x_axis_label: Option<String>,
    y_axis_label: Option<String>,
}

/// Thread-safe chart store.
static CHART_STORE: std::sync::LazyLock<Mutex<HashMap<String, ChartMeta>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Return tool definitions for chart operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "create_chart".to_string(),
            description: "Create a new chart from a data range".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet containing the data")),
                    (
                        "chart_type",
                        string_prop(
                            "Chart type: bar, line, pie, scatter, area, combo, histogram, candlestick, treemap, waterfall, radar, bubble, gauge",
                        ),
                    ),
                    ("data_range", string_prop("Data range in A1:B2 notation")),
                    ("title", string_prop("Chart title")),
                    (
                        "options",
                        json!({
                            "type": "object",
                            "description": "Chart options",
                            "properties": {
                                "title": {"type": "string"},
                                "x_axis_label": {"type": "string"},
                                "y_axis_label": {"type": "string"}
                            }
                        }),
                    ),
                ],
                &["sheet", "chart_type", "data_range"],
            ),
        },
        ToolDef {
            name: "list_charts".to_string(),
            description: "List all charts in the workbook or a specific sheet".to_string(),
            input_schema: object_schema(
                &[("sheet", string_prop("Optional: filter by sheet name"))],
                &[],
            ),
        },
        ToolDef {
            name: "delete_chart".to_string(),
            description: "Delete a chart by its ID".to_string(),
            input_schema: object_schema(
                &[("chart_id", string_prop("Chart identifier"))],
                &["chart_id"],
            ),
        },
    ]
}

/// Arguments for create_chart.
#[derive(Debug, Deserialize)]
pub struct CreateChartArgs {
    pub sheet: String,
    pub chart_type: String,
    pub data_range: String,
    pub title: Option<String>,
    pub options: Option<ChartOptions>,
}

/// Chart options.
#[derive(Debug, Deserialize)]
pub struct ChartOptions {
    pub title: Option<String>,
    pub x_axis_label: Option<String>,
    pub y_axis_label: Option<String>,
}

/// Handle the `create_chart` tool call.
pub fn handle_create_chart(args: Value) -> Result<Value, String> {
    let args: CreateChartArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    // Validate chart type.
    let valid_types = [
        "bar",
        "line",
        "pie",
        "scatter",
        "area",
        "combo",
        "histogram",
        "candlestick",
        "treemap",
        "waterfall",
        "radar",
        "bubble",
        "gauge",
    ];
    if !valid_types.contains(&args.chart_type.as_str()) {
        return Err(format!(
            "Invalid chart type '{}'. Valid types: {}",
            args.chart_type,
            valid_types.join(", ")
        ));
    }

    let chart_id = uuid::Uuid::new_v4().to_string();

    let title = args
        .title
        .or_else(|| args.options.as_ref().and_then(|o| o.title.clone()));

    let x_label = args.options.as_ref().and_then(|o| o.x_axis_label.clone());
    let y_label = args.options.as_ref().and_then(|o| o.y_axis_label.clone());

    let meta = ChartMeta {
        id: chart_id.clone(),
        sheet: args.sheet.clone(),
        chart_type: args.chart_type.clone(),
        data_range: args.data_range.clone(),
        title: title.clone(),
        x_axis_label: x_label.clone(),
        y_axis_label: y_label.clone(),
    };

    CHART_STORE
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?
        .insert(chart_id.clone(), meta);

    Ok(json!({
        "success": true,
        "chart_id": chart_id,
        "chart_type": args.chart_type,
        "data_range": args.data_range,
        "title": title,
        "message": "Chart created. Full rendering requires the GUI frontend.",
    }))
}

/// Arguments for list_charts.
#[derive(Debug, Deserialize)]
pub struct ListChartsArgs {
    pub sheet: Option<String>,
}

/// Handle the `list_charts` tool call.
pub fn handle_list_charts(args: Value) -> Result<Value, String> {
    let args: ListChartsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let store = CHART_STORE
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?;

    let charts: Vec<Value> = store
        .values()
        .filter(|chart| args.sheet.as_ref().is_none_or(|s| chart.sheet == *s))
        .map(|chart| {
            json!({
                "chart_id": chart.id,
                "sheet": chart.sheet,
                "chart_type": chart.chart_type,
                "data_range": chart.data_range,
                "title": chart.title,
                "x_axis_label": chart.x_axis_label,
                "y_axis_label": chart.y_axis_label,
            })
        })
        .collect();

    Ok(json!({
        "count": charts.len(),
        "charts": charts,
    }))
}

/// Arguments for delete_chart.
#[derive(Debug, Deserialize)]
pub struct DeleteChartArgs {
    pub chart_id: String,
}

/// Handle the `delete_chart` tool call.
pub fn handle_delete_chart(args: Value) -> Result<Value, String> {
    let args: DeleteChartArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let mut store = CHART_STORE
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?;

    if store.remove(&args.chart_id).is_some() {
        Ok(json!({
            "success": true,
            "chart_id": args.chart_id,
        }))
    } else {
        Err(format!("Chart not found: {}", args.chart_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a chart and return its ID.
    fn create_test_chart(sheet: &str, chart_type: &str, data_range: &str) -> String {
        let result = handle_create_chart(json!({
            "sheet": sheet,
            "chart_type": chart_type,
            "data_range": data_range,
        }))
        .unwrap();
        result["chart_id"].as_str().unwrap().to_string()
    }

    #[test]
    fn test_create_chart() {
        let result = handle_create_chart(json!({
            "sheet": "Sheet1",
            "chart_type": "bar",
            "data_range": "A1:B5",
            "title": "Sales Chart"
        }))
        .unwrap();

        assert_eq!(result["success"], true);
        assert!(result["chart_id"].is_string());
        assert_eq!(result["chart_type"], "bar");

        // Cleanup: remove the chart we just created.
        let id = result["chart_id"].as_str().unwrap();
        let _ = handle_delete_chart(json!({"chart_id": id}));
    }

    #[test]
    fn test_create_chart_invalid_type() {
        let result = handle_create_chart(json!({
            "sheet": "Sheet1",
            "chart_type": "invalid",
            "data_range": "A1:B5"
        }));

        assert!(result.is_err());
    }

    #[test]
    fn test_list_charts() {
        // Create two charts with a unique sheet name to avoid interference.
        let id1 = create_test_chart("ListTest_A", "bar", "A1:B5");
        let id2 = create_test_chart("ListTest_B", "line", "A1:C10");

        // Filter by the unique sheet name — only our chart should match.
        let result = handle_list_charts(json!({"sheet": "ListTest_A"})).unwrap();
        assert_eq!(result["count"], 1);
        assert_eq!(result["charts"][0]["chart_id"], id1);

        let result = handle_list_charts(json!({"sheet": "ListTest_B"})).unwrap();
        assert_eq!(result["count"], 1);
        assert_eq!(result["charts"][0]["chart_id"], id2);

        // Cleanup.
        let _ = handle_delete_chart(json!({"chart_id": id1}));
        let _ = handle_delete_chart(json!({"chart_id": id2}));
    }

    #[test]
    fn test_delete_chart() {
        let id = create_test_chart("DeleteTest", "pie", "A1:A5");

        let result = handle_delete_chart(json!({"chart_id": id})).unwrap();
        assert_eq!(result["success"], true);

        // Verify deletion by trying to delete again — should fail.
        let result = handle_delete_chart(json!({"chart_id": id}));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_chart_not_found() {
        let result = handle_delete_chart(json!({"chart_id": "nonexistent-fixed-id-for-test"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_create_chart_with_options() {
        let result = handle_create_chart(json!({
            "sheet": "Sheet1",
            "chart_type": "scatter",
            "data_range": "A1:B10",
            "options": {
                "title": "My Chart",
                "x_axis_label": "X Values",
                "y_axis_label": "Y Values"
            }
        }))
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["title"], "My Chart");

        // Cleanup.
        let id = result["chart_id"].as_str().unwrap();
        let _ = handle_delete_chart(json!({"chart_id": id}));
    }
}
