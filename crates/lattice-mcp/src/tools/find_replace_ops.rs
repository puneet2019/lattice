//! Find & Replace MCP tool handlers using the core find_replace module.
//!
//! Provides `find_in_workbook` (search-only) and `replace_in_workbook`
//! (search and replace) tools with full FindOptions support.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{FindOptions, Workbook, col_to_letter};

use super::ToolDef;
use crate::schema::{bool_prop, object_schema, string_prop};

/// Return tool definitions for find/replace operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "find_in_workbook".to_string(),
            description: "Search cells in the workbook for matching text or patterns. Returns all matches with their locations.".to_string(),
            input_schema: object_schema(
                &[
                    ("query", string_prop("Text or regex pattern to search for")),
                    ("case_sensitive", bool_prop("Case-sensitive search (default false)")),
                    ("whole_cell", bool_prop("Match entire cell content only (default false)")),
                    ("use_regex", bool_prop("Treat query as a regular expression (default false)")),
                    ("search_formulas", bool_prop("Search formula text instead of display values (default false)")),
                    ("sheet", string_prop("Restrict search to this sheet (omit to search all sheets)")),
                ],
                &["query"],
            ),
        },
        ToolDef {
            name: "replace_in_workbook".to_string(),
            description: "Find and replace text in workbook cells. Only text cells are modified; numbers and booleans are reported as matches but not replaced.".to_string(),
            input_schema: object_schema(
                &[
                    ("query", string_prop("Text or regex pattern to search for")),
                    ("replacement", string_prop("Replacement text")),
                    ("case_sensitive", bool_prop("Case-sensitive search (default false)")),
                    ("whole_cell", bool_prop("Match entire cell content only (default false)")),
                    ("use_regex", bool_prop("Treat query as a regular expression (default false)")),
                    ("search_formulas", bool_prop("Search/replace in formula text instead of display values (default false)")),
                    ("sheet", string_prop("Restrict replacement to this sheet (omit for all sheets)")),
                ],
                &["query", "replacement"],
            ),
        },
    ]
}

/// Arguments for find_in_workbook.
#[derive(Debug, Deserialize)]
struct FindInWorkbookArgs {
    query: String,
    case_sensitive: Option<bool>,
    whole_cell: Option<bool>,
    use_regex: Option<bool>,
    search_formulas: Option<bool>,
    sheet: Option<String>,
}

/// Handle the `find_in_workbook` tool call.
pub fn handle_find_in_workbook(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: FindInWorkbookArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let options = FindOptions {
        query: args.query,
        case_sensitive: args.case_sensitive.unwrap_or(false),
        whole_cell: args.whole_cell.unwrap_or(false),
        use_regex: args.use_regex.unwrap_or(false),
        search_formulas: args.search_formulas.unwrap_or(false),
        sheet_name: args.sheet,
    };

    let matches = lattice_core::find_replace::find(workbook, &options)
        .map_err(|e| e.to_string())?;

    let match_list: Vec<Value> = matches
        .iter()
        .map(|m| {
            let cell_label = format!("{}{}", col_to_letter(m.col), m.row + 1);
            json!({
                "sheet": m.sheet,
                "cell_ref": cell_label,
                "row": m.row,
                "col": m.col,
                "matched_text": m.matched_text,
            })
        })
        .collect();

    Ok(json!({
        "matches_found": match_list.len(),
        "matches": match_list,
    }))
}

/// Arguments for replace_in_workbook.
#[derive(Debug, Deserialize)]
struct ReplaceInWorkbookArgs {
    query: String,
    replacement: String,
    case_sensitive: Option<bool>,
    whole_cell: Option<bool>,
    use_regex: Option<bool>,
    search_formulas: Option<bool>,
    sheet: Option<String>,
}

