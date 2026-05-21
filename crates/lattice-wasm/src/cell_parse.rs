//! Cell-value parsing and formatting helpers.
//!
//! This is a faithful port of the corresponding helpers in
//! `src-tauri/src/commands/cell.rs` — `parse_cell_value`, the currency and
//! date parsers, `date_to_serial`, `cell_to_data` / `border_to_data`, and
//! `map_error_to_cell_error`. The logic here must stay byte-for-byte
//! equivalent to the desktop version so the browser and desktop builds
//! behave identically.

use serde::{Deserialize, Serialize};

use lattice_core::{
    BorderStyle, CellError, CellValue, LatticeError, NumberFormat, format_value,
};

// ---------------------------------------------------------------------------
// Serializable cell data (matches the `CellData` interface in tauri.ts)
// ---------------------------------------------------------------------------

/// A single border edge serialized for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderEdgeData {
    pub style: String,
    pub color: String,
}

/// Cell borders serialized for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellBordersData {
    pub top: Option<BorderEdgeData>,
    pub bottom: Option<BorderEdgeData>,
    pub left: Option<BorderEdgeData>,
    pub right: Option<BorderEdgeData>,
}

/// Serializable cell data returned to the frontend.
///
/// Field names match the `CellData` TypeScript interface exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    /// The display value as a string (formatted according to number_format).
    pub value: String,
    /// The raw formula text (without leading `=`), if any.
    pub formula: Option<String>,
    /// Format identifier (style_id from the cell).
    pub format_id: u32,
    /// Whether the cell is bold.
    pub bold: bool,
    /// Whether the cell is italic.
    pub italic: bool,
    /// Whether the cell is underlined.
    pub underline: bool,
    /// Whether the cell has strikethrough.
    pub strikethrough: bool,
    /// The number format pattern string, if any.
    pub number_format: Option<String>,
    /// Font color as CSS hex string, or null for theme default.
    pub font_color: Option<String>,
    /// Background/fill color as CSS hex string, if set.
    pub bg_color: Option<String>,
    /// Font family name.
    pub font_family: String,
    /// Horizontal alignment: "left", "center", or "right".
    pub h_align: String,
    /// Vertical alignment: "top", "middle", or "bottom".
    pub v_align: String,
    /// Font size in points.
    pub font_size: f64,
    /// Text wrapping mode: "Overflow", "Wrap", or "Clip".
    pub text_wrap: String,
    /// Cell border configuration.
    pub borders: Option<CellBordersData>,
    /// Text rotation in degrees (0-360, 0 = normal).
    pub text_rotation: i16,
    /// Number of indent levels (0 = none).
    pub indent: u8,
    /// Optional comment / note text.
    pub comment: Option<String>,
}

/// Convert a core `Border` to a frontend-serializable `BorderEdgeData`.
pub fn border_to_data(border: &lattice_core::Border) -> BorderEdgeData {
    BorderEdgeData {
        style: match border.style {
            BorderStyle::None => "none".to_string(),
            BorderStyle::Thin => "thin".to_string(),
            BorderStyle::Medium => "medium".to_string(),
            BorderStyle::Thick => "thick".to_string(),
            BorderStyle::Dashed => "dashed".to_string(),
            BorderStyle::Dotted => "dotted".to_string(),
            BorderStyle::Double => "double".to_string(),
        },
        color: border.color.clone(),
    }
}

/// Convert a core `Cell` into a frontend `CellData` with all format fields.
pub fn cell_to_data(c: &lattice_core::Cell) -> CellData {
    let borders = {
        let b = &c.format.borders;
        let has_any =
            b.top.is_some() || b.bottom.is_some() || b.left.is_some() || b.right.is_some();
        if has_any {
            Some(CellBordersData {
                top: b.top.as_ref().map(border_to_data),
                bottom: b.bottom.as_ref().map(border_to_data),
                left: b.left.as_ref().map(border_to_data),
                right: b.right.as_ref().map(border_to_data),
            })
        } else {
            None
        }
    };

    CellData {
        value: format_cell_display(&c.value, &c.format.number_format),
        formula: c.formula.clone(),
        format_id: c.style_id,
        bold: c.format.bold,
        italic: c.format.italic,
        underline: c.format.underline,
        strikethrough: c.format.strikethrough,
        number_format: c.format.number_format.clone(),
        font_color: c.format.font_color.clone(),
        bg_color: c.format.bg_color.clone(),
        font_family: c.format.font_family.clone(),
        h_align: match c.format.h_align {
            lattice_core::HAlign::Left => "left".to_string(),
            lattice_core::HAlign::Center => "center".to_string(),
            lattice_core::HAlign::Right => "right".to_string(),
        },
        v_align: match c.format.v_align {
            lattice_core::VAlign::Top => "top".to_string(),
            lattice_core::VAlign::Middle => "middle".to_string(),
            lattice_core::VAlign::Bottom => "bottom".to_string(),
        },
        font_size: c.format.font_size,
        text_wrap: match c.format.text_wrap {
            lattice_core::TextWrap::Overflow => "Overflow".to_string(),
            lattice_core::TextWrap::Wrap => "Wrap".to_string(),
            lattice_core::TextWrap::Clip => "Clip".to_string(),
        },
        borders,
        text_rotation: c.format.text_rotation,
        indent: c.format.indent,
        comment: c.comment.clone(),
    }
}

