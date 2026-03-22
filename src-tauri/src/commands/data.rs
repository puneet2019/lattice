use tauri::State;

use lattice_core::Operation;

use crate::state::AppState;

/// Find text in a sheet.
///
/// When `case_sensitive` is false, both the query and cell text are
/// compared in lowercase.
#[tauri::command]
pub async fn find_in_sheet(
    state: State<'_, AppState>,
    sheet: String,
    query: String,
    case_sensitive: Option<bool>,
) -> Result<Vec<(u32, u32)>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;

    let case_sensitive = case_sensitive.unwrap_or(true);
    let query_lower = if case_sensitive {
        query.clone()
    } else {
        query.to_lowercase()
    };

    let mut matches = Vec::new();
    for (&(row, col), cell) in s.cells() {
        let text = match &cell.value {
            lattice_core::CellValue::Text(t) => Some(t.as_str()),
            lattice_core::CellValue::Number(n) => {
                // Numbers are compared as their string representation.
                // We need to allocate, so handle below.
                let s = n.to_string();
                if case_sensitive {
                    if s.contains(&query) {
                        matches.push((row, col));
                    }
                } else if s.to_lowercase().contains(&query_lower) {
                    matches.push((row, col));
                }
                None
            }
            _ => None,
        };

        if let Some(t) = text {
            if case_sensitive {
                if t.contains(&query) {
                    matches.push((row, col));
                }
            } else if t.to_lowercase().contains(&query_lower) {
                matches.push((row, col));
            }
        }
    }
    Ok(matches)
}

/// Duplicate a sheet.
#[tauri::command]
pub async fn duplicate_sheet(
    state: State<'_, AppState>,
    source: String,
    new_name: String,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let sheet = wb.get_sheet(&source).map_err(|e| e.to_string())?.clone();
    wb.add_sheet(&new_name).map_err(|e| e.to_string())?;
    let dest = wb.get_sheet_mut(&new_name).map_err(|e| e.to_string())?;
    for (&(row, col), cell) in sheet.cells() {
        dest.set_cell(row, col, cell.clone());
    }
    Ok(())
}

/// Insert rows at the given position.
///
/// Pushes an `InsertRows` operation to the undo stack so the change
/// is reversible.
#[tauri::command]
pub async fn insert_rows(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.insert_rows(row, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::InsertRows { sheet, row, count });

    Ok(())
}

/// Delete rows at the given position.
///
/// Saves the deleted cells so undo can restore them. Pushes a
/// `DeleteRows` operation to the undo stack.
#[tauri::command]
pub async fn delete_rows(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;

    // Save cells in the rows being deleted for undo.
    let end_row = row + count;
    let deleted_cells: Vec<(u32, u32, lattice_core::Cell)> = s
        .cells()
        .iter()
        .filter(|((r, _), _)| *r >= row && *r < end_row)
        .map(|((r, c), cell)| (*r, *c, cell.clone()))
        .collect();

    s.delete_rows(row, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::DeleteRows {
        sheet,
        row,
        count,
        deleted_cells,
    });

    Ok(())
}

/// Insert columns at the given position.
///
/// Pushes an `InsertCols` operation to the undo stack so the change
/// is reversible.
#[tauri::command]
pub async fn insert_cols(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.insert_cols(col, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::InsertCols { sheet, col, count });

    Ok(())
}

/// Delete columns at the given position.
///
/// Saves the deleted cells so undo can restore them. Pushes a
/// `DeleteCols` operation to the undo stack.
#[tauri::command]
pub async fn delete_cols(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;

    // Save cells in the columns being deleted for undo.
    let end_col = col + count;
    let deleted_cells: Vec<(u32, u32, lattice_core::Cell)> = s
        .cells()
        .iter()
        .filter(|((_, c), _)| *c >= col && *c < end_col)
        .map(|((r, c), cell)| (*r, *c, cell.clone()))
        .collect();

    s.delete_cols(col, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::DeleteCols {
        sheet,
        col,
        count,
        deleted_cells,
    });

    Ok(())
}
