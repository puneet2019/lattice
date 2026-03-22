//! CSV import and export for Lattice workbooks.
//!
//! CSV files are mapped to a single sheet. When reading, the sheet name
//! is derived from the file name (without extension).

use std::path::Path;

use lattice_core::{Cell, CellValue, Workbook};

use crate::{IoError, Result};

/// Read a CSV file and return a `Workbook` with a single sheet containing
/// the CSV data.
///
/// The sheet name is derived from the file stem (e.g. `data.csv` -> `"data"`).
/// All values are either parsed as numbers or kept as text strings.
pub fn read_csv(path: &Path) -> Result<Workbook> {
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
        .from_path(path)
        .map_err(|e| IoError::Csv(e.to_string()))?;

    let mut workbook = Workbook::new();
    // Rename the default Sheet1 to the CSV file name.
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
                    ..Default::default()
                };
                sheet.set_cell(row_idx as u32, col_idx as u32, cell);
            }
        }
    }

    workbook.active_sheet = sheet_name;
    Ok(workbook)
}

/// Write the specified sheet of a workbook to a CSV file.
///
/// If `sheet_name` is `None`, the active sheet is exported.
/// Values are written as plain text. Formulas are written as their
/// computed values, not the formula text.
pub fn write_csv(workbook: &Workbook, path: &Path, sheet_name: Option<&str>) -> Result<()> {
    let name = sheet_name.unwrap_or(&workbook.active_sheet);
    let sheet = workbook.get_sheet(name).map_err(IoError::Core)?;

    let (max_row, max_col) = sheet.used_range();
    if max_row == 0 && max_col == 0 && sheet.cells().is_empty() {
        // Empty sheet — write an empty file.
        let mut writer = csv::Writer::from_path(path).map_err(|e| IoError::Csv(e.to_string()))?;
        writer.flush().map_err(IoError::Io)?;
        return Ok(());
    }

    let mut writer = csv::Writer::from_path(path).map_err(|e| IoError::Csv(e.to_string()))?;

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

/// Parse a CSV field into a `CellValue`.
///
/// Tries to parse as: boolean -> number -> text. Empty strings become `Empty`.
pub(crate) fn parse_csv_value(field: &str) -> CellValue {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        return CellValue::Empty;
    }

    // Check for booleans (case-insensitive).
    match trimmed.to_lowercase().as_str() {
        "true" => return CellValue::Boolean(true),
        "false" => return CellValue::Boolean(false),
        _ => {}
    }

    // Try parsing as a number.
    if let Ok(n) = trimmed.parse::<f64>() {
        return CellValue::Number(n);
    }

    CellValue::Text(field.to_string())
}

/// Convert a `CellValue` to a string suitable for CSV output.
pub(crate) fn cell_value_to_csv_string(value: &CellValue) -> String {
    match value {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => {
            // Use a clean representation: avoid trailing zeros.
            if *n == n.trunc() && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        CellValue::Boolean(b) | CellValue::Checkbox(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_value_empty() {
        assert_eq!(parse_csv_value(""), CellValue::Empty);
        assert_eq!(parse_csv_value("   "), CellValue::Empty);
    }

    #[test]
    fn test_parse_csv_value_bool() {
        assert_eq!(parse_csv_value("true"), CellValue::Boolean(true));
        assert_eq!(parse_csv_value("FALSE"), CellValue::Boolean(false));
    }

    #[test]
    fn test_parse_csv_value_number() {
        assert_eq!(parse_csv_value("42"), CellValue::Number(42.0));
        assert_eq!(parse_csv_value("3.14"), CellValue::Number(3.14));
        assert_eq!(parse_csv_value("-100"), CellValue::Number(-100.0));
    }

    #[test]
    fn test_parse_csv_value_text() {
        assert_eq!(
            parse_csv_value("hello world"),
            CellValue::Text("hello world".to_string())
        );
    }

    #[test]
    fn test_cell_value_to_csv_string() {
        assert_eq!(cell_value_to_csv_string(&CellValue::Empty), "");
        assert_eq!(
            cell_value_to_csv_string(&CellValue::Text("hi".into())),
            "hi"
        );
        assert_eq!(cell_value_to_csv_string(&CellValue::Number(42.0)), "42");
        assert_eq!(cell_value_to_csv_string(&CellValue::Number(3.14)), "3.14");
        assert_eq!(cell_value_to_csv_string(&CellValue::Boolean(true)), "TRUE");
    }
}