/// Handle the `replace_in_workbook` tool call.
pub fn handle_replace_in_workbook(
    workbook: &mut Workbook,
    args: Value,
) -> Result<Value, String> {
    let args: ReplaceInWorkbookArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let options = FindOptions {
        query: args.query,
        case_sensitive: args.case_sensitive.unwrap_or(false),
        whole_cell: args.whole_cell.unwrap_or(false),
        use_regex: args.use_regex.unwrap_or(false),
        search_formulas: args.search_formulas.unwrap_or(false),
        sheet_name: args.sheet,
    };

    let result = lattice_core::find_replace::replace_all(workbook, &options, &args.replacement)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "matches_found": result.matches_found,
        "replacements_made": result.replacements_made,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_find_in_workbook_basic() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello World".into())).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello there".into())).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("goodbye".into())).unwrap();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": "hello"}),
        ).unwrap();

        assert_eq!(result["matches_found"], 2);
        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches[0]["cell_ref"], "A1");
        assert_eq!(matches[1]["cell_ref"], "A2");
    }

    #[test]
    fn test_find_in_workbook_case_sensitive() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello".into())).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello".into())).unwrap();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": "Hello", "case_sensitive": true}),
        ).unwrap();

        assert_eq!(result["matches_found"], 1);
    }

    #[test]
    fn test_find_in_workbook_regex() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("abc123".into())).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("xyz".into())).unwrap();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": "\\d+", "use_regex": true}),
        ).unwrap();

        assert_eq!(result["matches_found"], 1);
        assert_eq!(result["matches"][0]["matched_text"], "123");
    }

    #[test]
    fn test_find_in_workbook_whole_cell() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello".into())).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello world".into())).unwrap();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": "hello", "whole_cell": true}),
        ).unwrap();

        assert_eq!(result["matches_found"], 1);
        assert_eq!(result["matches"][0]["cell_ref"], "A1");
    }

    #[test]
    fn test_find_in_workbook_sheet_scope() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("apple".into())).unwrap();
        wb.add_sheet("Sheet2").unwrap();
        wb.set_cell("Sheet2", 0, 0, CellValue::Text("apple pie".into())).unwrap();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": "apple", "sheet": "Sheet2"}),
        ).unwrap();

        assert_eq!(result["matches_found"], 1);
        assert_eq!(result["matches"][0]["sheet"], "Sheet2");
    }

    #[test]
    fn test_find_in_workbook_empty_query() {
        let wb = Workbook::new();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": ""}),
        ).unwrap();

        assert_eq!(result["matches_found"], 0);
    }

    #[test]
    fn test_find_in_workbook_invalid_regex() {
        let wb = Workbook::new();

        let result = handle_find_in_workbook(
            &wb,
            json!({"query": "[bad", "use_regex": true}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_replace_in_workbook_basic() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello world".into())).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello again".into())).unwrap();

        let result = handle_replace_in_workbook(
            &mut wb,
            json!({"query": "hello", "replacement": "hi"}),
        ).unwrap();

        assert_eq!(result["matches_found"], 2);
        assert_eq!(result["replacements_made"], 2);

        let cell = wb.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        assert_eq!(cell.value, CellValue::Text("hi world".into()));
    }

    #[test]
    fn test_replace_in_workbook_skips_numbers() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0)).unwrap();

        let result = handle_replace_in_workbook(
            &mut wb,
            json!({"query": "42", "replacement": "99"}),
        ).unwrap();

        assert_eq!(result["matches_found"], 1);
        assert_eq!(result["replacements_made"], 0);
    }

    #[test]
    fn test_replace_in_workbook_regex() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("foo123bar".into())).unwrap();

        let result = handle_replace_in_workbook(
            &mut wb,
            json!({"query": "\\d+", "replacement": "NUM", "use_regex": true}),
        ).unwrap();

        assert_eq!(result["replacements_made"], 1);
        let cell = wb.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        assert_eq!(cell.value, CellValue::Text("fooNUMbar".into()));
    }
}
