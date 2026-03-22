//! Atomic file saves for cloud sync compatibility.
//!
//! Writes files via a temp-file-then-rename strategy so that cloud sync
//! clients (Google Drive, iCloud, Dropbox) never observe a partially-written
//! file. The temp file is created in the system temp directory to avoid
//! polluting the user's sync folder.

use std::fs;
use std::io::Write;
use std::path::Path;

use lattice_core::Workbook;

use crate::xlsx_writer::write_xlsx_to_buffer;
use crate::{IoError, Result};

/// Write raw bytes atomically to `path`.
///
/// Steps:
/// 1. Write `data` to a temp file in the OS temp directory.
/// 2. Flush and `fsync` the temp file to ensure durability.
/// 3. Rename (atomic on POSIX) the temp file to the target path.
///
/// If the rename fails (e.g. cross-device), falls back to copy + remove.
pub fn save_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let temp_dir = std::env::temp_dir();
    let temp_name = format!(".lattice-tmp-{}.xlsx", uuid::Uuid::new_v4());
    let temp_path = temp_dir.join(temp_name);

    // Write data to temp file.
    let mut file = fs::File::create(&temp_path).map_err(|e| {
        IoError::Io(std::io::Error::new(
            e.kind(),
            format!("failed to create temp file {}: {}", temp_path.display(), e),
        ))
    })?;
    file.write_all(data)?;
    file.flush()?;
    file.sync_all()?;
    drop(file);

    // Atomic rename. On POSIX, rename() within the same filesystem is atomic.
    // When temp dir and target are on different filesystems, rename returns
    // EXDEV and we fall back to copy + remove.
    if let Err(rename_err) = fs::rename(&temp_path, path) {
        // Cross-device fallback: copy then remove temp.
        if rename_err.raw_os_error() == Some(libc::EXDEV) {
            fs::copy(&temp_path, path)?;
            // Best-effort remove of temp file.
            let _ = fs::remove_file(&temp_path);
        } else {
            // Clean up temp file before returning the error.
            let _ = fs::remove_file(&temp_path);
            return Err(IoError::Io(rename_err));
        }
    }

    Ok(())
}

/// Serialize a workbook to `.xlsx` bytes and write atomically to `path`.
///
/// This combines xlsx serialization with the atomic write strategy so the
/// file at `path` is never partially written.
pub fn write_atomic(workbook: &Workbook, path: &Path) -> Result<()> {
    let data = write_xlsx_to_buffer(workbook)?;
    save_atomic(path, &data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::CellValue;

    #[test]
    fn test_save_atomic_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.xlsx");

        save_atomic(&path, b"hello atomic world").unwrap();

        assert!(path.exists());
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, b"hello atomic world");
    }

    #[test]
    fn test_save_atomic_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.xlsx");

        std::fs::write(&path, b"old content").unwrap();
        save_atomic(&path, b"new content").unwrap();

        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, b"new content");
    }

    #[test]
    fn test_save_atomic_no_temp_file_left() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.xlsx");

        save_atomic(&path, b"data").unwrap();

        // Verify no .lattice-tmp files remain in the target directory.
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".lattice-tmp-"))
            .collect();
        assert!(
            entries.is_empty(),
            "temp files should not remain in target dir"
        );
    }

    #[test]
    fn test_write_atomic_workbook() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workbook.xlsx");

        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("atomic".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(42.0))
            .unwrap();

        write_atomic(&wb, &path).unwrap();
        assert!(path.exists());

        // Verify the file can be read back.
        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("atomic".into())
        );
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 1).unwrap().unwrap().value,
            CellValue::Number(42.0)
        );
    }

    #[test]
    fn test_save_atomic_error_on_invalid_dir() {
        let path = Path::new("/nonexistent_dir_12345/output.xlsx");
        let result = save_atomic(path, b"data");
        assert!(result.is_err());
    }
}
