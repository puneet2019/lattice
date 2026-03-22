//! Data validation MCP tool handlers.
//!
//! Provides tools to set, get, remove validation rules on cells, and
//! to check whether a cell's current value passes its validation rule.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellRef, ValidationRule, ValidationType, Workbook};

use super::ToolDef;
use crate::schema::{bool_prop, object_schema, string_prop};

/// Return tool definitions for validation operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "set_validation".to_string(),
            description: "Set a data validation rule on a cell. Supports list, number_range, date_range, text_length, and custom rule types.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation")),
                    ("rule_type", string_prop("Validation type: 'list', 'number_range', 'date_range', 'text_length', or 'custom'")),
                    ("list_items", json!({
                        "type": "array",
                        "description": "Allowed values for 'list' type",
                        "items": {"type": "string"}
                    })),
                    ("min", json!({
                        "description": "Minimum value for number_range, min date (ISO 8601) for date_range, or min length for text_length",
                        "oneOf": [
                            {"type": "number"},
                            {"type": "string"}
                        ]
                    })),
                    ("max", json!({
                        "description": "Maximum value for number_range, max date (ISO 8601) for date_range, or max length for text_length",
                        "oneOf": [
                            {"type": "number"},
                            {"type": "string"}
                        ]
                    })),
                    ("formula", string_prop("Custom formula string for 'custom' type")),
                    ("allow_blank", bool_prop("Whether blank cells pass validation (default true)")),
                    ("error_message", string_prop("Error message to display when validation fails")),
                ],
                &["sheet", "cell_ref", "rule_type"],
            ),
        },
        ToolDef {
            name: "get_validation".to_string(),
            description: "Get the validation rule for a cell, if any.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "remove_validation".to_string(),
            description: "Remove the validation rule from a cell.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "validate_cell".to_string(),
            description: "Check if a cell's current value passes its validation rule. Returns valid/invalid with details.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
    ]
}

