//! File format detection by extension and magic bytes.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::{IoError, Result};

/// Supported file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Excel 2007+ (.xlsx) — a ZIP archive with XML inside.
    Xlsx,
    /// Legacy Excel (.xls) — BIFF / Compound Document format.
    Xls,
    /// Comma-separated values.
    Csv,
    /// Tab-separated values.
    Tsv,
    /// OpenDocument Spreadsheet (.ods) — also a ZIP archive.
    Ods,
    /// JSON.
    Json,
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Xlsx => write!(f, "xlsx"),
            FileFormat::Xls => write!(f, "xls"),
            FileFormat::Csv => write!(f, "csv"),
            FileFormat::Tsv => write!(f, "tsv"),
            FileFormat::Ods => write!(f, "ods"),
            FileFormat::Json => write!(f, "json"),
        }
    }
}

/// Detect the file format of a file at the given path.
///
/// Uses a two-pass approach:
/// 1. Read the first few bytes to check magic bytes (ZIP for xlsx/ods, BIFF for xls).
/// 2. Fall back to file extension if magic bytes are inconclusive.
pub fn detect_format(path: &Path) -> Result<FileFormat> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    // Try magic bytes first.
    if let Some(fmt) = detect_by_magic_bytes(path)? {
        return Ok(fmt);
    }

    // Fall back to extension.
    if let Some(fmt) = detect_by_extension(path) {
        return Ok(fmt);
    }

    Err(IoError::UnsupportedFormat(path.display().to_string()))
}

/// Detect format by reading the first bytes of the file.
fn detect_by_magic_bytes(path: &Path) -> Result<Option<FileFormat>> {
    let mut file = File::open(path)?;
    let mut buf = [0u8; 8];
    let bytes_read = file.read(&mut buf)?;

    if bytes_read < 4 {
        return Ok(None);
    }

    // ZIP signature: PK\x03\x04
    // Both .xlsx and .ods are ZIP-based. We need the extension to distinguish.
    if buf[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        // It's a ZIP file. Check extension to distinguish xlsx from ods.
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("ods") => return Ok(Some(FileFormat::Ods)),
            _ => return Ok(Some(FileFormat::Xlsx)),
        }
    }

    // Compound Document File V2 (legacy .xls): D0 CF 11 E0 A1 B1 1A E1
    if bytes_read >= 8 && buf[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
        return Ok(Some(FileFormat::Xls));
    }

    // JSON files often start with '{' or '['
    if buf[0] == b'{' || buf[0] == b'[' {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        if ext.as_deref() == Some("json") {
            return Ok(Some(FileFormat::Json));
        }
    }

    Ok(None)
}

/// Detect format from file extension alone.
fn detect_by_extension(path: &Path) -> Option<FileFormat> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "xlsx" => Some(FileFormat::Xlsx),
        "xls" => Some(FileFormat::Xls),
        "csv" => Some(FileFormat::Csv),
        "tsv" | "tab" => Some(FileFormat::Tsv),
        "ods" => Some(FileFormat::Ods),
        "json" => Some(FileFormat::Json),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_by_extension() {
        assert_eq!(
            detect_by_extension(Path::new("data.xlsx")),
            Some(FileFormat::Xlsx)
        );
        assert_eq!(
            detect_by_extension(Path::new("data.csv")),
            Some(FileFormat::Csv)
        );
        assert_eq!(
            detect_by_extension(Path::new("data.tsv")),
            Some(FileFormat::Tsv)
        );
        assert_eq!(
            detect_by_extension(Path::new("data.json")),
            Some(FileFormat::Json)
        );
        assert_eq!(
            detect_by_extension(Path::new("data.xls")),
            Some(FileFormat::Xls)
        );
        assert_eq!(
            detect_by_extension(Path::new("data.ods")),
            Some(FileFormat::Ods)
        );
        assert_eq!(detect_by_extension(Path::new("data.txt")), None);
        assert_eq!(detect_by_extension(Path::new("noext")), None);
    }

    #[test]
    fn test_display_format() {
        assert_eq!(format!("{}", FileFormat::Xlsx), "xlsx");
        assert_eq!(format!("{}", FileFormat::Csv), "csv");
    }
}
