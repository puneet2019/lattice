//! TSV (tab-separated values) import and export for Lattice workbooks.
//!
//! Reuses the same value-parsing and serialization logic as the CSV module,
//! but uses a tab character (`\t`) as the field delimiter.

#[cfg(feature = "native")]
use std::path::Path;

use lattice_core::Workbook;

use crate::csv_io::{read_delimited_str, write_delimited_string};
use crate::Result;

/// Read a TSV file and return a `Workbook` with a single sheet containing
/// the TSV data.
///
/// The sheet name is derived from the file stem (e.g. `data.tsv` -> `"data"`).
/// All values are either parsed as numbers, booleans, or kept as text strings.
#[cfg(feature = "native")]
pub fn read_tsv(path: &Path) -> Result<Workbook> {
    if !path.exists() {
        return Err(crate::IoError::FileNotFound(path.display().to_string()));
    }

    let sheet_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Sheet1")
        .to_string();

    let text = std::fs::read_to_string(path)?;
    read_tsv_str(&text, &sheet_name)
}

/// Parse TSV text into a `Workbook` with a single sheet named `sheet_name`.
///
/// WASM-available counterpart of [`read_tsv`]: works purely on an in-memory
/// string. The path-based [`read_tsv`] derives the sheet name and delegates.
pub fn read_tsv_str(text: &str, sheet_name: &str) -> Result<Workbook> {
    read_delimited_str(text, sheet_name, b'\t')
}

/// Write the specified sheet of a workbook to a TSV file.
///
/// If `sheet_name` is `None`, the active sheet is exported.
/// Values are written as plain text. Formulas are written as their
/// computed values, not the formula text.
#[cfg(feature = "native")]
pub fn write_tsv(workbook: &Workbook, path: &Path, sheet_name: Option<&str>) -> Result<()> {
    let text = write_tsv_string(workbook, sheet_name)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Serialize the specified sheet of a workbook to a TSV `String`.
///
/// WASM-available counterpart of [`write_tsv`].
pub fn write_tsv_string(workbook: &Workbook, sheet_name: Option<&str>) -> Result<String> {
    write_delimited_string(workbook, sheet_name, b'\t')
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use lattice_core::CellValue;
    use std::path::Path;

    #[test]
    fn test_write_and_read_tsv_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.tsv");

        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(42.0))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Boolean(true))
            .unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Text("World".into()))
            .unwrap();

        write_tsv(&wb, &path, None).unwrap();
        assert!(path.exists());

        // Read the raw content and verify it uses tabs
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains('\t'));
        assert!(!content.contains(','));

        let wb2 = read_tsv(&path).unwrap();
        assert_eq!(
            wb2.get_cell("test", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Hello".into())
        );
        assert_eq!(
            wb2.get_cell("test", 0, 1).unwrap().unwrap().value,
            CellValue::Number(42.0)
        );
        assert_eq!(
            wb2.get_cell("test", 1, 0).unwrap().unwrap().value,
            CellValue::Boolean(true)
        );
    }

    #[test]
    fn test_write_tsv_specific_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.tsv");

        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();
        wb.set_cell("Data", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Data", 0, 1, CellValue::Number(2.0)).unwrap();

        write_tsv(&wb, &path, Some("Data")).unwrap();

        let wb2 = read_tsv(&path).unwrap();
        assert_eq!(
            wb2.get_cell("data", 0, 0).unwrap().unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            wb2.get_cell("data", 0, 1).unwrap().unwrap().value,
            CellValue::Number(2.0)
        );
    }

    #[test]
    fn test_write_tsv_empty_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.tsv");

        let wb = Workbook::new();
        write_tsv(&wb, &path, None).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_read_tsv_file_not_found() {
        let result = read_tsv(Path::new("/nonexistent/data.tsv"));
        assert!(result.is_err());
    }

    #[test]
    fn test_tsv_sheet_name_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sales_data.tsv");
        std::fs::write(&path, "a\tb\n1\t2\n").unwrap();

        let wb = read_tsv(&path).unwrap();
        assert_eq!(wb.sheet_names(), vec!["sales_data"]);
        assert_eq!(wb.active_sheet, "sales_data");
    }
}