/// Format a cell value for display, using the core engine's `format_value`.
pub fn format_cell_display(value: &CellValue, number_format: &Option<String>) -> String {
    let fmt = match number_format {
        Some(pattern) => NumberFormat::Custom(pattern.clone()),
        None => NumberFormat::General,
    };
    format_value(value, &fmt)
}

/// Map a [`LatticeError`] to the most appropriate [`CellError`] variant.
pub fn map_error_to_cell_error(err: &LatticeError) -> CellError {
    match err {
        LatticeError::FormulaError(msg) => {
            let upper = msg.to_uppercase();
            if upper.contains("DIV") || upper.contains("DIVISION") {
                CellError::DivZero
            } else if upper.contains("REF") {
                CellError::Ref
            } else if upper.contains("NAME") {
                CellError::Name
            } else if upper.contains("N/A") {
                CellError::NA
            } else if upper.contains("NUM") {
                CellError::Num
            } else {
                CellError::Value
            }
        }
        _ => CellError::Value,
    }
}

// ---------------------------------------------------------------------------
// Value parsing
// ---------------------------------------------------------------------------

/// Parse a string into a `CellValue`, inferring the type.
///
/// Returns `(CellValue, Option<String>)` where the second element is an
/// optional number format pattern to apply to the cell (e.g. "0%" for
/// percentage input).
pub fn parse_cell_value(s: &str) -> (CellValue, Option<String>) {
    if s.is_empty() {
        return (CellValue::Empty, None);
    }

    // Try boolean.
    match s.to_uppercase().as_str() {
        "TRUE" => return (CellValue::Boolean(true), None),
        "FALSE" => return (CellValue::Boolean(false), None),
        _ => {}
    }

    // Try percentage: trailing `%` means divide by 100 and format as percent.
    if let Some(before_pct) = s.strip_suffix('%') {
        let trimmed = before_pct.trim();
        if let Ok(n) = trimmed.parse::<f64>() {
            return (CellValue::Number(n / 100.0), Some("0%".to_string()));
        }
    }

    // Try number.
    if let Ok(n) = s.parse::<f64>() {
        return (CellValue::Number(n), None);
    }

    // Try currency: leading $, EUR, GBP, JPY symbols with optional commas.
    if let Some(result) = try_parse_currency(s) {
        return result;
    }

    // Try date: various common date formats.
    if let Some(result) = try_parse_date(s) {
        return result;
    }

    // Default to text.
    (CellValue::Text(s.to_string()), None)
}

/// Try to parse a currency string like "$1,234.56", "EUR1234", etc.
fn try_parse_currency(s: &str) -> Option<(CellValue, Option<String>)> {
    let trimmed = s.trim();

    let (rest, fmt) = if let Some(r) = trimmed.strip_prefix('$') {
        (r, "$#,##0.00")
    } else if let Some(r) = trimmed.strip_prefix('\u{20AC}') {
        // Euro sign
        (r, "\u{20AC}#,##0.00")
    } else if let Some(r) = trimmed.strip_prefix('\u{00A3}') {
        // Pound sign
        (r, "\u{00A3}#,##0.00")
    } else if let Some(r) = trimmed.strip_prefix('\u{00A5}') {
        // Yen sign
        (r, "\u{00A5}#,##0.00")
    } else {
        return None;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let without_commas: String = rest.chars().filter(|&c| c != ',').collect();
    if let Ok(n) = without_commas.parse::<f64>() {
        Some((CellValue::Number(n), Some(fmt.to_string())))
    } else {
        None
    }
}

/// Try to parse a date string into an Excel serial date number.
fn try_parse_date(s: &str) -> Option<(CellValue, Option<String>)> {
    let trimmed = s.trim();

    if let Some(result) = try_parse_mdy_slash(trimmed) {
        return Some(result);
    }
    if let Some(result) = try_parse_iso_date(trimmed) {
        return Some(result);
    }
    if let Some(result) = try_parse_dmy_month_name(trimmed) {
        return Some(result);
    }
    None
}

/// Parse M/D/YYYY or MM/DD/YYYY format.
fn try_parse_mdy_slash(s: &str) -> Option<(CellValue, Option<String>)> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: i32 = parts[2].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let serial = date_to_serial(year, month, day);
    Some((
        CellValue::Number(serial as f64),
        Some("MM/DD/YYYY".to_string()),
    ))
}

