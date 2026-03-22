//! TSV (tab-separated values) import and export for Lattice workbooks.
//!
//! Reuses the same value-parsing and serialization logic as the CSV module,
//! but uses a tab character (`\t`) as the field delimiter.

use std::path::Path;

use lattice_core::{Cell, CellValue, Workbook};

use crate::csv_io::{cell_value_to_csv_string, parse_csv_value};
use crate::{IoError, Result};

/// Read a TSV file and return a `Workbook` with a single sheet containing
/// the TSV data.
///
/// The sheet name is derived from the file stem (e.g. `data.tsv` -> `"data"`).
/// All values are either parsed as numbers, booleans, or kept as text strings.
pub fn read_tsv(path: &Path) -> Result<Workbook> {
    if !path.exists() {
        return Err(IoError::FileNotFound(path.display().to_string()));
    }

    let sheet_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Sheet1")
        .to_string();

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(b'\t')
        .from_path(path)
        .map_err(|e| IoError::Csv(e.to_string()))?;

    let mut workbook = Workbook::new();
    if sheet_name != "Sheet1" {
        workbook
            .rename_sheet("Sheet1", &sheet_name)
            .map_err(IoError::Core)?;
    }

    let sheet = workbook.get_sheet_mut(&sheet_name).map_err(IoError::Core)?;

    for (row_idx, record) in reader.records().enumerate() {
        let record = record.map_err(|e| IoError::Csv(e.to_string()))?;

        for (col_idx, field) in record.iter().enumerate() {
            let value = parse_csv_value(field);
            if value != CellValue::Empty {
                let cell = Cell {
                    value,
                    formula: None,
                    format: Default::default(),
                    style_id: 0,
                    comment: None,
                    hyperlink: None,
                };
                sheet.set_cell(row_idx as u32, col_idx as u32, cell);
            }
        }
    }

    workbook.active_sheet = sheet_name;
    Ok(workbook)
}

/// Write the specified sheet of a workbook to a TSV file.
///
/// If `sheet_name` is `None`, the active sheet is exported.
/// Values are written as plain text. Formulas are written as their
/// computed values, not the formula text.
pub fn write_tsv(workbook: &Workbook, path: &Path, sheet_name: Option<&str>) -> Result<()> {
    let name = sheet_name.unwrap_or(&workbook.active_sheet);
    let sheet = workbook.get_sheet(name).map_err(IoError::Core)?;

    let (max_row, max_col) = sheet.used_range();
    if max_row == 0 && max_col == 0 && sheet.cells().is_empty() {
        // Empty sheet -- write an empty file.
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .from_path(path)
            .map_err(|e| IoError::Csv(e.to_string()))?;
        writer.flush().map_err(IoError::Io)?;
        return Ok(());
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_path(path)
        .map_err(|e| IoError::Csv(e.to_string()))?;

    for row in 0..=max_row {
        let mut record = Vec::with_capacity((max_col + 1) as usize);
        for col in 0..=max_col {
            let value_str = match sheet.get_cell(row, col) {
                Some(cell) => cell_value_to_csv_string(&cell.value),
                None => String::new(),
            };
            record.push(value_str);
        }
        writer
            .write_record(&record)
            .map_err(|e| IoError::Csv(e.to_string()))?;
    }

    writer.flush().map_err(IoError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
