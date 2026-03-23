use std::path::Path;

use tauri::State;

use lattice_io::{export_print_html, write_csv, write_tsv, PrintSettings};

use crate::state::AppState;

/// Export the specified sheet as a CSV file.
///
/// If `sheet` is empty or not found, defaults to the active sheet.
#[tauri::command]
pub async fn export_csv(
    state: State<'_, AppState>,
    sheet: String,
    path: String,
) -> Result<(), String> {
    let wb = state.workbook.read().await;
    let sheet_name = if sheet.is_empty() {
        None
    } else {
        Some(sheet.as_str())
    };
    write_csv(&wb, Path::new(&path), sheet_name).map_err(|e| e.to_string())
}

/// Export the specified sheet as a TSV file.
///
/// If `sheet` is empty or not found, defaults to the active sheet.
#[tauri::command]
pub async fn export_tsv(
    state: State<'_, AppState>,
    sheet: String,
    path: String,
) -> Result<(), String> {
    let wb = state.workbook.read().await;
    let sheet_name = if sheet.is_empty() {
        None
    } else {
        Some(sheet.as_str())
    };
    write_tsv(&wb, Path::new(&path), sheet_name).map_err(|e| e.to_string())
}

/// Export the specified sheet as print-ready HTML.
///
/// Returns the HTML string which can be opened in a browser and printed to PDF.
/// If `sheet` is empty, defaults to the active sheet.
/// Accepts optional print settings to customize paper size, orientation, etc.
#[tauri::command]
pub async fn export_html(
    state: State<'_, AppState>,
    sheet: String,
    paper_size: Option<String>,
    orientation: Option<String>,
    show_gridlines: Option<bool>,
    show_headers: Option<bool>,
    scale: Option<f64>,
    margins: Option<String>,
    custom_margins: Option<[f64; 4]>,
) -> Result<String, String> {
    let wb = state.workbook.read().await;
    let sheet_name = if sheet.is_empty() {
        None
    } else {
        Some(sheet.as_str())
    };

    // Build settings from optional parameters.
    let has_any = paper_size.is_some()
        || orientation.is_some()
        || show_gridlines.is_some()
        || show_headers.is_some()
        || scale.is_some()
        || margins.is_some();

    let settings = if has_any {
        Some(PrintSettings {
            paper_size: paper_size.unwrap_or_else(|| "letter".to_string()),
            orientation: orientation.unwrap_or_else(|| "portrait".to_string()),
            show_gridlines: show_gridlines.unwrap_or(true),
            show_headers: show_headers.unwrap_or(false),
            scale: scale.unwrap_or(1.0),
            margins: margins.unwrap_or_else(|| "normal".to_string()),
            custom_margins,
        })
    } else {
        None
    };

    export_print_html(&wb, sheet_name, settings.as_ref()).map_err(|e| e.to_string())
}

/// Open a CSV file and load it as the current workbook.
#[tauri::command]
pub async fn open_csv(
    state: State<'_, AppState>,
    path: String,
) -> Result<super::file::WorkbookInfo, String> {
    let p = Path::new(&path);
    let wb = lattice_io::read_csv(p).map_err(|e| e.to_string())?;
    let info = super::file::WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;
    let mut file_path = state.file_path.write().await;
    *file_path = Some(path);
    Ok(info)
}

/// Open a TSV file and load it as the current workbook.
#[tauri::command]
pub async fn open_tsv(
    state: State<'_, AppState>,
    path: String,
) -> Result<super::file::WorkbookInfo, String> {
    let p = Path::new(&path);
    let wb = lattice_io::read_tsv(p).map_err(|e| e.to_string())?;
    let info = super::file::WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;
    let mut file_path = state.file_path.write().await;
    *file_path = Some(path);
    Ok(info)
}

/// Get the list of recently opened files.
#[tauri::command]
pub async fn get_recent_files() -> Result<Vec<lattice_io::RecentFile>, String> {
    let store = lattice_io::RecentFileStore::load();
    Ok(store.list().to_vec())
}

/// Add a file to the recent files list.
#[tauri::command]
pub async fn add_recent_file(path: String, name: String) -> Result<(), String> {
    let mut store = lattice_io::RecentFileStore::load();
    store.add(&path, &name);
    store.save().map_err(|e| e.to_string())
}
