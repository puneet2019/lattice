//! File format detection by extension, magic bytes, and content sniffing.

#[cfg(feature = "native")]
use std::fs::File;
#[cfg(feature = "native")]
use std::io::Read;
#[cfg(feature = "native")]
use std::path::Path;

#[cfg(feature = "native")]
use crate::IoError;
use crate::Result;

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

/// Detect the file format from in-memory bytes.
///
/// WASM-available counterpart of [`detect_format`]. Without a filename there
/// is no extension to consult, so detection relies on:
/// 1. Magic bytes (ZIP for xlsx/ods, Compound Document for xls).
/// 2. Content sniffing (JSON, TSV vs CSV by delimiter frequency).
///
/// ZIP-based files are reported as [`FileFormat::Xlsx`] (the common case);
/// callers that know the extension is `.ods` should pick the ODS reader
/// themselves. An empty or unrecognisable buffer yields an error.
pub fn detect_format_from_bytes(bytes: &[u8]) -> Result<FileFormat> {
    if bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        // ZIP archive: xlsx or ods. Without an extension, assume xlsx.
        return Ok(FileFormat::Xlsx);
    }

    if bytes.len() >= 8
        && bytes[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
    {
        return Ok(FileFormat::Xls);
    }

    if let Some(fmt) = sniff_bytes(bytes) {
        return Ok(fmt);
    }

    Err(crate::IoError::UnsupportedFormat(
        "could not determine format from bytes".to_string(),
    ))
}

/// Content-sniff a byte buffer for a text-based format (JSON, TSV, CSV).
///
/// Returns `None` for binary data or empty input.
pub(crate) fn sniff_bytes(bytes: &[u8]) -> Option<FileFormat> {
    if bytes.is_empty() || bytes.contains(&0) {
        return None;
    }

    let sample_len = bytes.len().min(8192);
    let text = std::str::from_utf8(&bytes[..sample_len]).ok()?;

    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(FileFormat::Json);
    }

    let mut tab_count = 0usize;
    let mut comma_count = 0usize;
    for line in text.lines().take(20) {
        tab_count += line.matches('\t').count();
        comma_count += line.matches(',').count();
    }

    if tab_count > 0 && tab_count >= comma_count {
        return Some(FileFormat::Tsv);
    }
    if comma_count > 0 {
        return Some(FileFormat::Csv);
    }
    if text.lines().count() > 1 {
        return Some(FileFormat::Csv);
    }

    None
}

/// Detect the file format of a file at the given path.
///
/// Uses a multi-pass approach:
/// 1. Check for known-unsupported formats (`.numbers`) and return an error.
/// 2. Read the first bytes to check magic bytes (ZIP for xlsx/ods, BIFF for xls).
/// 3. Fall back to file extension.
/// 4. Sniff content for files without a recognised extension.
#[cfg(feature = "native")]
pub fn detect_format(path: &Path) -> Result<FileFormat> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    // Check for known unsupported formats.
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if ext_lower == "numbers" {
            return Err(IoError::UnsupportedFormat(
                "Apple Numbers (.numbers) files are not supported. \
                 Export as .xlsx from Numbers first."
                    .to_string(),
            ));
        }
    }

    // Try magic bytes first.
    if let Some(fmt) = detect_by_magic_bytes(path)? {
        return Ok(fmt);
    }

    // Fall back to extension.
    if let Some(fmt) = detect_by_extension(path) {
        return Ok(fmt);
    }

    // Content sniffing for files without a recognised extension.
    if let Some(fmt) = sniff_content(path)? {
        return Ok(fmt);
    }

    Err(IoError::UnsupportedFormat(format!(
        "could not determine format of '{}'",
        path.display()
    )))
}

/// Detect format by reading the first bytes of the file.
#[cfg(feature = "native")]
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
#[cfg(feature = "native")]
fn detect_by_extension(path: &Path) -> Option<FileFormat> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "xlsx" => Some(FileFormat::Xlsx),
        "xls" => Some(FileFormat::Xls),
        "csv" => Some(FileFormat::Csv),
        "tsv" | "tab" => Some(FileFormat::Tsv),
        "ods" => Some(FileFormat::Ods),
        "json" => Some(FileFormat::Json),
        // .txt files: assume CSV (comma-separated or single-column text).
        "txt" => Some(FileFormat::Csv),
        _ => None,
    }
}

