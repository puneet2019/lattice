//! Analysis tool stubs: describe_data, correlate, trend_analysis, portfolio_summary.

use super::ToolDef;
use crate::schema::{number_prop, object_schema, string_prop};

/// Return tool definitions for analysis operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "describe_data".to_string(),
            description: "Compute descriptive statistics for a data range (mean, median, std, min, max, count)".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Data range in A1:B2 notation")),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "correlate".to_string(),
            description: "Compute the Pearson correlation coefficient between two ranges".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range_x", string_prop("First data range")),
                    ("range_y", string_prop("Second data range")),
                ],
                &["sheet", "range_x", "range_y"],
            ),
        },
        ToolDef {
            name: "trend_analysis".to_string(),
            description: "Perform linear regression and optional forecast on a data range".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Data range")),
                    ("periods", number_prop("Number of periods to forecast")),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "portfolio_summary".to_string(),
            description: "Generate a financial portfolio summary from a data range".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Portfolio data range")),
                ],
                &["sheet", "range"],
            ),
        },
    ]
}
