//! Write a Lattice `Workbook` to `.xlsx` using rust_xlsxwriter.
//!
//! Handles cell values, formatting, column widths, row heights, formulas,
//! comments, and proper date serial numbers.

use std::path::Path;

use rust_xlsxwriter::{Format, Note, Workbook as XlsxWorkbook};

use lattice_core::{CellValue, Workbook};

use crate::xlsx_reader::iso_to_excel_serial;
use crate::{IoError, Result};

/// Write a `Workbook` to an `.xlsx` file at the given path.
///
/// Iterates over all sheets and cells, writing values with appropriate types.
/// Formatting is mapped from `CellFormat` to rust_xlsxwriter `Format`.
pub fn write_xlsx(workbook: &Workbook, path: &Path) -> Result<()> {
    let mut xlsx = build_xlsx_workbook(workbook)?;

    xlsx.save(path)
        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;

    Ok(())
}

/// Serialize a `Workbook` to `.xlsx` bytes in memory.
///
/// Returns the raw bytes of a valid `.xlsx` file. Useful for atomic saves
/// where the data is first written to a temp file before being renamed.
pub fn write_xlsx_to_buffer(workbook: &Workbook) -> Result<Vec<u8>> {
    let mut xlsx = build_xlsx_workbook(workbook)?;

    xlsx.save_to_buffer()
        .map_err(|e| IoError::XlsxWrite(e.to_string()))
}

