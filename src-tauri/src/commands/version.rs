use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_io::{read_xlsx, write_xlsx};

use crate::state::AppState;

/// Information about a saved version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Index of the version (0-based, newest first).
    pub index: usize,
    /// Unix timestamp (seconds) when the version was saved.
    pub timestamp: u64,
    /// User-provided description of the version.
    pub description: String,
    /// File size in bytes.
    pub size: u64,
}

/// Get the versions directory for the current workbook.
#[allow(dead_code)]
fn versions_dir(file_path: &Option<String>) -> PathBuf {
    let filename = match file_path {
        Some(p) => {
            let path = Path::new(p);
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string()
        }
        None => "untitled".to_string(),
    };

    // ~/Library/Application Support/Lattice/versions/{filename}/
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Lattice")
        .join("versions")
        .join(filename)
}

/// Get current unix timestamp in seconds.
#[allow(dead_code)]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Save a snapshot of the current workbook as a versioned .xlsx file.
#[allow(dead_code)]
#[tauri::command]
pub async fn save_version(state: State<'_, AppState>, description: String) -> Result<(), String> {
    let wb = state.workbook.read().await;
    let fp = state.file_path.read().await;
    let dir = versions_dir(&fp);
    drop(fp);

    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ts = now_secs();
    let safe_desc = sanitize_description(&description);
    let filename = format!("{}_{}.xlsx", ts, safe_desc);
    let path = dir.join(&filename);

    write_xlsx(&wb, &path).map_err(|e| e.to_string())?;

    // Write a companion .meta file with the description and timestamp
    let meta_path = path.with_extension("meta");
    let meta = format!("{}\n{}", ts, description);
    fs::write(&meta_path, meta).map_err(|e| e.to_string())?;

    Ok(())
}

/// List all saved versions for the current workbook (newest first).
#[allow(dead_code)]
#[tauri::command]
pub async fn list_versions(state: State<'_, AppState>) -> Result<Vec<VersionInfo>, String> {
    let fp = state.file_path.read().await;
    let dir = versions_dir(&fp);
    drop(fp);

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("xlsx") {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        // Try to read companion .meta file
        let meta_path = path.with_extension("meta");
        let (timestamp, description) = if meta_path.exists() {
            let content = fs::read_to_string(&meta_path).unwrap_or_default();
            let mut lines = content.lines();
            let ts: u64 = lines.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let desc = lines.next().unwrap_or("").to_string();
            (ts, desc)
        } else {
            // Fall back to file modified time
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (modified, String::new())
        };

        versions.push(VersionInfo {
            index: 0, // assigned after sorting
            timestamp,
            description,
            size,
        });
    }

    // Sort newest first
    versions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Assign indices
    for (i, v) in versions.iter_mut().enumerate() {
        v.index = i;
    }

    Ok(versions)
}

/// Restore a previously saved version by index.
#[allow(dead_code)]
#[tauri::command]
pub async fn restore_version(
    state: State<'_, AppState>,
    index: usize,
) -> Result<super::file::WorkbookInfo, String> {
    let fp = state.file_path.read().await;
    let dir = versions_dir(&fp);
    drop(fp);

    if !dir.exists() {
        return Err("No versions found".into());
    }

    // Collect xlsx files
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    let mut xlsx_files: Vec<PathBuf> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("xlsx") {
            xlsx_files.push(path);
        }
    }

    // Sort by filename descending (newest first since filenames start with timestamp)
    xlsx_files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    let path = xlsx_files
        .get(index)
        .ok_or_else(|| format!("Version index {} out of bounds", index))?;

    let wb = read_xlsx(path).map_err(|e| e.to_string())?;
    let info = super::file::WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    state.replace_workbook(wb).await;

    Ok(info)
}

/// Sanitize a description string for use in a filename.
#[allow(dead_code)]
fn sanitize_description(desc: &str) -> String {
    desc.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(50)
        .collect()
}
