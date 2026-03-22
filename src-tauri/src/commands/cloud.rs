//! Tauri commands for cloud storage operations.
//!
//! These commands expose the cloud provider abstraction to the frontend,
//! allowing the user to list, open, and save files from Google Drive,
//! iCloud Drive, and Dropbox.

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_io::cloud::{CloudFile, CloudProvider};
use lattice_io::cloud::dropbox::DropboxProvider;
use lattice_io::cloud::google_drive::GoogleDriveProvider;
use lattice_io::cloud::icloud::ICloudProvider;
use lattice_io::{read_xlsx, write_atomic};

use crate::state::AppState;

/// Serializable provider info for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfoResponse {
    pub key: String,
    pub name: String,
    pub authenticated: bool,
}

/// List available cloud providers and their authentication status.
#[tauri::command]
pub async fn list_cloud_providers() -> Result<Vec<ProviderInfoResponse>, String> {
    let providers: Vec<Box<dyn CloudProvider>> = vec![
        Box::new(GoogleDriveProvider::new()),
        Box::new(ICloudProvider::new()),
        Box::new(DropboxProvider::new()),
    ];

    let infos = providers
        .iter()
        .map(|p| ProviderInfoResponse {
            key: p.key().to_string(),
            name: p.name().to_string(),
            authenticated: p.is_authenticated(),
        })
        .collect();

    Ok(infos)
}

/// List spreadsheet files from a specific cloud provider.
#[tauri::command]
pub async fn list_cloud_files(provider: String) -> Result<Vec<CloudFile>, String> {
    let p = get_provider(&provider)?;
    p.list_files().map_err(|e| e.to_string())
}

/// Download a file from a cloud provider and open it in the workbook.
#[tauri::command]
pub async fn open_cloud_file(
    state: State<'_, AppState>,
    provider: String,
    file_id: String,
) -> Result<crate::commands::file::WorkbookInfo, String> {
    let p = get_provider(&provider)?;
    let local_path = p.download(&file_id).map_err(|e| e.to_string())?;

    let wb = read_xlsx(&local_path).map_err(|e| e.to_string())?;
    let info = crate::commands::file::WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;

    let mut file_path = state.file_path.write().await;
    *file_path = Some(local_path.to_string_lossy().to_string());

    Ok(info)
}

/// Save the current workbook to a cloud provider.
#[tauri::command]
pub async fn save_to_cloud(
    state: State<'_, AppState>,
    provider: String,
    name: String,
) -> Result<CloudFile, String> {
    let p = get_provider(&provider)?;

    // Write the workbook to a temp file first.
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("lattice-cloud-{}.xlsx", uuid::Uuid::new_v4()));

    let workbook = state.workbook.read().await;
    write_atomic(&workbook, &temp_path).map_err(|e| e.to_string())?;
    drop(workbook);

    // Upload to the cloud provider.
    let cloud_file = p
        .upload(&temp_path, &name)
        .map_err(|e| e.to_string())?;

    // Clean up the temp file.
    let _ = std::fs::remove_file(&temp_path);

    Ok(cloud_file)
}

/// Resolve a provider key string to a concrete CloudProvider instance.
fn get_provider(key: &str) -> Result<Box<dyn CloudProvider>, String> {
    match key {
        "google_drive" => Ok(Box::new(GoogleDriveProvider::new())),
        "icloud" => Ok(Box::new(ICloudProvider::new())),
        "dropbox" => Ok(Box::new(DropboxProvider::new())),
        _ => Err(format!("unknown cloud provider: {}", key)),
    }
}
