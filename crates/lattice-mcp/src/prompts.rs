//! MCP prompt templates — stub implementation.
//!
//! Prompts provide pre-built instruction templates for common tasks.

use serde_json::{Value, json};

/// Handle the `prompts/list` method.
///
/// Returns available prompt templates.
pub fn handle_list_prompts() -> Result<Value, (i32, String)> {
    Ok(json!({
        "prompts": [
            {
                "name": "analyze-portfolio",
                "description": "Analyze the investment portfolio in the current spreadsheet",
                "arguments": [],
            },
            {
                "name": "clean-data",
                "description": "Identify and fix data quality issues in the selected range",
                "arguments": [
                    {
                        "name": "range",
                        "description": "The data range to clean (e.g. A1:F100)",
                        "required": false,
                    },
                ],
            },
            {
                "name": "create-dashboard",
                "description": "Create a summary dashboard from the current data",
                "arguments": [],
            },
            {
                "name": "financial-model",
                "description": "Build a financial model based on the data in the spreadsheet",
                "arguments": [],
            },
            {
                "name": "explain-formulas",
                "description": "Explain all formulas in the current sheet in plain language",
                "arguments": [],
            },
        ],
    }))
}
