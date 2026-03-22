use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_io::write_atomic;

use crate::state::AppState;

/// Serializable auto-save configuration for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveConfigResponse {
    pub enabled: bool,
    pub interval_secs: u64,
}

/// Get the current auto-save configuration.
#[tauri::command]
pub async fn get_autosave_config(
    state: State<'_, AppState>,
) -> Result<AutoSaveConfigResponse, String> {
    let config = state.autosave_config.read().await;
    Ok(AutoSaveConfigResponse {
        enabled: config.enabled,
        interval_secs: config.interval_secs,
    })
}

/// Update the auto-save configuration.
#[tauri::command]
pub async fn set_autosave_config(
    state: State<'_, AppState>,
    enabled: bool,
    interval_secs: u64,
) -> Result<AutoSaveConfigResponse, String> {
    let mut config = state.autosave_config.write().await;
    config.enabled = enabled;
    // Clamp to minimum 5 seconds.
    config.interval_secs = interval_secs.max(5);
    Ok(AutoSaveConfigResponse {
        enabled: config.enabled,
        interval_secs: config.interval_secs,
    })
}

/// Trigger an auto-save of the current workbook.
///
/// Saves the workbook to its current file path using atomic writes.
/// Returns an error if no file path is set (workbook was never saved).
#[tauri::command]
pub async fn trigger_autosave(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.autosave_config.read().await;
    if !config.enabled {
        return Ok(());
    }
    drop(config);

    let file_path = state.file_path.read().await;
    let path_str = file_path
        .as_deref()
        .ok_or_else(|| "no file path set: save the workbook first".to_string())?;
    let path = Path::new(path_str).to_path_buf();
    drop(file_path);

    let workbook = state.workbook.read().await;
    write_atomic(&workbook, &path).map_err(|e| e.to_string())
}
