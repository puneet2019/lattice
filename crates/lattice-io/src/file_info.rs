//! File metadata and properties.
//!
//! Provides [`FileInfo`] which combines filesystem metadata with workbook
//! content information (sheet count, cell count, format) into a single
//! struct suitable for display in the UI or exposure via MCP.

use std::path::Path;

use serde::{Deserialize, Serialize};

use lattice_core::Workbook;

use crate::format_detect::detect_format;
use crate::{IoError, Result};

/// Summary information about a spreadsheet file on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Absolute path to the file.
    pub path: String,
    /// File name (basename) without the directory.
    pub name: String,
    /// Size of the file in bytes.
    pub size_bytes: u64,
    /// Creation timestamp in ISO 8601 format, if available.
    pub created: Option<String>,
    /// Last-modified timestamp in ISO 8601 format, if available.
    pub modified: Option<String>,
    /// Detected file format.
    pub format: String,
    /// Number of sheets in the workbook.
    pub sheet_count: usize,
    /// Total number of non-empty cells across all sheets.
    pub cell_count: usize,
}

/// Build a [`FileInfo`] from a file path and a loaded workbook.
///
/// Filesystem metadata (size, timestamps) comes from the file at `path`.
/// Content metadata (sheet count, cell count) comes from the in-memory
/// workbook.
pub fn get_file_info(path: &Path, workbook: &Workbook) -> Result<FileInfo> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    let metadata = std::fs::metadata(path)?;

    let created = metadata.created().ok().and_then(system_time_to_iso);

    let modified = metadata.modified().ok().and_then(system_time_to_iso);

    let format = detect_format(path)
        .map(|f| f.to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let sheet_count = workbook.sheet_names().len();

    let cell_count: usize = workbook
        .sheet_names()
        .iter()
        .filter_map(|name| workbook.get_sheet(name).ok())
        .map(|sheet| sheet.cells().len())
        .sum();

    Ok(FileInfo {
        path: path.display().to_string(),
        name,
        size_bytes: metadata.len(),
        created,
        modified,
        format,
        sheet_count,
        cell_count,
    })
}

/// Convert a `SystemTime` to an ISO 8601 string (UTC).
fn system_time_to_iso(time: std::time::SystemTime) -> Option<String> {
    let duration = time.duration_since(std::time::UNIX_EPOCH).ok()?;
    let secs = duration.as_secs();

    // Simple UTC formatting without pulling in chrono.
    // Seconds since epoch -> broken-down time.
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Convert days since epoch to calendar date (civil_from_days algorithm).
    let (year, month, day) = civil_from_days(days as i64);

    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    ))
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Based on Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_get_file_info_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.xlsx");

        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(42.0))
            .unwrap();

        crate::write_xlsx(&wb, &path).unwrap();

        let info = get_file_info(&path, &wb).unwrap();
        assert_eq!(info.name, "test.xlsx");
        assert_eq!(info.sheet_count, 1);
        assert_eq!(info.cell_count, 2);
        assert_eq!(info.format, "xlsx");
        assert!(info.size_bytes > 0);
        assert!(info.modified.is_some());
        assert!(info.path.ends_with("test.xlsx"));
    }

    #[test]
    fn test_get_file_info_multiple_sheets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("multi.xlsx");

        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("a".into()))
            .unwrap();
        wb.set_cell("Data", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Data", 1, 0, CellValue::Number(2.0)).unwrap();

        crate::write_xlsx(&wb, &path).unwrap();

        let info = get_file_info(&path, &wb).unwrap();
        assert_eq!(info.sheet_count, 2);
        assert_eq!(info.cell_count, 3);
    }

    #[test]
    fn test_get_file_info_not_found() {
        let wb = Workbook::new();
        let result = get_file_info(Path::new("/nonexistent/file.xlsx"), &wb);
        assert!(result.is_err());
    }

    #[test]
    fn test_system_time_to_iso() {
        // Unix epoch should be 1970-01-01T00:00:00Z.
        let epoch = std::time::UNIX_EPOCH;
        assert_eq!(
            system_time_to_iso(epoch),
            Some("1970-01-01T00:00:00Z".to_string())
        );
    }

    #[test]
    fn test_civil_from_days() {
        // Day 0 = 1970-01-01.
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // Day 1 = 1970-01-02.
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        // 2000-01-01 = day 10957.
        assert_eq!(civil_from_days(10957), (2000, 1, 1));
    }
}
