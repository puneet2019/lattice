use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::{CellValue, NumberFormat, Operation, format_value};

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

    let new_value = parse_cell_value(&value);

    // Set the cell value on the sheet.
    workbook
        .set_cell(&sheet, row, col, new_value.clone())
        .map_err(|e| e.to_string())?;

    // If a formula was provided, set it on the cell.
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
        sheet,
        row,
        col,
        old_value,
        new_value,
    });

    Ok(())
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