/// Content sniffing for files without a recognised extension.
///
/// Reads the first few KB of the file and looks for patterns:
/// - High tab-to-comma ratio -> TSV
/// - Commas between values -> CSV
/// - Starts with `{` or `[` -> JSON
#[cfg(feature = "native")]
fn sniff_content(path: &Path) -> Result<Option<FileFormat>> {
    let mut file = File::open(path)?;
    let mut buf = vec![0u8; 8192];
    let bytes_read = file.read(&mut buf)?;
    if bytes_read == 0 {
        return Ok(None);
    }
    let sample = &buf[..bytes_read];

    // Check for binary data (NUL bytes suggest it's not a text format).
    if sample.contains(&0) {
        return Ok(None);
    }

    let text = match std::str::from_utf8(sample) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    // JSON detection.
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Ok(Some(FileFormat::Json));
    }

    // Count tabs and commas in the first few lines to distinguish TSV/CSV.
    let mut tab_count = 0usize;
    let mut comma_count = 0usize;
    for line in text.lines().take(20) {
        tab_count += line.matches('\t').count();
        comma_count += line.matches(',').count();
    }

    if tab_count > 0 && tab_count >= comma_count {
        return Ok(Some(FileFormat::Tsv));
    }
    if comma_count > 0 {
        return Ok(Some(FileFormat::Csv));
    }

    // Single-column text data: treat as CSV.
    if text.lines().count() > 1 {
        return Ok(Some(FileFormat::Csv));
    }

    Ok(None)
}

#[cfg(test)]
mod bytes_tests {
    use super::*;

    #[test]
    fn test_detect_format_from_bytes_xlsx() {
        let zip_magic = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00];
        assert_eq!(
            detect_format_from_bytes(&zip_magic).unwrap(),
            FileFormat::Xlsx
        );
    }

    #[test]
    fn test_detect_format_from_bytes_xls() {
        let cdf_magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        assert_eq!(
            detect_format_from_bytes(&cdf_magic).unwrap(),
            FileFormat::Xls
        );
    }

    #[test]
    fn test_detect_format_from_bytes_csv() {
        let csv = b"a,b,c\n1,2,3\n";
        assert_eq!(detect_format_from_bytes(csv).unwrap(), FileFormat::Csv);
    }

    #[test]
    fn test_detect_format_from_bytes_tsv() {
        let tsv = b"a\tb\tc\n1\t2\t3\n";
        assert_eq!(detect_format_from_bytes(tsv).unwrap(), FileFormat::Tsv);
    }

    #[test]
    fn test_detect_format_from_bytes_json() {
        let json = br#"{"key": "value"}"#;
        assert_eq!(detect_format_from_bytes(json).unwrap(), FileFormat::Json);
    }

    #[test]
    fn test_detect_format_from_bytes_empty_errors() {
        assert!(detect_format_from_bytes(&[]).is_err());
    }
}

#[cfg(all(test, feature = "native"))]
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
            detect_by_extension(Path::new("data.tab")),
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
        // .txt -> CSV
        assert_eq!(
            detect_by_extension(Path::new("data.txt")),
            Some(FileFormat::Csv)
        );
        assert_eq!(detect_by_extension(Path::new("noext")), None);
    }

    #[test]
    fn test_display_format() {
        assert_eq!(format!("{}", FileFormat::Xlsx), "xlsx");
        assert_eq!(format!("{}", FileFormat::Csv), "csv");
        assert_eq!(format!("{}", FileFormat::Tsv), "tsv");
    }

    #[test]
    fn test_detect_numbers_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.numbers");
        std::fs::write(&path, "fake numbers data").unwrap();

        let result = detect_format(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            IoError::UnsupportedFormat(msg) => {
                assert!(msg.contains("Numbers"));
            }
            other => panic!("expected UnsupportedFormat, got {:?}", other),
        }
    }

    #[test]
    fn test_sniff_tsv_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.unknown");
        std::fs::write(&path, "a\tb\tc\n1\t2\t3\n").unwrap();

        let fmt = detect_format(&path).unwrap();
        assert_eq!(fmt, FileFormat::Tsv);
    }

    #[test]
    fn test_sniff_csv_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.unknown");
        std::fs::write(&path, "a,b,c\n1,2,3\n").unwrap();

        let fmt = detect_format(&path).unwrap();
        assert_eq!(fmt, FileFormat::Csv);
    }

    #[test]
    fn test_sniff_json_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.unknown");
        std::fs::write(&path, r#"{"key": "value"}"#).unwrap();

        let fmt = detect_format(&path).unwrap();
        assert_eq!(fmt, FileFormat::Json);
    }

    #[test]
    fn test_txt_treated_as_csv() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "some,data\n1,2\n").unwrap();

        let fmt = detect_format(&path).unwrap();
        assert_eq!(fmt, FileFormat::Csv);
    }
}
