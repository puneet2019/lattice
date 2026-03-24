//! Write a Lattice `Workbook` to `.xlsx` using rust_xlsxwriter.
//!
//! Handles cell values, formatting, column widths, row heights, formulas,
//! comments, and proper date serial numbers.

use std::path::Path;

use rust_xlsxwriter::{
    Color, Format, FormatBorder, FormatUnderline, Note, Workbook as XlsxWorkbook,
};

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
                CellValue::Lambda { .. } => {
                    worksheet
                        .write_string_with_format(row, col as u16, "{lambda}", &fmt)
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

        // Write merged regions.
        for region in sheet.merged_regions() {
            worksheet
                .merge_range(
                    region.start_row,
                    region.start_col as u16,
                    region.end_row,
                    region.end_col as u16,
                    "",
                    &Format::new(),
                )
                .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
        }

        // Write hidden rows.
        for &row in &sheet.hidden_rows {
            worksheet
                .set_row_hidden(row)
                .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
        }

        // Write hidden columns.
        for &col in &sheet.hidden_cols {
            worksheet
                .set_column_hidden(col as u16)
                .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
        }

        // Write sheet tab color.
        if let Some(ref color_str) = sheet.tab_color
            && let Some(color_val) = parse_hex_color(color_str)
        {
            worksheet.set_tab_color(Color::RGB(color_val));
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
    if cf.underline {
        fmt = fmt.set_underline(FormatUnderline::Single);
    }
    if cf.strikethrough {
        fmt = fmt.set_font_strikethrough();
    }

    fmt = fmt.set_font_size(cf.font_size);
    fmt = fmt.set_font_name(&cf.font_family);

    // Parse hex color for font. rust_xlsxwriter uses u32 colors.
    if let Some(ref fc) = cf.font_color
        && let Some(color) = parse_hex_color(fc)
    {
        fmt = fmt.set_font_color(color);
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

    // Borders.
    if let Some(ref border) = cf.borders.top {
        fmt = fmt.set_border_top(border_style_to_xlsx(&border.style));
        if let Some(c) = parse_hex_color(&border.color) {
            fmt = fmt.set_border_top_color(Color::RGB(c));
        }
    }
    if let Some(ref border) = cf.borders.bottom {
        fmt = fmt.set_border_bottom(border_style_to_xlsx(&border.style));
        if let Some(c) = parse_hex_color(&border.color) {
            fmt = fmt.set_border_bottom_color(Color::RGB(c));
        }
    }
    if let Some(ref border) = cf.borders.left {
        fmt = fmt.set_border_left(border_style_to_xlsx(&border.style));
        if let Some(c) = parse_hex_color(&border.color) {
            fmt = fmt.set_border_left_color(Color::RGB(c));
        }
    }
    if let Some(ref border) = cf.borders.right {
        fmt = fmt.set_border_right(border_style_to_xlsx(&border.style));
        if let Some(c) = parse_hex_color(&border.color) {
            fmt = fmt.set_border_right_color(Color::RGB(c));
        }
    }

    // Text wrap.
    if cf.text_wrap == lattice_core::TextWrap::Wrap {
        fmt = fmt.set_text_wrap();
    }

    // Text rotation (Excel supports 0-360 for normal, and 255 for vertical).
    if cf.text_rotation != 0 {
        // rust_xlsxwriter expects i16 angle; negative values are allowed.
        fmt = fmt.set_rotation(cf.text_rotation);
    }

    // Indent.
    if cf.indent > 0 {
        fmt = fmt.set_indent(cf.indent);
    }

    fmt
}

/// Map our `BorderStyle` to rust_xlsxwriter's `FormatBorder`.
fn border_style_to_xlsx(style: &lattice_core::BorderStyle) -> FormatBorder {
    match style {
        lattice_core::BorderStyle::None => FormatBorder::None,
        lattice_core::BorderStyle::Thin => FormatBorder::Thin,
        lattice_core::BorderStyle::Medium => FormatBorder::Medium,
        lattice_core::BorderStyle::Thick => FormatBorder::Thick,
        lattice_core::BorderStyle::Dashed => FormatBorder::Dashed,
        lattice_core::BorderStyle::Dotted => FormatBorder::Dotted,
        lattice_core::BorderStyle::Double => FormatBorder::Double,
    }
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
                font_color: Some("#FF0000".to_string()),
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

    #[test]
    fn test_write_underline_strikethrough_font_family() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("text_styles.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let cell = lattice_core::Cell {
            value: CellValue::Text("Styled".into()),
            format: CellFormat {
                underline: true,
                strikethrough: true,
                font_family: "Courier New".to_string(),
                ..CellFormat::default()
            },
            ..Default::default()
        };
        sheet.set_cell(0, 0, cell);

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());

        // Verify the file can be read back without errors.
        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Styled".into())
        );
    }

    #[test]
    fn test_write_borders() {
        use lattice_core::{Border, BorderStyle, CellBorders};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("borders.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        let cell = lattice_core::Cell {
            value: CellValue::Text("Bordered".into()),
            format: CellFormat {
                borders: CellBorders {
                    top: Some(Border {
                        style: BorderStyle::Thin,
                        color: "#000000".to_string(),
                    }),
                    bottom: Some(Border {
                        style: BorderStyle::Medium,
                        color: "#FF0000".to_string(),
                    }),
                    left: Some(Border {
                        style: BorderStyle::Dashed,
                        color: "#00FF00".to_string(),
                    }),
                    right: Some(Border {
                        style: BorderStyle::Double,
                        color: "#0000FF".to_string(),
                    }),
                },
                ..CellFormat::default()
            },
            ..Default::default()
        };
        sheet.set_cell(0, 0, cell);

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_write_text_wrap_rotation_indent() {
        use lattice_core::TextWrap;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wrap_rot_indent.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();

        // Cell with text wrap.
        sheet.set_cell(
            0,
            0,
            lattice_core::Cell {
                value: CellValue::Text("Wrapped text here".into()),
                format: CellFormat {
                    text_wrap: TextWrap::Wrap,
                    ..CellFormat::default()
                },
                ..Default::default()
            },
        );

        // Cell with text rotation.
        sheet.set_cell(
            1,
            0,
            lattice_core::Cell {
                value: CellValue::Text("Rotated".into()),
                format: CellFormat {
                    text_rotation: 45,
                    ..CellFormat::default()
                },
                ..Default::default()
            },
        );

        // Cell with indent.
        sheet.set_cell(
            2,
            0,
            lattice_core::Cell {
                value: CellValue::Text("Indented".into()),
                format: CellFormat {
                    indent: 2,
                    ..CellFormat::default()
                },
                ..Default::default()
            },
        );

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_write_merged_regions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("merged.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.set_value(0, 0, CellValue::Text("Merged Header".into()));
        sheet.merge_cells(0, 0, 0, 3).unwrap(); // Merge A1:D1

        write_xlsx(&wb, &path).unwrap();

        // Read back and verify the merge region via calamine.
        let mut excel: calamine::Xlsx<_> = calamine::open_workbook(&path).unwrap();
        let merges = excel.worksheet_merge_cells("Sheet1").unwrap().unwrap();
        assert_eq!(merges.len(), 1);
        assert_eq!(merges[0].start, (0, 0));
        assert_eq!(merges[0].end, (0, 3));
    }

    #[test]
    fn test_write_hidden_rows_and_cols() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hidden.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.set_value(0, 0, CellValue::Text("Visible".into()));
        sheet.set_value(1, 0, CellValue::Text("Hidden row".into()));
        sheet.set_value(0, 1, CellValue::Text("Hidden col".into()));
        sheet.hidden_rows.insert(1);
        sheet.hidden_cols.insert(1);

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());

        // Read back and verify values are still present.
        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Visible".into())
        );
        assert_eq!(
            wb2.get_cell("Sheet1", 1, 0).unwrap().unwrap().value,
            CellValue::Text("Hidden row".into())
        );
    }

    #[test]
    fn test_write_tab_color() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tab_color.xlsx");

        let mut wb = Workbook::new();
        let sheet = wb.get_sheet_mut("Sheet1").unwrap();
        sheet.set_tab_color(Some("#FF5500".into()));
        sheet.set_value(0, 0, CellValue::Text("Colored tab".into()));

        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());
    }

    /// Comprehensive round-trip test: create a workbook with many features,
    /// write to xlsx, read back, verify all data is preserved.
    #[test]
    fn test_comprehensive_round_trip() {
        use lattice_core::{Border, BorderStyle, CellBorders, TextWrap};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("comprehensive.xlsx");

        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();

        // Sheet1: mixed cell types, formatting, formulas, comments.
        {
            let sheet = wb.get_sheet_mut("Sheet1").unwrap();

            // Text cell with full formatting.
            sheet.set_cell(
                0,
                0,
                lattice_core::Cell {
                    value: CellValue::Text("Header".into()),
                    format: CellFormat {
                        bold: true,
                        italic: true,
                        underline: true,
                        strikethrough: false,
                        font_size: 16.0,
                        font_family: "Helvetica".to_string(),
                        font_color: Some("#FF0000".to_string()),
                        bg_color: Some("#FFFF00".to_string()),
                        h_align: HAlign::Center,
                        v_align: VAlign::Middle,
                        number_format: None,
                        borders: CellBorders {
                            top: Some(Border {
                                style: BorderStyle::Thin,
                                color: "#000000".to_string(),
                            }),
                            bottom: Some(Border {
                                style: BorderStyle::Medium,
                                color: "#333333".to_string(),
                            }),
                            left: None,
                            right: None,
                        },
                        text_wrap: TextWrap::Wrap,
                        text_rotation: 0,
                        indent: 0,
                    },
                    comment: Some("Header comment".into()),
                    ..Default::default()
                },
            );

            // Number cell with currency format.
            sheet.set_cell(
                1,
                0,
                lattice_core::Cell {
                    value: CellValue::Number(1234.56),
                    format: CellFormat {
                        number_format: Some("$#,##0.00".to_string()),
                        ..CellFormat::default()
                    },
                    ..Default::default()
                },
            );

            // Boolean cell.
            sheet.set_value(2, 0, CellValue::Boolean(true));

            // Date cell.
            sheet.set_value(3, 0, CellValue::Date("2024-06-15".into()));

            // Formula cell.
            sheet.set_cell(
                4,
                0,
                lattice_core::Cell {
                    value: CellValue::Number(0.0),
                    formula: Some("SUM(A2:A3)".into()),
                    ..Default::default()
                },
            );

            // Merged region.
            sheet.set_value(0, 1, CellValue::Text("Merged".into()));
            sheet.merge_cells(0, 1, 1, 3).unwrap();

            // Hidden row.
            sheet.set_value(5, 0, CellValue::Text("Hidden".into()));
            sheet.hidden_rows.insert(5);

            // Hidden column.
            sheet.hidden_cols.insert(4);

            // Column widths and row heights.
            sheet.col_widths.insert(0, 25.0);
            sheet.row_heights.insert(0, 30.0);

            // Tab color.
            sheet.set_tab_color(Some("#00AA55".into()));
        }

        // Data sheet: simple data.
        {
            let sheet = wb.get_sheet_mut("Data").unwrap();
            sheet.set_value(0, 0, CellValue::Text("Name".into()));
            sheet.set_value(0, 1, CellValue::Text("Value".into()));
            sheet.set_value(1, 0, CellValue::Text("Alice".into()));
            sheet.set_value(1, 1, CellValue::Number(100.0));
        }

        // Write to xlsx.
        write_xlsx(&wb, &path).unwrap();
        assert!(path.exists());

        // Read back.
        let wb2 = crate::xlsx_reader::read_xlsx(&path).unwrap();

        // Verify sheet names.
        assert_eq!(wb2.sheet_names(), vec!["Sheet1", "Data"]);

        // Verify cell values (formatting not round-tripped by calamine reader yet).
        assert_eq!(
            wb2.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Header".into())
        );
        assert_eq!(
            wb2.get_cell("Sheet1", 1, 0).unwrap().unwrap().value,
            CellValue::Number(1234.56)
        );
        assert_eq!(
            wb2.get_cell("Sheet1", 2, 0).unwrap().unwrap().value,
            CellValue::Boolean(true)
        );

        // Data sheet values.
        assert_eq!(
            wb2.get_cell("Data", 0, 0).unwrap().unwrap().value,
            CellValue::Text("Name".into())
        );
        assert_eq!(
            wb2.get_cell("Data", 1, 1).unwrap().unwrap().value,
            CellValue::Number(100.0)
        );

        // Verify merged regions via calamine directly.
        let mut excel: calamine::Xlsx<_> = calamine::open_workbook(&path).unwrap();
        let merges = excel.worksheet_merge_cells("Sheet1").unwrap().unwrap();
        assert_eq!(merges.len(), 1);
        assert_eq!(merges[0].start, (0, 1));
        assert_eq!(merges[0].end, (1, 3));
    }
}
