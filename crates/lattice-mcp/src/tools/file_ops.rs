//! File operation tool stubs: open_file, save_file, export_csv, import_csv.

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for file operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "open_file".to_string(),
            description: "Open a spreadsheet file (xlsx, csv, etc.)".to_string(),
            input_schema: object_schema(&[("path", string_prop("File path to open"))], &["path"]),
        },
        ToolDef {
            name: "save_file".to_string(),
            description: "Save the current workbook to a file".to_string(),
            input_schema: object_schema(
                &[
                    ("path", string_prop("File path to save to")),
                    (
                        "format",
                        string_prop("File format: xlsx, csv (default: xlsx)"),
                    ),
                ],
                &[],
            ),
        },
        ToolDef {
            name: "export_csv".to_string(),
            description: "Export a sheet as CSV".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet to export")),
                    ("path", string_prop("Output file path")),
                ],
                &["sheet", "path"],
            ),
        },
        ToolDef {
            name: "import_csv".to_string(),
            description: "Import a CSV file into a new or existing sheet".to_string(),
            input_schema: object_schema(
                &[
                    ("path", string_prop("CSV file path")),
                    (
                        "sheet",
                        string_prop("Target sheet name (creates new if not exists)"),
                    ),
                ],
                &["path"],
            ),
        },
    ]
}
