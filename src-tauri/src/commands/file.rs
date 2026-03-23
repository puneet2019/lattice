use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::Workbook;
use lattice_io::{read_spreadsheet, write_atomic};

use crate::state::AppState;

/// Summary information about an opened workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookInfo {
    /// List of sheet names.
    pub sheets: Vec<String>,
    /// The name of the active sheet.
    pub active_sheet: String,
}

/// Open a spreadsheet file (xlsx, xls, ods, csv, tsv) and load it into the app state.
///
/// Uses format auto-detection to pick the right reader based on magic bytes
/// and file extension.
#[tauri::command]
pub async fn open_file(state: State<'_, AppState>, path: String) -> Result<WorkbookInfo, String> {
    let p = Path::new(&path);
    let wb = read_spreadsheet(p).map_err(|e| e.to_string())?;
    let info = WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;
    // Track the file path for autosave.
    let mut file_path = state.file_path.write().await;
    *file_path = Some(path);
    Ok(info)
}

/// Save the current workbook to an xlsx file using atomic writes.
#[tauri::command]
pub async fn save_file(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let workbook = state.workbook.read().await;
    let p = Path::new(&path);
    write_atomic(&workbook, p).map_err(|e| e.to_string())?;
    drop(workbook);
    // Track the file path for autosave.
    let mut file_path = state.file_path.write().await;
    *file_path = Some(path);
    Ok(())
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
    // Clear the file path since this is a new unsaved workbook.
    let mut file_path = state.file_path.write().await;
    *file_path = None;
    Ok(info)
}