// ── Argument structs ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SetValidationArgs {
    sheet: String,
    cell_ref: String,
    rule_type: String,
    list_items: Option<Vec<String>>,
    min: Option<Value>,
    max: Option<Value>,
    formula: Option<String>,
    allow_blank: Option<bool>,
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CellRefArgs {
    sheet: String,
    cell_ref: String,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// Handle the `set_validation` tool call.
pub fn handle_set_validation(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: SetValidationArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let cell = CellRef::parse(&args.cell_ref).map_err(|e| e.to_string())?;

    let validation_type = match args.rule_type.as_str() {
        "list" => {
            let items = args
                .list_items
                .ok_or("list_items is required for rule_type 'list'".to_string())?;
            if items.is_empty() {
                return Err("list_items must not be empty".to_string());
            }
            ValidationType::List(items)
        }
        "number_range" => {
            let min = args.min.as_ref().and_then(|v| v.as_f64());
            let max = args.max.as_ref().and_then(|v| v.as_f64());
            ValidationType::NumberRange { min, max }
        }
        "date_range" => {
            let min = args.min.as_ref().and_then(|v| v.as_str()).map(String::from);
            let max = args.max.as_ref().and_then(|v| v.as_str()).map(String::from);
            ValidationType::DateRange { min, max }
        }
        "text_length" => {
            let min = args
                .min
                .as_ref()
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let max = args
                .max
                .as_ref()
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            ValidationType::TextLength { min, max }
        }
        "custom" => {
            let formula = args
                .formula
                .ok_or("formula is required for rule_type 'custom'".to_string())?;
            ValidationType::Custom(formula)
        }
        other => {
            return Err(format!(
                "Unknown rule_type '{}'. Must be one of: list, number_range, date_range, text_length, custom",
                other
            ));
        }
    };

    let rule = ValidationRule {
        validation_type,
        allow_blank: args.allow_blank.unwrap_or(true),
        error_message: args.error_message,
    };

    workbook
        .validations
        .set_rule(&args.sheet, cell.row, cell.col, rule);

    Ok(json!({
        "success": true,
        "sheet": args.sheet,
        "cell_ref": args.cell_ref,
        "rule_type": args.rule_type,
    }))
}

/// Handle the `get_validation` tool call.
pub fn handle_get_validation(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: CellRefArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let cell = CellRef::parse(&args.cell_ref).map_err(|e| e.to_string())?;

    match workbook
        .validations
        .get_rule(&args.sheet, cell.row, cell.col)
    {
        Some(rule) => Ok(json!({
            "has_validation": true,
            "sheet": args.sheet,
            "cell_ref": args.cell_ref,
            "rule": format_rule(rule),
        })),
        None => Ok(json!({
            "has_validation": false,
            "sheet": args.sheet,
            "cell_ref": args.cell_ref,
        })),
    }
}

/// Handle the `remove_validation` tool call.
pub fn handle_remove_validation(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: CellRefArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let cell = CellRef::parse(&args.cell_ref).map_err(|e| e.to_string())?;

    workbook
        .validations
        .remove_rule(&args.sheet, cell.row, cell.col);

    Ok(json!({
        "success": true,
        "sheet": args.sheet,
        "cell_ref": args.cell_ref,
    }))
}

/// Handle the `validate_cell` tool call.
pub fn handle_validate_cell(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: CellRefArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    let cell = CellRef::parse(&args.cell_ref).map_err(|e| e.to_string())?;

    let rule = match workbook
        .validations
        .get_rule(&args.sheet, cell.row, cell.col)
    {
        Some(r) => r,
        None => {
            return Ok(json!({
                "has_validation": false,
                "valid": true,
                "message": "No validation rule set for this cell",
            }));
        }
    };

    // Get the cell value.
    let cell_value = workbook
        .get_cell(&args.sheet, cell.row, cell.col)
        .map_err(|e| e.to_string())?
        .map(|c| c.value.clone())
        .unwrap_or(lattice_core::CellValue::Empty);

    let is_valid = lattice_core::validation::validate(&cell_value, rule);

    let mut result = json!({
        "has_validation": true,
        "valid": is_valid,
        "cell_ref": args.cell_ref,
        "rule": format_rule(rule),
    });

    if !is_valid {
        if let Some(ref msg) = rule.error_message {
            result["error_message"] = json!(msg);
        }
    }

    Ok(result)
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Format a validation rule as JSON for tool responses.
fn format_rule(rule: &ValidationRule) -> Value {
    let type_info = match &rule.validation_type {
        ValidationType::List(items) => json!({
            "type": "list",
            "items": items,
        }),
        ValidationType::NumberRange { min, max } => json!({
            "type": "number_range",
            "min": min,
            "max": max,
        }),
        ValidationType::DateRange { min, max } => json!({
            "type": "date_range",
            "min": min,
            "max": max,
        }),
        ValidationType::TextLength { min, max } => json!({
            "type": "text_length",
            "min": min,
            "max": max,
        }),
        ValidationType::Custom(formula) => json!({
            "type": "custom",
            "formula": formula,
        }),
    };

    json!({
        "validation_type": type_info,
        "allow_blank": rule.allow_blank,
        "error_message": rule.error_message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_set_validation_number_range() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "number_range", "min": 0, "max": 100}),
        ).unwrap();
        assert_eq!(result["success"], true);
        assert!(wb.validations.get_rule("Sheet1", 0, 0).is_some());
    }

    #[test]
    fn test_set_validation_list() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "B2", "rule_type": "list", "list_items": ["Yes", "No", "Maybe"]}),
        ).unwrap();
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_set_validation_list_missing_items() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "list"}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_set_validation_custom() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "custom", "formula": "=A1>0"}),
        )
        .unwrap();
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_set_validation_unknown_type() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "nonsense"}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_set_validation_text_length() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "text_length", "min": 3, "max": 10}),
        ).unwrap();
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_set_validation_date_range() {
        let mut wb = Workbook::new();
        let result = handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "date_range", "min": "2024-01-01", "max": "2024-12-31"}),
        ).unwrap();
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_get_validation_exists() {
        let mut wb = Workbook::new();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "number_range", "min": 0, "max": 100}),
        ).unwrap();
        let result =
            handle_get_validation(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["has_validation"], true);
        assert_eq!(result["rule"]["validation_type"]["type"], "number_range");
    }

    #[test]
    fn test_get_validation_not_exists() {
        let wb = Workbook::new();
        let result =
            handle_get_validation(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["has_validation"], false);
    }

    #[test]
    fn test_remove_validation() {
        let mut wb = Workbook::new();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "number_range", "min": 0}),
        )
        .unwrap();
        let result =
            handle_remove_validation(&mut wb, json!({"sheet": "Sheet1", "cell_ref": "A1"}))
                .unwrap();
        assert_eq!(result["success"], true);
        assert!(wb.validations.get_rule("Sheet1", 0, 0).is_none());
    }

    #[test]
    fn test_validate_cell_passes() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(50.0))
            .unwrap();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "number_range", "min": 0, "max": 100}),
        ).unwrap();
        let result =
            handle_validate_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["valid"], true);
        assert_eq!(result["has_validation"], true);
    }

    #[test]
    fn test_validate_cell_fails() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(150.0))
            .unwrap();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "number_range", "min": 0, "max": 100, "error_message": "Value must be between 0 and 100"}),
        ).unwrap();
        let result =
            handle_validate_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["valid"], false);
        assert_eq!(result["error_message"], "Value must be between 0 and 100");
    }

    #[test]
    fn test_validate_cell_no_rule() {
        let wb = Workbook::new();
        let result =
            handle_validate_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["valid"], true);
        assert_eq!(result["has_validation"], false);
    }

    #[test]
    fn test_validate_cell_list_valid() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Yes".into()))
            .unwrap();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "list", "list_items": ["Yes", "No"]}),
        ).unwrap();
        let result =
            handle_validate_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["valid"], true);
    }

    #[test]
    fn test_validate_cell_list_invalid() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Maybe".into()))
            .unwrap();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "list", "list_items": ["Yes", "No"]}),
        ).unwrap();
        let result =
            handle_validate_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["valid"], false);
    }

    #[test]
    fn test_validate_cell_blank_allowed() {
        let mut wb = Workbook::new();
        handle_set_validation(
            &mut wb,
            json!({"sheet": "Sheet1", "cell_ref": "A1", "rule_type": "number_range", "min": 0, "max": 100, "allow_blank": true}),
        ).unwrap();
        let result =
            handle_validate_cell(&wb, json!({"sheet": "Sheet1", "cell_ref": "A1"})).unwrap();
        assert_eq!(result["valid"], true);
    }
}
