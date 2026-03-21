use tauri::State;

use crate::state::AppState;

/// Find text in a sheet.
#[tauri::command]
pub async fn find_in_sheet(
    state: State<'_, AppState>,
    sheet: String,
    query: String,
) -> Result<Vec<(u32, u32)>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;

    let mut matches = Vec::new();
    for (&(row, col), cell) in s.cells() {
        match &cell.value {
            lattice_core::CellValue::Text(t) => {
                if t.contains(&query) {
                    matches.push((row, col));
                }
            }
            lattice_core::CellValue::Number(n) => {
                if n.to_string().contains(&query) {
                    matches.push((row, col));
                }
            }
            _ => {}
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
    Ok(())
}

/// Delete rows at the given position.
#[tauri::command]
pub async fn delete_rows(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.delete_rows(row, count);
    Ok(())
}

/// Insert columns at the given position.
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
    Ok(())
}

/// Delete columns at the given position.
#[tauri::command]
pub async fn delete_cols(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.delete_cols(col, count);
    Ok(())
}
