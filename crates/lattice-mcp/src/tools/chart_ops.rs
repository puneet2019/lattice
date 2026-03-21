//! Chart operation tool stubs: create_chart, update_chart, delete_chart, list_charts.

use serde_json::json;

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

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
                            "Chart type: bar, line, pie, scatter, area, combo, histogram, candlestick",
                        ),
                    ),
                    ("data_range", string_prop("Data range in A1:B2 notation")),
                    (
                        "options",
                        json!({
                            "type": "object",
                            "description": "Chart options (title, legend, axes, etc.)",
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
            name: "update_chart".to_string(),
            description: "Update an existing chart's properties".to_string(),
            input_schema: object_schema(
                &[
                    ("chart_id", string_prop("Chart identifier")),
                    (
                        "updates",
                        json!({
                            "type": "object",
                            "description": "Properties to update"
                        }),
                    ),
                ],
                &["chart_id", "updates"],
            ),
        },
        ToolDef {
            name: "delete_chart".to_string(),
            description: "Delete a chart".to_string(),
            input_schema: object_schema(
                &[("chart_id", string_prop("Chart identifier"))],
                &["chart_id"],
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
    ]
}
