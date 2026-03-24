use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::CellValue;

use crate::state::AppState;

/// Information about the auto-filter range and active filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterInfo {
    /// Whether an auto-filter is active on this sheet.
    pub active: bool,
    /// The start column of the filter range (0-based).
    pub start_col: u32,
    /// The end column of the filter range (0-based, inclusive).
    pub end_col: u32,
    /// The header row of the filter range (0-based).
    pub header_row: u32,
    /// Columns that have active filters applied (0-based column indices).
    pub filtered_cols: Vec<u32>,
    /// Total number of data rows in the range (excluding header).
    pub total_rows: u32,
    /// Number of visible (non-hidden) rows.
    pub visible_rows: u32,
}

/// Set auto-filter on a sheet, determining the data range automatically.
///
/// The header row is assumed to be row 0 of the used range.
/// If a filter is already active, this is a no-op (use clear_filter first).
#[tauri::command]
pub async fn set_auto_filter(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<FilterInfo, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    let (max_row, max_col) = s.used_range();

    let total_rows = if max_row > 0 { max_row } else { 0 };
    let visible = (1..=max_row)
        .filter(|r| !s.is_row_hidden(*r))
        .count() as u32;

    Ok(FilterInfo {
        active: true,
        start_col: 0,
        end_col: max_col,
        header_row: 0,
        filtered_cols: Vec::new(),
        total_rows,
        visible_rows: visible,
    })
}

/// Get unique values in a column for the filter dropdown.
///
/// Returns sorted list of unique string values in the given column,
/// starting from `header_row + 1` down to the last row with data.
#[tauri::command]
pub async fn get_column_values(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
) -> Result<Vec<String>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    let (max_row, _) = s.used_range();

    let mut values = std::collections::BTreeSet::new();
    for row in 1..=max_row {
        if let Some(cell) = s.get_cell(row, col) {
            let text = cell_value_to_display(&cell.value);
            if !text.is_empty() {
                values.insert(text);
            }
        }
    }
    // Add "(Blanks)" entry if any row in the range has no data for this column
    for row in 1..=max_row {
        if s.get_cell(row, col).is_none()
            || matches!(
                s.get_cell(row, col).map(|c| &c.value),
                Some(CellValue::Empty)
            )
        {
            values.insert("(Blanks)".to_string());
            break;
        }
    }

    Ok(values.into_iter().collect())
}

/// Apply a column filter: hide rows whose values are NOT in the provided
/// list of allowed values.
#[tauri::command]
pub async fn apply_column_filter(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    values: Vec<String>,
) -> Result<FilterInfo, String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    let (max_row, max_col) = s.used_range();

    // Unhide all data rows to reset previous filter state before re-applying.
    s.unhide_rows(1, max_row);

    let allow_blanks = values.iter().any(|v| v == "(Blanks)");
    let allowed: std::collections::HashSet<String> = values
        .iter()
        .filter(|v| *v != "(Blanks)")
        .map(|v| v.to_lowercase())
        .collect();

    for row in 1..=max_row {
        let cell_val = s.get_cell(row, col).map(|c| &c.value);
        let is_blank = cell_val.is_none()
            || matches!(cell_val, Some(CellValue::Empty));

        let passes = if is_blank {
            allow_blanks
        } else {
            let text = cell_value_to_display(cell_val.unwrap()).to_lowercase();
            allowed.contains(&text)
        };

        if !passes {
            s.hide_rows(row, 1);
        }
    }

    let visible = (1..=max_row)
        .filter(|r| !s.is_row_hidden(*r))
        .count() as u32;

    Ok(FilterInfo {
        active: true,
        start_col: 0,
        end_col: max_col,
        header_row: 0,
        filtered_cols: vec![col],
        total_rows: max_row,
        visible_rows: visible,
    })
}

/// Clear all filters and unhide all rows.
#[tauri::command]
pub async fn clear_filter(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    let (max_row, _) = s.used_range();
    if max_row > 0 {
        s.unhide_rows(0, max_row + 1);
    }
    Ok(())
}

/// Get the current filter state for a sheet.
#[tauri::command]
pub async fn get_filter_info(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<FilterInfo, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    let (max_row, max_col) = s.used_range();

    let has_hidden = (1..=max_row).any(|r| s.is_row_hidden(r));
    let visible = (1..=max_row)
        .filter(|r| !s.is_row_hidden(*r))
        .count() as u32;

    Ok(FilterInfo {
        active: has_hidden,
        start_col: 0,
        end_col: max_col,
        header_row: 0,
        filtered_cols: Vec::new(),
        total_rows: max_row,
        visible_rows: visible,
    })
}

/// Get the set of hidden rows for a sheet.
#[tauri::command]
pub async fn get_hidden_rows(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Vec<u32>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    let mut rows: Vec<u32> = s.hidden_rows.iter().copied().collect();
    rows.sort();
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Filter Views — saved named filter configurations
// ---------------------------------------------------------------------------

/// Serialized filter view info returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterViewInfo {
    /// Name of the filter view.
    pub name: String,
    /// Column filters: maps column index to list of allowed values.
    pub column_filters: std::collections::HashMap<u32, Vec<String>>,
}

/// Save a named filter view to the workbook.
#[tauri::command]
pub async fn save_filter_view(
    state: State<'_, AppState>,
    name: String,
    column_filters: std::collections::HashMap<u32, Vec<String>>,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    wb.filter_views
        .add(name, column_filters)
        .map_err(|e| e.to_string())
}

/// List all saved filter views.
#[tauri::command]
pub async fn list_filter_views(
    state: State<'_, AppState>,
) -> Result<Vec<FilterViewInfo>, String> {
    let wb = state.workbook.read().await;
    Ok(wb
        .filter_views
        .list()
        .iter()
        .map(|v| FilterViewInfo {
            name: v.name.clone(),
            column_filters: v.column_filters.clone(),
        })
        .collect())
}

/// Apply a saved filter view to a sheet, hiding non-matching rows.
///
/// Returns the number of rows hidden.
#[tauri::command]
pub async fn apply_filter_view(
    state: State<'_, AppState>,
    sheet: String,
    name: String,
) -> Result<u32, String> {
    let mut wb = state.workbook.write().await;
    // We need to borrow filter_views immutably and the sheet mutably,
    // so clone the filter view first.
    let view = wb
        .filter_views
        .get(&name)
        .cloned()
        .ok_or_else(|| format!("filter view '{}' not found", name))?;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    lattice_core::filter_view::apply_filter_view(s, &view).map_err(|e| e.to_string())
}

/// Delete a saved filter view by name.
#[tauri::command]
pub async fn delete_filter_view(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    wb.filter_views.remove(&name).map_err(|e| e.to_string())
}

/// Convert a CellValue to a display string.
fn cell_value_to_display(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) => b.to_string().to_uppercase(),
        CellValue::Checkbox(b) => b.to_string().to_uppercase(),
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".to_string(),
        CellValue::Lambda { .. } => "{lambda}".to_string(),
    }
}