/// Parse YYYY-MM-DD (ISO 8601) format.
fn try_parse_iso_date(s: &str) -> Option<(CellValue, Option<String>)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    if parts[0].len() != 4 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let serial = date_to_serial(year, month, day);
    Some((
        CellValue::Number(serial as f64),
        Some("MM/DD/YYYY".to_string()),
    ))
}

/// Parse D-Mon-YYYY or DD-MMM-YYYY format (e.g., "15-Jan-2024").
fn try_parse_dmy_month_name(s: &str) -> Option<(CellValue, Option<String>)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let day: u32 = parts[0].parse().ok()?;
    let month = month_name_to_number(parts[1])?;
    let year: i32 = parts[2].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let serial = date_to_serial(year, month, day);
    Some((
        CellValue::Number(serial as f64),
        Some("MM/DD/YYYY".to_string()),
    ))
}

/// Convert a 3-letter month abbreviation to its 1-based month number.
fn month_name_to_number(name: &str) -> Option<u32> {
    match name.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

/// Check whether a date is valid (within reasonable spreadsheet bounds).
fn is_valid_date(year: i32, month: u32, day: u32) -> bool {
    if !(1900..=9999).contains(&year) {
        return false;
    }
    if !(1..=12).contains(&month) {
        return false;
    }
    if !(1..=31).contains(&day) {
        return false;
    }
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap { 29 } else { 28 }
        }
        _ => return false,
    };
    day <= days_in_month
}

/// Convert a date to an Excel serial date number.
///
/// Excel serial dates count days from 1900-01-01 as day 1, with the
/// intentional Lotus 1-2-3 bug that treats 1900 as a leap year.
pub fn date_to_serial(year: i32, month: u32, day: u32) -> i32 {
    let y = year as i64;
    let m = month as i64;
    let days_to_year = |yr: i64| -> i64 {
        let yr = yr - 1;
        yr * 365 + yr / 4 - yr / 100 + yr / 400
    };
    let month_days: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mut day_of_year: i64 = 0;
    for (i, &md) in month_days.iter().enumerate().take((m - 1) as usize) {
        day_of_year += md;
        if i == 1 && is_leap {
            day_of_year += 1;
        }
    }
    day_of_year += day as i64;
    let abs_days = days_to_year(y) + day_of_year;
    let base = days_to_year(1900) + 1;
    let mut serial = (abs_days - base) + 1;
    if serial >= 60 {
        serial += 1;
    }
    serial as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let (val, fmt) = parse_cell_value("");
        assert!(matches!(val, CellValue::Empty));
        assert!(fmt.is_none());
    }

    #[test]
    fn parse_boolean() {
        assert!(matches!(parse_cell_value("TRUE").0, CellValue::Boolean(true)));
        assert!(matches!(
            parse_cell_value("false").0,
            CellValue::Boolean(false)
        ));
    }

    #[test]
    fn parse_percentage() {
        let (val, fmt) = parse_cell_value("50%");
        match val {
            CellValue::Number(n) => assert!((n - 0.5).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("0%".to_string()));
    }

    #[test]
    fn parse_currency_dollar() {
        let (val, fmt) = parse_cell_value("$1,234.56");
        match val {
            CellValue::Number(n) => assert!((n - 1234.56).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("$#,##0.00".to_string()));
    }

    #[test]
    fn parse_date_iso() {
        let (val, fmt) = parse_cell_value("2024-01-15");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2024, 1, 15)),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("MM/DD/YYYY".to_string()));
    }

    #[test]
    fn date_serial_known_dates() {
        assert_eq!(date_to_serial(1900, 1, 1), 1);
        assert_eq!(date_to_serial(2000, 1, 1), 36526);
        assert_eq!(date_to_serial(2024, 1, 1), 45292);
    }

    #[test]
    fn parse_plain_text() {
        let (val, _) = parse_cell_value("hello world");
        match val {
            CellValue::Text(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected Text"),
        }
    }
}
