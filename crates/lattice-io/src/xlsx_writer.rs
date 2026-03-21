//! Write a Lattice `Workbook` to `.xlsx` using rust_xlsxwriter.

use std::path::Path;

use rust_xlsxwriter::{Format, Workbook as XlsxWorkbook};

use lattice_core::{CellValue, Workbook};

use crate::{IoError, Result};

/// Write a `Workbook` to an `.xlsx` file at the given path.
///
/// Iterates over all sheets and cells, writing values with appropriate types.
/// Formatting is mapped from `CellFormat` to rust_xlsxwriter `Format`.
pub fn write_xlsx(workbook: &Workbook, path: &Path) -> Result<()> {
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
                CellValue::Boolean(b) => {
                    worksheet
                        .write_boolean_with_format(row, col as u16, *b, &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
                CellValue::Error(e) => {
                    // Write errors as text — xlsx doesn't have a direct error write method.
                    worksheet
                        .write_string_with_format(row, col as u16, e.to_string(), &fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
                CellValue::Date(s) => {
                    // Write date as a formatted string for now.
                    let date_fmt = Format::new().set_num_format("yyyy-mm-dd");
                    worksheet
                        .write_string_with_format(row, col as u16, s, &date_fmt)
                        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;
                }
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

    xlsx.save(path)
        .map_err(|e| IoError::XlsxWrite(e.to_string()))?;

    Ok(())
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
    if let Some(color) = parse_hex_color(&cf.font_color) {
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

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#FF0000"), Some(0xFF0000));
        assert_eq!(parse_hex_color("#000000"), Some(0x000000));
        assert_eq!(parse_hex_color("#FFFFFF"), Some(0xFFFFFF));
        assert_eq!(parse_hex_color("invalid"), None);
    }
}
