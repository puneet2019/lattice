//! Formula operation tool handlers: evaluate_formula, get_formula, insert_formula, bulk_formula.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellRef, CellValue, FormulaEngine, Workbook};
use lattice_core::formula::evaluator::SimpleEvaluator;

use super::ToolDef;
use crate::schema::{array_prop, object_schema, string_prop};

/// Return tool definitions for formula operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "evaluate_formula".to_string(),
            description: "Evaluate a formula string against a sheet and return the result without writing it to any cell".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name to evaluate against (for cell references)")),
                    ("formula", string_prop("Formula to evaluate (without leading '=')")),
                ],
                &["sheet", "formula"],
            ),
        },
        ToolDef {
            name: "get_formula".to_string(),
            description: "Get the formula text from a specific cell. Returns null if the cell has no formula.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation (e.g. 'B3')")),
                ],
                &["sheet", "cell_ref"],
            ),
        },
        ToolDef {
            name: "insert_formula".to_string(),
            description: "Write a formula to a cell, evaluate it, and return the result".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("cell_ref", string_prop("Cell reference in A1 notation")),
                    ("formula", string_prop("Formula to insert (without leading '=')")),
                ],
                &["sheet", "cell_ref", "formula"],
            ),
        },
        ToolDef {
            name: "bulk_formula".to_string(),
            description: "Apply multiple formula operations in a single call. Each operation inserts a formula into a cell and returns the evaluated result.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    (
                        "operations",
                        array_prop(
                            "List of formula operations to apply",
                            json!({
                                "type": "object",
                                "properties": {
                                    "cell_ref": { "type": "string", "description": "Cell reference in A1 notation" },
                                    "formula": { "type": "string", "description": "Formula to insert (without leading '=')" }
                                },
                                "required": ["cell_ref", "formula"]
                            }),
                        ),
                    ),
                ],
                &["sheet", "operations"],
            ),
        },
    ]
}

/// Arguments for evaluate_formula.
#[derive(Debug, Deserialize)]
pub struct EvaluateFormulaArgs {
    pub sheet: String,
    pub formula: String,
}

/// Handle the `evaluate_formula` tool call.
pub fn handle_evaluate_formula(
    workbook: &Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: EvaluateFormulaArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;
    let evaluator = SimpleEvaluator;

    let result = evaluator
        .evaluate(&args.formula, sheet)
        .map_err(|e| format!("Formula evaluation error: {}", e))?;

    Ok(json!({
        "formula": args.formula,
        "result": cell_value_to_json(&result),
        "result_type": cell_value_type_name(&result),
    }))
}

/// Arguments for get_formula.
#[derive(Debug, Deserialize)]
pub struct GetFormulaArgs {
    pub sheet: String,
    pub cell_ref: String,
}

/// Handle the `get_formula` tool call.
pub fn handle_get_formula(
    workbook: &Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: GetFormulaArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell_ref =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    let cell = workbook
        .get_cell(&args.sheet, cell_ref.row, cell_ref.col)
        .map_err(|e| e.to_string())?;

    match cell {
        Some(c) => Ok(json!({
            "cell_ref": args.cell_ref,
            "formula": c.formula,
            "value": cell_value_to_json(&c.value),
        })),
        None => Ok(json!({
            "cell_ref": args.cell_ref,
            "formula": null,
            "value": null,
        })),
    }
}

/// Arguments for insert_formula.
#[derive(Debug, Deserialize)]
pub struct InsertFormulaArgs {
    pub sheet: String,
    pub cell_ref: String,
    pub formula: String,
}