/// Build a rust_xlsxwriter `Workbook` from our Lattice `Workbook`.
///
/// This is the shared implementation used by both `write_xlsx` (file path)
/// and `write_xlsx_to_buffer` (in-memory).
fn build_xlsx_workbook(workbook: &Workbook) -> Result<XlsxWorkbook> {
    let mut xlsx = XlsxWorkbook::new();

    for sheet_name in workbook.sheet_names() {
        let sheet = workbook.get_sheet(&sheet_name).map_err(IoError::Core)?;

        let worksheet = xlsx
            .add_worksheet()
            .set_name(&sheet_name)
            .map_err(|e| IoError::XlsxWrite(e.to_string()))?;

        // Write cells.
        for (&(row, col), cell) in sheet.cells() {
            let fmt = cell_format_to_xlsx_format(&cell.format);

            // If there is a formula, write the formula instead of the value.
            if let Some(ref formula_text) = cell.formula {
                let formula = rust_xlsxwriter::Formula::new(formula_text);
                worksheet
                    .write_formula_with_format(row, col as u16, formula, &fmt)
                    .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                continue;
            }

            match &cell.value {
                CellValue::Empty => {
                    // Nothing to write for empty cells.
                }
                CellValue::Text(s) => {
                    worksheet
                        .write_string_with_format(row, col as u16, s, &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
                CellValue::Number(n) => {
                    worksheet
                        .write_number_with_format(row, col as u16, *n, &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
                CellValue::Boolean(b) | CellValue::Checkbox(b) => {
                    worksheet
                        .write_boolean_with_format(row, col as u16, *b, &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
                CellValue::Error(e) => {
                    // Write errors as text.
                    worksheet
                        .write_string_with_format(row, col as u16, e.to_string(), &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
                CellValue::Date(s) => {
                    // Try to write as a proper Excel date serial number.
                    if let Some(serial) = iso_to_excel_serial(s) {
                        let date_fmt = if s.contains('T') {
                            Format::new().set_num_format("yyyy-mm-dd hh:mm:ss")
                        } else {
                            Format::new().set_num_format("yyyy-mm-dd")
                        };
                        worksheet
                            .write_number_with_format(row, col as u16, serial, &date_fmt)
                            .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                    } else {
                        // Fall back to string if we can't parse the date.
                        let date_fmt = Format::new().set_num_format("yyyy-mm-dd");
                        worksheet
                            .write_string_with_format(row, col as u16, s, &date_fmt)
                            .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                    }
                }
                CellValue::Array(_) => {
                    // Array values are written as their first element display
                    worksheet
                        .write_string_with_format(row, col as u16, "{array}", &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
            }

            // Write cell comment/note if present.
            if let Some(ref comment_text) = cell.comment {
                let note = Note::new(comment_text);
                worksheet
                    .insert_note(row, col as u16, &note)
                    .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
            }
        }

        // Write column widths.
        for (&col, &width) in &sheet.col_widths {
            worksheet
                .set_column_width(col as u16, width)
                .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
        }

        // Write row heights.
        for (&row, &height) in &sheet.row_heights {
            worksheet
                .set_row_height(row, height)
                .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
        }
    }

    Ok(xlsx)
}

/// Convert our `CellFormat` to a rust_xlsxwriter `Format`.
fn cell_format_to_xlsx_format(cf: &lattice_core::CellFormat) -> Format {
    let mut fmt = Format::new();

    if cf.bold {
        fmt = fmt.set_bold();
    }
    if cf.italic {
        fmt = fmt.set_italic();
    }

    fmt = fmt.set_font_size(cf.font_size);

    // Parse hex color for font. rust_xlsxwriter uses u32 colors.
    if let Some(ref fc) = cf.font_color {
        if let Some(color) = parse_hex_color(fc) {
            fmt = fmt.set_font_color(color);
        }
    }

    if let Some(ref bg) = cf.bg_color
        && let Some(color) = parse_hex_color(bg)
    {
        fmt = fmt.set_background_color(color);
    }

    if let Some(ref nf) = cf.number_format {
        fmt = fmt.set_num_format(nf);
    }

    // Horizontal alignment.
    fmt = match cf.h_align {
        lattice_core::HAlign::Left => fmt.set_align(rust_xlsxwriter::FormatAlign::Left),
        lattice_core::HAlign::Center => fmt.set_align(rust_xlsxwriter::FormatAlign::Center),
        lattice_core::HAlign::Right => fmt.set_align(rust_xlsxwriter::FormatAlign::Right),
    };

    // Vertical alignment.
    fmt = match cf.v_align {
        lattice_core::VAlign::Top => fmt.set_align(rust_xlsxwriter::FormatAlign::Top),
        lattice_core::VAlign::Middle => fmt.set_align(rust_xlsxwriter::FormatAlign::VerticalCenter),
        lattice_core::VAlign::Bottom => fmt.set_align(rust_xlsxwriter::FormatAlign::Bottom),
    };

    fmt
}

/// Parse a CSS hex color string like `"#FF0000"` to a u32 color value.
fn parse_hex_color(s: &str) -> Option<u32> {
    let s = s.strip_prefix('#')?;
    u32::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{CellFormat, CellValue, HAlign, VAlign, Workbook};

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#FF0000"), Some(0xFF0000));
        assert_eq!(parse_hex_color("#000000"), Some(0x000000));
        assert_eq!(parse_hex_color("#FFFFFF"), Some(0xFFFFFF));
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_write_and_read_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("roundtrip.xlsx");

        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(42.0))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Boolean(true))
            .unwrap();

        write_xlsx(&wb, &path).unwrap();

        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Hello".into())
        );
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 1).unwrap().unwrap().value,
            CellValue::Number(42.0)
        );
        assert_eq!(
            wb2.get_cell("Sheet1", 1, 0).unwrap().unwrap().value,
            CellValue::Boolean(true)
        );
    }

    #[test]
    fn test_write_date_as_serial() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("date_test.xlsx");

        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Date("2021-01-01".into()))
            .unwrap();

        write_xlsx(&wb, &path).unwrap();

        // Read back — calamine should see it as a DateTime.
        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();
        let cell = wb2.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        match &cell.value {
            CellValue::Date(s) => assert_eq!(s, "2021-01-01"),
            CellValue::Number(n) => {
                // It's valid for calamine to return it as a number.
                assert!((n - 44197.0).abs() < 1.0);
            }
            other => panic!("expected Date or Number, got {:?}", other),
        }
    }

    #[test]
    fn test_write_multiple_sheets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("multi_sheet.xlsx");

        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Sheet1 data".into()))
            .unwrap();
        wb.set_cell("Data", 0, 0, CellValue::Text("Data sheet data".into()))
            .unwrap();

        write_xlsx(&wb, &path).unwrap();

        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();
        assert_eq!(wb2.sheet_names(), vec!["Sheet1", "Data"]);
        assert_eq!(
            wb2.get_cell("Data", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Data sheet data".into())
        );
    }

    #[test]
    fn test_write_with_formatting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("formatted.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let cell = lattice_core::Cell {
            value: CellValue::Number(100.0),
            format: CellFormat {
                bold: true,
                italic: true,
                font_size: 14.0,
                font_color: "#FF0000".to_string(),
                bg_color: Some("#FFFF00".to_string()),
                h_align: HAlign::Center,
                v_align: VAlign::Middle,
                number_format: Some("#,##0.00".to_string()),
                ..CellFormat::default()
            },
            ..Default::default()
        };
        sheet.set_cell(0, 0, cell);

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_write_with_comment() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("comment.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let cell = lattice_core::Cell {
            value: CellValue::Text("Annotated".into()),
            comment: Some("This is a comment".to_string()),
            ..Default::default()
        };
        sheet.set_cell(0, 0, cell);

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_write_column_widths_and_row_heights() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sizes.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.col_widths.insert(0, 20.0);
        sheet.col_widths.insert(1, 30.0);
        sheet.row_heights.insert(0, 25.0);
        sheet.set_value(0, 0, CellValue::Text("wide".into()));

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());
    }
}
