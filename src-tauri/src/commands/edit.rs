use tauri::State;

use lattice_core::Operation;

use crate::state::AppState;

/// Undo the last operation.
#[tauri::command]
pub async fn undo(state: State<'_, AppState>) -> Result<(), String> {
    let mut stack = state.undo_stack.write().await;
    let op = stack.undo().ok_or_else(|| "Nothing to undo".to_string())?;
    drop(stack);

    let mut workbook = state.workbook.write().await;

    match op {
        Operation::SetCell {
            sheet,
            row,
            col,
            old_value,
            ..
        } => {
            workbook
                .set_cell(&sheet, row, col, old_value)
                .map_err(|e| e.to_string())?;
        }
        Operation::AddSheet { name } => {
            workbook.remove_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RemoveSheet { name } => {
            workbook.add_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RenameSheet { old_name, new_name } => {
            workbook
                .rename_sheet(&new_name, &old_name)
                .map_err(|e| e.to_string())?;
        }
        Operation::FormatCells { sheet, cells } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            for (row, col, old_format, _new_format) in cells {
                if let Some(cell) = s.get_cell(row, col) {
                    let mut cell = cell.clone();
                    cell.format = old_format;
                    s.set_cell(row, col, cell);
                }
            }
        }
        Operation::InsertRows { sheet, row, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_rows(row, count);
        }
        Operation::DeleteRows {
            sheet,
            row,
            count,
            deleted_cells,
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            // First insert the rows back
            s.insert_rows(row, count);
            // Then restore the deleted cells
            for (r, c, cell) in deleted_cells {
                s.set_cell(r, c, cell);
            }
        }
        Operation::InsertCols { sheet, col, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_cols(col, count);
        }
        Operation::DeleteCols {
            sheet,
            col,
            count,
            deleted_cells,
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            // First insert the columns back
            s.insert_cols(col, count);
            // Then restore the deleted cells
            for (r, c, cell) in deleted_cells {
                s.set_cell(r, c, cell);
            }
        }
    }

    Ok(())
}

/// Redo the last undone operation.
#[tauri::command]
pub async fn redo(state: State<'_, AppState>) -> Result<(), String> {
    let mut stack = state.undo_stack.write().await;
    let op = stack.redo().ok_or_else(|| "Nothing to redo".to_string())?;
    drop(stack);

    let mut workbook = state.workbook.write().await;

    match op {
        Operation::SetCell {
            sheet,
            row,
            col,
            new_value,
            ..
        } => {
            workbook
                .set_cell(&sheet, row, col, new_value)
                .map_err(|e| e.to_string())?;
        }
        Operation::AddSheet { name } => {
            workbook.add_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RemoveSheet { name } => {
            workbook.remove_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RenameSheet { old_name, new_name } => {
            workbook
                .rename_sheet(&old_name, &new_name)
                .map_err(|e| e.to_string())?;
        }
        Operation::FormatCells { sheet, cells } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            for (row, col, _old_format, new_format) in cells {
                if let Some(cell) = s.get_cell(row, col) {
                    let mut cell = cell.clone();
                    cell.format = new_format;
                    s.set_cell(row, col, cell);
                }
            }
        }
        Operation::InsertRows { sheet, row, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.insert_rows(row, count);
        }
        Operation::DeleteRows {
            sheet,
            row,
            count,
            ..
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_rows(row, count);
        }
        Operation::InsertCols { sheet, col, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.insert_cols(col, count);
        }
        Operation::DeleteCols {
            sheet,
            col,
            count,
            ..
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_cols(col, count);
        }
    }

    Ok(())
}
