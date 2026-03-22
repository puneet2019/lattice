use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::{CellValue, FormulaEngine, NumberFormat, Operation, format_value};
use lattice_core::formula::evaluator::SimpleEvaluator;

use crate::state::AppState;

/// Serializable cell data returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    /// The display value as a string (formatted according to number_format).
    pub value: String,
    /// The raw formula text (without leading `=`), if any.
    pub formula: Option<String>,
    /// Format identifier (style_id from the cell).
    pub format_id: u32,
    /// Whether the cell is bold.
    pub bold: bool,
    /// Whether the cell is italic.
    pub italic: bool,
    /// The number format pattern string, if any.
    pub number_format: Option<String>,
}

/// Get a single cell's data.
#[tauri::command]
pub async fn get_cell(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<Option<CellData>, String> {
    let workbook = state.workbook.read().await;
    let cell = workbook
        .get_cell(&sheet, row, col)
        .map_err(|e| e.to_string())?;

    Ok(cell.map(|c| CellData {
        value: format_cell_display(&c.value, &c.format.number_format),
        formula: c.formula.clone(),
        format_id: c.style_id,
        bold: c.format.bold,
        italic: c.format.italic,
        number_format: c.format.number_format.clone(),
    }))
}

/// Set a cell's value (and optionally a formula).
///
/// When a formula is provided (value starting with `=`), the formula is
/// evaluated using [`SimpleEvaluator`] and the computed result is stored
/// as the cell's value. The raw formula text is preserved on the cell so
/// it can be shown in the formula bar.
#[tauri::command]
pub async fn set_cell(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
    value: String,
    formula: Option<String>,
) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;

    // Record the old value for undo.
    let old_value = workbook
        .get_cell(&sheet, row, col)
        .map_err(|e| e.to_string())?
        .map(|c| c.value.clone())
        .unwrap_or(CellValue::Empty);

    let new_value = if let Some(ref formula_text) = formula {
        // Evaluate the formula to get the computed value.
        let evaluator = SimpleEvaluator;
        let eval_result = {
            let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;
            evaluator.evaluate_with_context(formula_text, s, Some(&*workbook))
        };
        match eval_result {
            Ok(v) => v,
            Err(_) => CellValue::Error(lattice_core::CellError::Value),
        }
    } else {
        parse_cell_value(&value)
    };

    // Set the cell value on the sheet.
    workbook
        .set_cell(&sheet, row, col, new_value.clone())
        .map_err(|e| e.to_string())?;

    // If a formula was provided, store it on the cell.
    if let Some(ref formula_text) = formula {
        let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
        if let Some(cell) = s.get_cell(row, col) {
            let mut cell = cell.clone();
            cell.formula = Some(formula_text.clone());
            s.set_cell(row, col, cell);
        }
    }

    // Push to undo stack.
    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::SetCell {
        sheet: sheet.clone(),
        row,
        col,
        old_value,
        new_value,
    });

    // Recalculate dependent cells: any cell in this sheet that has a formula
    // might depend on the cell we just changed. Re-evaluate all formula cells.
    recalculate_formulas(&mut workbook, &sheet);

    Ok(())
}

/// Re-evaluate all formula cells on the given sheet.
///
/// This is a simple brute-force recalculation. A future optimisation would
/// build a dependency graph and only recalculate affected cells.
fn recalculate_formulas(workbook: &mut lattice_core::Workbook, sheet_name: &str) {
    // Collect all cells with formulas first (to avoid borrow conflicts).
    let formula_cells: Vec<(u32, u32, String)> = {
        let Ok(s) = workbook.get_sheet(sheet_name) else {
            return;
        };
        s.cells()
            .iter()
            .filter_map(|(&(r, c), cell)| {
                cell.formula.as_ref().map(|f| (r, c, f.clone()))
            })
            .collect()
    };

    let evaluator = SimpleEvaluator;

    for (r, c, formula_text) in formula_cells {
        let result = {
            let Ok(s) = workbook.get_sheet(sheet_name) else {
                continue;
            };
            evaluator.evaluate_with_context(&formula_text, s, Some(&*workbook))
        };
        let new_val = match result {
            Ok(v) => v,
            Err(_) => CellValue::Error(lattice_core::CellError::Value),
        };
        // Update the value without clearing the formula.
        if let Ok(s) = workbook.get_sheet_mut(sheet_name) {
            if let Some(cell) = s.get_cell(r, c) {
                let mut cell = cell.clone();
                cell.value = new_val;
                s.set_cell(r, c, cell);
            }
        }
    }
}

/// Get a rectangular range of cells.
#[tauri::command]
pub async fn get_range(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
) -> Result<Vec<Vec<Option<CellData>>>, String> {
    let workbook = state.workbook.read().await;
    let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    for r in start_row..=end_row {
        let mut row_data = Vec::new();
        for c in start_col..=end_col {
            let cell = s.get_cell(r, c);
            row_data.push(cell.map(|c| CellData {
                value: format_cell_display(&c.value, &c.format.number_format),
                formula: c.formula.clone(),
                format_id: c.style_id,
                bold: c.format.bold,
                italic: c.format.italic,
                number_format: c.format.number_format.clone(),
            }));
        }
        rows.push(row_data);
    }

    Ok(rows)
}

/// Format a cell value for display, using the core engine's `format_value`.
///
/// When a number_format pattern is set, uses `NumberFormat::Custom` (which
/// currently falls back to General). When no pattern is set, uses General.
fn format_cell_display(value: &CellValue, number_format: &Option<String>) -> String {
    let fmt = match number_format {
        Some(pattern) => NumberFormat::Custom(pattern.clone()),
        None => NumberFormat::General,
    };
    format_value(value, &fmt)
}

/// Parse a string into a `CellValue`, inferring the type.
fn parse_cell_value(s: &str) -> CellValue {
    if s.is_empty() {
        return CellValue::Empty;
    }

    // Try boolean.
    match s.to_uppercase().as_str() {
        "TRUE" => return CellValue::Boolean(true),
        "FALSE" => return CellValue::Boolean(false),
        _ => {}
    }

    // Try number.
    if let Ok(n) = s.parse::<f64>() {
        return CellValue::Number(n);
    }

    // Default to text.
    CellValue::Text(s.to_string())
}
