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
    }

    Ok(())
}
