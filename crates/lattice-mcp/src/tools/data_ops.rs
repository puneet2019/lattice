//! Data operation tool stubs: sort_range, filter_range, find_replace, deduplicate.

use serde_json::json;

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for data operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "sort_range".to_string(),
            description: "Sort a range of cells by one or more columns".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range to sort in A1:B2 notation")),
                    (
                        "sort_by",
                        json!({
                            "type": "array",
                            "description": "Columns to sort by",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "column": {"type": "string"},
                                    "ascending": {"type": "boolean"}
                                }
                            }
                        }),
                    ),
                ],
                &["sheet", "range", "sort_by"],
            ),
        },
        ToolDef {
            name: "filter_range".to_string(),
            description: "Filter rows in a range based on conditions".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range to filter")),
                    (
                        "conditions",
                        json!({
                            "type": "array",
                            "description": "Filter conditions",
                            "items": {"type": "object"}
                        }),
                    ),
                ],
                &["sheet", "range", "conditions"],
            ),
        },
        ToolDef {
            name: "find_replace".to_string(),
            description: "Find and optionally replace text in cells".to_string(),
            input_schema: object_schema(
                &[
                    ("find", string_prop("Text to search for")),
                    (
                        "replace",
                        string_prop("Replacement text (omit for find-only)"),
                    ),
                    (
                        "sheet",
                        string_prop("Sheet to search (omit for all sheets)"),
                    ),
                    (
                        "regex",
                        json!({"type": "boolean", "description": "Treat find as regex"}),
                    ),
                ],
                &["find"],
            ),
        },
        ToolDef {
            name: "deduplicate".to_string(),
            description: "Remove duplicate rows from a range".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range to deduplicate")),
                    (
                        "columns",
                        json!({
                            "type": "array",
                            "description": "Columns to check for duplicates",
                            "items": {"type": "string"}
                        }),
                    ),
                ],
                &["sheet", "range"],
            ),
        },
    ]
}
