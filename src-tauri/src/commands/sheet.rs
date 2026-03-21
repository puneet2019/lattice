use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

/// Summary info about a sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetInfo {
    pub name: String,
    pub is_active: bool,
}

/// List all sheets with their active state.
#[tauri::command]
pub async fn list_sheets(state: State<'_, AppState>) -> Result<Vec<SheetInfo>, String> {
    let workbook = state.workbook.read().await;
    let active = &workbook.active_sheet;
    Ok(workbook
        .sheet_names()
        .into_iter()
        .map(|name| SheetInfo {
            is_active: name == *active,
            name,
        })
        .collect())
}

/// Add a new sheet with the given name.
#[tauri::command]
pub async fn add_sheet(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;
    workbook.add_sheet(&name).map_err(|e| e.to_string())
}

/// Rename an existing sheet.
#[tauri::command]
pub async fn rename_sheet(
    state: State<'_, AppState>,
    old: String,
    new_name: String,
) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;
    workbook
        .rename_sheet(&old, &new_name)
        .map_err(|e| e.to_string())
}

/// Delete a sheet by name.
#[tauri::command]
pub async fn delete_sheet(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;
    workbook.remove_sheet(&name).map_err(|e| e.to_string())
}

/// Set the active sheet.
#[tauri::command]
pub async fn set_active_sheet(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;
    // Verify the sheet exists.
    let _ = workbook.get_sheet(&name).map_err(|e| e.to_string())?;
    workbook.active_sheet = name;
    Ok(())
}