/// Handle the `insert_formula` tool call.
pub fn handle_insert_formula(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: InsertFormulaArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let cell_ref =
        CellRef::parse(&args.cell_ref).map_err(|e| format!("Invalid cell reference: {}", e))?;

    // Evaluate the formula first to get the result value.
    let evaluator = SimpleEvaluator;
    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;
    let result = evaluator
        .evaluate(&args.formula, sheet)
        .map_err(|e| format!("Formula evaluation error: {}", e))?;

    // Write the evaluated value to the cell.
    workbook
        .set_cell(&args.sheet, cell_ref.row, cell_ref.col, result.clone())
        .map_err(|e| e.to_string())?;

    // Set the formula on the cell.
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;
    if let Some(cell) = sheet.get_cell_mut(cell_ref.row, cell_ref.col) {
        cell.formula = Some(args.formula.clone());
    }

    Ok(json!({
        "success": true,
        "cell_ref": args.cell_ref,
        "formula": args.formula,
        "result": cell_value_to_json(&result),
        "result_type": cell_value_type_name(&result),
    }))
}

/// Arguments for bulk_formula.
#[derive(Debug, Deserialize)]
pub struct BulkFormulaArgs {
    pub sheet: String,
    pub operations: Vec<FormulaOperation>,
}

/// A single formula operation within a bulk_formula call.
#[derive(Debug, Deserialize)]
pub struct FormulaOperation {
    pub cell_ref: String,
    pub formula: String,
}

/// Handle the `bulk_formula` tool call.
pub fn handle_bulk_formula(
    workbook: &mut Workbook,
    args: Value,
) -> std::result::Result<Value, String> {
    let args: BulkFormulaArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let evaluator = SimpleEvaluator;
    let mut results = Vec::new();
    let mut success_count = 0u32;
    let mut error_count = 0u32;

    for op in &args.operations {
        let cell_ref = match CellRef::parse(&op.cell_ref) {
            Ok(cr) => cr,
            Err(e) => {
                error_count += 1;
                results.push(json!({
                    "cell_ref": op.cell_ref,
                    "formula": op.formula,
                    "error": format!("Invalid cell reference: {}", e),
                }));
                continue;
            }
        };

        // Evaluate against current sheet state (which may include previous ops' results).
        let eval_result = {
            let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;
            evaluator.evaluate(&op.formula, sheet)
        };

        match eval_result {
            Ok(value) => {
                // Write the evaluated value.
                workbook
                    .set_cell(&args.sheet, cell_ref.row, cell_ref.col, value.clone())
                    .map_err(|e| e.to_string())?;

                // Set the formula.
                let sheet = workbook
                    .get_sheet_mut(&args.sheet)
                    .map_err(|e| e.to_string())?;
                if let Some(cell) = sheet.get_cell_mut(cell_ref.row, cell_ref.col) {
                    cell.formula = Some(op.formula.clone());
                }

                success_count += 1;
                results.push(json!({
                    "cell_ref": op.cell_ref,
                    "formula": op.formula,
                    "result": cell_value_to_json(&value),
                    "result_type": cell_value_type_name(&value),
                }));
            }
            Err(e) => {
                error_count += 1;
                results.push(json!({
                    "cell_ref": op.cell_ref,
                    "formula": op.formula,
                    "error": format!("Formula evaluation error: {}", e),
                }));
            }
        }
    }

    Ok(json!({
        "success": error_count == 0,
        "total": args.operations.len(),
        "succeeded": success_count,
        "failed": error_count,
        "results": results,
    }))
}

/// Convert a CellValue to a serde_json::Value for responses.
fn cell_value_to_json(cv: &CellValue) -> Value {
    match cv {
        CellValue::Empty => Value::Null,
        CellValue::Text(s) => Value::String(s.clone()),
        CellValue::Number(n) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        CellValue::Boolean(b) => Value::Bool(*b),
        CellValue::Error(e) => Value::String(e.to_string()),
        CellValue::Date(s) => Value::String(s.clone()),
    }
}

/// Return a human-readable type name for a CellValue.
fn cell_value_type_name(cv: &CellValue) -> &'static str {
    match cv {
        CellValue::Empty => "empty",
        CellValue::Text(_) => "text",
        CellValue::Number(_) => "number",
        CellValue::Boolean(_) => "boolean",
        CellValue::Error(_) => "error",
        CellValue::Date(_) => "date",
    }
}

// Tests are in a separate commit to keep commit size under 400 lines.
