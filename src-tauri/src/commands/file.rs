use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::Workbook;
use lattice_io::{read_xlsx, write_xlsx};

use crate::state::AppState;

/// Summary information about an opened workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookInfo {
    /// List of sheet names.
    pub sheets: Vec<String>,
    /// The name of the active sheet.
    pub active_sheet: String,
}

/// Open an xlsx file and load it into the app state.
#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, path: String) -> Result<WorkbookInfo, String> {
    let p = Path::new(&path);
    let wb = read_xlsx(p).map_err(|e| e.to_string())?;
    let info = WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;
    Ok(info)
}

/// Save the current workbook to an xlsx file.
#[tauri::command]
pub async fn save_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let workbook = state.workbook.read().await;
    let p = Path::new(&path);
    write_xlsx(&workbook, p).map_err(|e| e.to_string())
}

/// Create a new empty workbook, replacing the current one.
#[tauri::command]
pub async fn new_workbook(state: State<'_, AppState>) -> Result<WorkbookInfo, String> {
    let wb = Workbook::new();
    let info = WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;
    Ok(info)
}
