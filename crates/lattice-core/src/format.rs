use serde::{Deserialize, Serialize};

use crate::cell::CellValue;

/// Horizontal alignment within a cell.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum HAlign {
    /// Align to the left edge.
    #[default]
    Left,
    /// Centre horizontally.
    Center,
    /// Align to the right edge.
    Right,
}

/// Vertical alignment within a cell.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum VAlign {
    /// Align to the top edge.
    Top,
    /// Centre vertically.
    Middle,
    /// Align to the bottom edge.
    #[default]
    Bottom,
}

/// Structured number format for a cell.
///
/// This enum represents the semantic meaning of a number format,
/// as opposed to the raw Excel-compatible format pattern string
/// stored in `CellFormat::number_format`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NumberFormat {
    /// Default -- display value as-is with smart formatting.
    General,
    /// Fixed-decimal number with thousand separators (e.g. `1,234.56`).
    Number { decimal_places: u8 },
    /// Currency with symbol prefix and thousand separators (e.g. `$1,234.56`).
    Currency { symbol: String, decimal_places: u8 },
    /// Percentage -- stored as decimal, displayed as percent (e.g. 0.455 -> `45.5%`).
    Percentage { decimal_places: u8 },
    /// Scientific / exponential notation (e.g. `1.23E+4`).
    Scientific { decimal_places: u8 },
    /// Date format with a pattern string (e.g. `MM/DD/YYYY`).
    Date { pattern: String },
    /// Time format with a pattern string (e.g. `HH:MM:SS`).
    Time { pattern: String },
    /// Accounting -- like currency but with symbol alignment (e.g. `$ 1,234.56`).
    Accounting { symbol: String, decimal_places: u8 },
    /// User-defined format string (Excel-compatible pattern).
    Custom(String),
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self::General
    }
}

/// Style of a cell border line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BorderStyle {
    /// No border.
    None,
    /// Thin solid line (1px).
    Thin,
    /// Medium solid line (2px).
    Medium,
    /// Thick solid line (3px).
    Thick,
    /// Dashed line.
    Dashed,
    /// Dotted line.
    Dotted,
    /// Double line.
    Double,
}

/// A single border edge with style and colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Border {
    /// The line style of this border edge.
    pub style: BorderStyle,
    /// CSS hex colour string (e.g. `"#000000"`).
    pub color: String,
}

/// Borders on all four edges of a cell.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CellBorders {
    /// Top edge border.
    pub top: Option<Border>,
    /// Bottom edge border.
    pub bottom: Option<Border>,
    /// Left edge border.
    pub left: Option<Border>,
    /// Right edge border.
    pub right: Option<Border>,
}

/// Text wrapping mode for cell content.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum TextWrap {
    /// Text overflows into adjacent empty cells (default).
    #[default]
    Overflow,
    /// Text wraps within the cell, expanding row height as needed.
    Wrap,
    /// Text is clipped at the cell boundary.
    Clip,
}

/// Visual formatting for a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellFormat {
    /// Whether the cell text is bold.
    pub bold: bool,
    /// Whether the cell text is italic.
    pub italic: bool,
    /// Whether the cell text is underlined.
    pub underline: bool,
    /// Whether the cell text has a strikethrough.
    pub strikethrough: bool,
    /// Font size in points (e.g. 11.0).
    pub font_size: f64,
    /// Font colour as a CSS-style hex string (e.g. `"#000000"`).
    pub font_color: String,
    /// Background / fill colour.
    pub bg_color: Option<String>,
    /// Horizontal alignment.
    pub h_align: HAlign,
    /// Vertical alignment.
    pub v_align: VAlign,
    /// Number format pattern (e.g. `"#,##0.00"`).
    ///
    /// This is the raw Excel-compatible pattern string used for serialization
    /// and file I/O. For structured format operations, convert to/from
    /// [`NumberFormat`] using [`NumberFormat::to_pattern`].
    pub number_format: Option<String>,
    /// Cell border configuration.
    pub borders: CellBorders,
    /// Text wrapping mode.
    pub text_wrap: TextWrap,
}

impl Default for CellFormat {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            font_size: 11.0,
            font_color: "#000000".to_string(),
            bg_color: None,
            h_align: HAlign::default(),
            v_align: VAlign::default(),
            number_format: None,
            borders: CellBorders::default(),
            text_wrap: TextWrap::default(),
        }
    }
}

impl NumberFormat {
    /// Convert this structured format to an Excel-compatible pattern string.
    ///
    /// # Examples
    /// ```
    /// use lattice_core::format::NumberFormat;
    /// assert_eq!(NumberFormat::General.to_pattern(), "General");
    /// assert_eq!(
    ///     NumberFormat::Currency { symbol: "$".into(), decimal_places: 2 }.to_pattern(),
    ///     "$#,##0.00"
    /// );
    /// ```
    pub fn to_pattern(&self) -> String {
        match self {
            Self::General => "General".to_string(),
            Self::Number { decimal_places } => {
                if *decimal_places == 0 {
                    "#,##0".to_string()
                } else {
                    format!("#,##0.{}", "0".repeat(*decimal_places as usize))
                }
            }
            Self::Currency { symbol, decimal_places } => {
                if *decimal_places == 0 {
                    format!("{}#,##0", symbol)
                } else {
                    format!("{}#,##0.{}", symbol, "0".repeat(*decimal_places as usize))
                }
            }
            Self::Percentage { decimal_places } => {
                if *decimal_places == 0 {
                    "0%".to_string()
                } else {
                    format!("0.{}%", "0".repeat(*decimal_places as usize))
                }
            }
            Self::Scientific { decimal_places } => {
                if *decimal_places == 0 {
                    "0E+0".to_string()
                } else {
                    format!("0.{}E+0", "0".repeat(*decimal_places as usize))
                }
            }
            Self::Date { pattern } => pattern.clone(),
            Self::Time { pattern } => pattern.clone(),
            Self::Accounting { symbol, decimal_places } => {
                if *decimal_places == 0 {
                    format!("{} #,##0", symbol)
                } else {
                    format!("{} #,##0.{}", symbol, "0".repeat(*decimal_places as usize))
                }
            }
            Self::Custom(s) => s.clone(),
        }
    }
}

// ── Serial date helpers (Excel convention: days since 1899-12-30) ────

/// Convert an Excel serial date number to (year, month, day).
///
/// Uses the Hinnant civil_from_days algorithm. Handles the Lotus 1-2-3
/// bug where serial 60 is the phantom "Feb 29, 1900".
fn serial_to_ymd(serial: i64) -> (i32, u32, u32) {
    if serial <= 0 {
        return (1900, 1, 1);
    }
    if serial == 60 {
        return (1900, 2, 29); // Lotus 1-2-3 phantom leap day
    }
    let adjusted = if serial > 60 { serial - 1 } else { serial };
    let days_from_1900 = adjusted - 1;
    // Jan 1, 1900 to Jan 1, 1970 = 25567 days
    let unix_days = days_from_1900 - 25567;
    // Hinnant's civil_from_days
    let z = unix_days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// Extract (hours, minutes, seconds) from the fractional part of a serial date.
fn serial_to_hms(serial: f64) -> (u32, u32, u32) {
    let frac = serial.fract().abs();
    let total_seconds = (frac * 86400.0).round() as u64;
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    (hours as u32, minutes as u32, seconds as u32)
}

/// Format a serial date using a date pattern. Tokens: `YYYY`, `YY`, `MM`, `DD`.
fn format_date_pattern(serial: f64, pattern: &str) -> String {
    let (year, month, day) = serial_to_ymd(serial.trunc() as i64);
    let mut r = pattern.to_string();
    r = r.replace("YYYY", &format!("{:04}", year));
    r = r.replace("YY", &format!("{:02}", year % 100));
    r = r.replace("MM", &format!("{:02}", month));
    r = r.replace("DD", &format!("{:02}", day));
    r = r.replace('M', &format!("{}", month));
    r = r.replace('D', &format!("{}", day));
    r
}

/// Format a serial date using a time pattern. Tokens: `HH`, `MM`, `SS`.
fn format_time_pattern(serial: f64, pattern: &str) -> String {
    let (hours, minutes, seconds) = serial_to_hms(serial);
    let mut r = pattern.to_string();
    r = r.replace("HH", &format!("{:02}", hours));
    r = r.replace("SS", &format!("{:02}", seconds));
    r = r.replace("MM", &format!("{:02}", minutes));
    r = r.replace('H', &format!("{}", hours));
    r = r.replace('S', &format!("{}", seconds));
    r
}

/// Format a number with thousand separators and the given decimal places.
fn format_number_with_separators(value: f64, decimal_places: u8) -> String {
    let is_negative = value < 0.0;
    let abs_value = value.abs();
    let factor = 10f64.powi(decimal_places as i32);
    let rounded = (abs_value * factor).round() / factor;
    let int_part = rounded.trunc() as u64;
    let int_str = int_part.to_string();
    // Add thousand separators
    let mut separated = String::new();
    for (i, ch) in int_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            separated.push(',');
        }
        separated.push(ch);
    }
    let separated: String = separated.chars().rev().collect();
    let result = if decimal_places > 0 {
        let frac = ((rounded - rounded.trunc()) * factor).round() as u64;
        format!("{}.{:0>width$}", separated, frac, width = decimal_places as usize)
    } else {
        separated
    };
    if is_negative { format!("-{}", result) } else { result }
}

/// Smart "General" formatting: integers without decimals, floats with up to
/// 10 significant digits and trailing zeros trimmed.
fn format_general(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    let s = format!("{:.10e}", n);
    if let Some((mantissa_str, exp_str)) = s.split_once('e') {
        let exp: i32 = exp_str.parse().unwrap_or(0);
        let mantissa: f64 = mantissa_str.parse().unwrap_or(n);
        let formatted = format!("{:.10}", mantissa * 10f64.powi(exp));
        if formatted.contains('.') {
            formatted.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            formatted
        }
    } else {
        format!("{}", n)
    }
}

/// Format a number in scientific notation with the given decimal places.
fn format_scientific(n: f64, decimal_places: u8) -> String {
    if n == 0.0 {
        return if decimal_places == 0 {
            "0E+0".to_string()
        } else {
            format!("0.{}E+0", "0".repeat(decimal_places as usize))
        };
    }
    let is_negative = n < 0.0;
    let abs_n = n.abs();
    let exp = abs_n.log10().floor() as i32;
    let mantissa = abs_n / 10f64.powi(exp);
    let factor = 10f64.powi(decimal_places as i32);
    let mantissa_rounded = (mantissa * factor).round() / factor;
    let sign = if is_negative { "-" } else { "" };
    let exp_sign = if exp >= 0 { "+" } else { "-" };
    let exp_abs = exp.unsigned_abs();
    if decimal_places == 0 {
        format!("{}{}E{}{}", sign, mantissa_rounded as i64, exp_sign, exp_abs)
    } else {
        format!("{}{:.prec$}E{}{}", sign, mantissa_rounded, exp_sign, exp_abs, prec = decimal_places as usize)
    }
}

/// Format a `CellValue` according to the given `NumberFormat`.
///
/// This is the primary function the frontend and MCP layer should call
/// to obtain the display string for a cell.
///
/// # Examples
/// ```
/// use lattice_core::cell::CellValue;
/// use lattice_core::format::{NumberFormat, format_value};
///
/// assert_eq!(format_value(&CellValue::Number(42.0), &NumberFormat::General), "42");
/// assert_eq!(
///     format_value(&CellValue::Number(1234.5), &NumberFormat::Number { decimal_places: 2 }),
///     "1,234.50"
/// );
/// ```
pub fn format_value(value: &CellValue, format: &NumberFormat) -> String {
    match value {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => s.clone(),
        CellValue::Boolean(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Number(n) => format_number(*n, format),
        CellValue::Checkbox(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        CellValue::Array(rows) => {
            // Show the first element of the array, or "{array}" if empty.
            rows.first()
                .and_then(|row| row.first())
                .map(|v| format_value(v, format))
                .unwrap_or_else(|| "{array}".to_string())
        }
    }
}

/// Format a numeric value according to the given number format.
fn format_number(n: f64, format: &NumberFormat) -> String {
    if n.is_nan() || n.is_infinite() {
        return "#NUM!".to_string();
    }
    match format {
        NumberFormat::General => format_general(n),
        NumberFormat::Number { decimal_places } => format_number_with_separators(n, *decimal_places),
        NumberFormat::Currency { symbol, decimal_places } => {
            let formatted = format_number_with_separators(n.abs(), *decimal_places);
            if n < 0.0 { format!("-{}{}", symbol, formatted) } else { format!("{}{}", symbol, formatted) }
        }
        NumberFormat::Percentage { decimal_places } => {
            let pct = n * 100.0;
            let factor = 10f64.powi(*decimal_places as i32);
            let rounded = (pct * factor).round() / factor;
            if *decimal_places == 0 {
                format!("{}%", rounded as i64)
            } else {
                format!("{:.prec$}%", rounded, prec = *decimal_places as usize)
            }
        }
        NumberFormat::Scientific { decimal_places } => format_scientific(n, *decimal_places),
        NumberFormat::Date { pattern } => format_date_pattern(n, pattern),
        NumberFormat::Time { pattern } => format_time_pattern(n, pattern),
        NumberFormat::Accounting { symbol, decimal_places } => {
            let formatted = format_number_with_separators(n.abs(), *decimal_places);
            if n < 0.0 { format!("-{} {}", symbol, formatted) } else { format!("{} {}", symbol, formatted) }
        }
        NumberFormat::Custom(_) => format_general(n), // Fallback to General
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{CellError, CellValue};

    // ── to_pattern tests ────────────────────────────────────────────

    #[test]
    fn test_to_pattern_general() {
        assert_eq!(NumberFormat::General.to_pattern(), "General");
    }

    #[test]
    fn test_to_pattern_number() {
        assert_eq!(NumberFormat::Number { decimal_places: 2 }.to_pattern(), "#,##0.00");
        assert_eq!(NumberFormat::Number { decimal_places: 0 }.to_pattern(), "#,##0");
    }

    #[test]
    fn test_to_pattern_currency() {
        assert_eq!(
            NumberFormat::Currency { symbol: "$".into(), decimal_places: 2 }.to_pattern(),
            "$#,##0.00"
        );
    }

    #[test]
    fn test_to_pattern_percentage() {
        assert_eq!(NumberFormat::Percentage { decimal_places: 1 }.to_pattern(), "0.0%");
    }

    #[test]
    fn test_to_pattern_scientific() {
        assert_eq!(NumberFormat::Scientific { decimal_places: 2 }.to_pattern(), "0.00E+0");
    }

    #[test]
    fn test_to_pattern_accounting() {
        assert_eq!(
            NumberFormat::Accounting { symbol: "$".into(), decimal_places: 2 }.to_pattern(),
            "$ #,##0.00"
        );
    }

    #[test]
    fn test_default_is_general() {
        assert_eq!(NumberFormat::default(), NumberFormat::General);
    }

    // ── General format ──────────────────────────────────────────────

    #[test]
    fn test_general_integer() {
        assert_eq!(format_value(&CellValue::Number(42.0), &NumberFormat::General), "42");
    }

    #[test]
    fn test_general_float() {
        assert_eq!(format_value(&CellValue::Number(3.14159), &NumberFormat::General), "3.14159");
    }

    #[test]
    fn test_general_zero() {
        assert_eq!(format_value(&CellValue::Number(0.0), &NumberFormat::General), "0");
    }

    #[test]
    fn test_general_negative() {
        assert_eq!(format_value(&CellValue::Number(-7.0), &NumberFormat::General), "-7");
    }

    #[test]
    fn test_general_large_integer() {
        assert_eq!(format_value(&CellValue::Number(1000000.0), &NumberFormat::General), "1000000");
    }

    #[test]
    fn test_general_float_precision() {
        // 0.1 + 0.2 should display cleanly as "0.3"
        assert_eq!(format_value(&CellValue::Number(0.1 + 0.2), &NumberFormat::General), "0.3");
    }

    // ── Number format ───────────────────────────────────────────────

    #[test]
    fn test_number_two_decimals() {
        assert_eq!(
            format_value(&CellValue::Number(1234.5), &NumberFormat::Number { decimal_places: 2 }),
            "1,234.50"
        );
    }

    #[test]
    fn test_number_zero_decimals() {
        assert_eq!(
            format_value(&CellValue::Number(1234.567), &NumberFormat::Number { decimal_places: 0 }),
            "1,235"
        );
    }

    #[test]
    fn test_number_negative() {
        assert_eq!(
            format_value(&CellValue::Number(-9876.54), &NumberFormat::Number { decimal_places: 2 }),
            "-9,876.54"
        );
    }

    #[test]
    fn test_number_small() {
        assert_eq!(
            format_value(&CellValue::Number(0.5), &NumberFormat::Number { decimal_places: 2 }),
            "0.50"
        );
    }

    // ── Currency format ─────────────────────────────────────────────

    #[test]
    fn test_currency_positive() {
        let fmt = NumberFormat::Currency { symbol: "$".into(), decimal_places: 2 };
        assert_eq!(format_value(&CellValue::Number(1234.5), &fmt), "$1,234.50");
    }

    #[test]
    fn test_currency_negative() {
        let fmt = NumberFormat::Currency { symbol: "$".into(), decimal_places: 2 };
        assert_eq!(format_value(&CellValue::Number(-50.0), &fmt), "-$50.00");
    }

    #[test]
    fn test_currency_euro() {
        let fmt = NumberFormat::Currency { symbol: "\u{20ac}".into(), decimal_places: 2 };
        assert_eq!(format_value(&CellValue::Number(99.9), &fmt), "\u{20ac}99.90");
    }

    // ── Percentage format ───────────────────────────────────────────

    #[test]
    fn test_percentage_one_decimal() {
        assert_eq!(
            format_value(&CellValue::Number(0.455), &NumberFormat::Percentage { decimal_places: 1 }),
            "45.5%"
        );
    }

    #[test]
    fn test_percentage_zero_decimals() {
        assert_eq!(
            format_value(&CellValue::Number(0.867), &NumberFormat::Percentage { decimal_places: 0 }),
            "87%"
        );
    }

    #[test]
    fn test_percentage_negative() {
        assert_eq!(
            format_value(&CellValue::Number(-0.05), &NumberFormat::Percentage { decimal_places: 1 }),
            "-5.0%"
        );
    }

    // ── Scientific format ───────────────────────────────────────────

    #[test]
    fn test_scientific_basic() {
        assert_eq!(
            format_value(&CellValue::Number(12345.0), &NumberFormat::Scientific { decimal_places: 2 }),
            "1.23E+4"
        );
    }

    #[test]
    fn test_scientific_small() {
        assert_eq!(
            format_value(&CellValue::Number(0.00456), &NumberFormat::Scientific { decimal_places: 2 }),
            "4.56E-3"
        );
    }

    #[test]
    fn test_scientific_negative() {
        assert_eq!(
            format_value(&CellValue::Number(-12345.0), &NumberFormat::Scientific { decimal_places: 2 }),
            "-1.23E+4"
        );
    }

    #[test]
    fn test_scientific_zero() {
        assert_eq!(
            format_value(&CellValue::Number(0.0), &NumberFormat::Scientific { decimal_places: 2 }),
            "0.00E+0"
        );
    }

    // ── Date format ─────────────────────────────────────────────────

    #[test]
    fn test_date_serial_1() {
        let fmt = NumberFormat::Date { pattern: "MM/DD/YYYY".into() };
        assert_eq!(format_value(&CellValue::Number(1.0), &fmt), "01/01/1900");
    }

    #[test]
    fn test_date_serial_59() {
        let fmt = NumberFormat::Date { pattern: "MM/DD/YYYY".into() };
        assert_eq!(format_value(&CellValue::Number(59.0), &fmt), "02/28/1900");
    }

    #[test]
    fn test_date_serial_61() {
        // Serial 61 = Mar 1, 1900 (serial 60 is phantom Feb 29)
        let fmt = NumberFormat::Date { pattern: "MM/DD/YYYY".into() };
        assert_eq!(format_value(&CellValue::Number(61.0), &fmt), "03/01/1900");
    }

    #[test]
    fn test_date_serial_45000() {
        // Serial 45000 = March 15, 2023 (verified by counting from serial 44927 = Jan 1, 2023)
        let fmt = NumberFormat::Date { pattern: "MM/DD/YYYY".into() };
        assert_eq!(format_value(&CellValue::Number(45000.0), &fmt), "03/15/2023");
    }

    #[test]
    fn test_date_iso_format() {
        let fmt = NumberFormat::Date { pattern: "YYYY-MM-DD".into() };
        assert_eq!(format_value(&CellValue::Number(45000.0), &fmt), "2023-03-15");
    }

    #[test]
    fn test_date_jan_1_2000() {
        let fmt = NumberFormat::Date { pattern: "YYYY-MM-DD".into() };
        assert_eq!(format_value(&CellValue::Number(36526.0), &fmt), "2000-01-01");
    }

    #[test]
    fn test_date_jan_1_2024() {
        let fmt = NumberFormat::Date { pattern: "YYYY-MM-DD".into() };
        assert_eq!(format_value(&CellValue::Number(45292.0), &fmt), "2024-01-01");
    }

    // ── Time format ─────────────────────────────────────────────────

    #[test]
    fn test_time_noon() {
        let fmt = NumberFormat::Time { pattern: "HH:MM:SS".into() };
        assert_eq!(format_value(&CellValue::Number(0.5), &fmt), "12:00:00");
    }

    #[test]
    fn test_time_quarter_day() {
        let fmt = NumberFormat::Time { pattern: "HH:MM:SS".into() };
        assert_eq!(format_value(&CellValue::Number(0.25), &fmt), "06:00:00");
    }

    // ── Accounting format ───────────────────────────────────────────

    #[test]
    fn test_accounting_positive() {
        let fmt = NumberFormat::Accounting { symbol: "$".into(), decimal_places: 2 };
        assert_eq!(format_value(&CellValue::Number(1234.5), &fmt), "$ 1,234.50");
    }

    #[test]
    fn test_accounting_negative() {
        let fmt = NumberFormat::Accounting { symbol: "$".into(), decimal_places: 2 };
        assert_eq!(format_value(&CellValue::Number(-50.0), &fmt), "-$ 50.00");
    }

    // ── Non-numeric values ──────────────────────────────────────────

    #[test]
    fn test_format_empty() {
        assert_eq!(format_value(&CellValue::Empty, &NumberFormat::Number { decimal_places: 2 }), "");
    }

    #[test]
    fn test_format_text() {
        assert_eq!(
            format_value(&CellValue::Text("hello".into()), &NumberFormat::Number { decimal_places: 2 }),
            "hello"
        );
    }

    #[test]
    fn test_format_boolean() {
        assert_eq!(format_value(&CellValue::Boolean(true), &NumberFormat::General), "TRUE");
        assert_eq!(format_value(&CellValue::Boolean(false), &NumberFormat::General), "FALSE");
    }

    #[test]
    fn test_format_error() {
        assert_eq!(format_value(&CellValue::Error(CellError::DivZero), &NumberFormat::General), "#DIV/0!");
    }

    #[test]
    fn test_format_nan() {
        assert_eq!(format_value(&CellValue::Number(f64::NAN), &NumberFormat::General), "#NUM!");
    }

    #[test]
    fn test_format_infinity() {
        assert_eq!(format_value(&CellValue::Number(f64::INFINITY), &NumberFormat::General), "#NUM!");
    }

    #[test]
    fn test_custom_format_fallback() {
        // Custom patterns fall back to General formatting for now
        assert_eq!(format_value(&CellValue::Number(42.0), &NumberFormat::Custom("#,##0.00".into())), "42");
    }

    // ── Borders ────────────────────────────────────────────────────────

    #[test]
    fn test_cell_borders_default() {
        let borders = CellBorders::default();
        assert!(borders.top.is_none());
        assert!(borders.bottom.is_none());
        assert!(borders.left.is_none());
        assert!(borders.right.is_none());
    }

    #[test]
    fn test_cell_borders_with_edges() {
        let borders = CellBorders {
            top: Some(Border { style: BorderStyle::Thin, color: "#000000".into() }),
            bottom: Some(Border { style: BorderStyle::Thick, color: "#FF0000".into() }),
            left: None,
            right: Some(Border { style: BorderStyle::Dashed, color: "#00FF00".into() }),
        };
        assert_eq!(borders.top.as_ref().unwrap().style, BorderStyle::Thin);
        assert_eq!(borders.bottom.as_ref().unwrap().color, "#FF0000");
        assert!(borders.left.is_none());
        assert_eq!(borders.right.as_ref().unwrap().style, BorderStyle::Dashed);
    }

    #[test]
    fn test_border_style_variants() {
        // Ensure all variants are distinct
        let styles = vec![
            BorderStyle::None, BorderStyle::Thin, BorderStyle::Medium,
            BorderStyle::Thick, BorderStyle::Dashed, BorderStyle::Dotted,
            BorderStyle::Double,
        ];
        for (i, a) in styles.iter().enumerate() {
            for (j, b) in styles.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // ── Text Wrap ──────────────────────────────────────────────────────

    #[test]
    fn test_text_wrap_default() {
        assert_eq!(TextWrap::default(), TextWrap::Overflow);
    }

    #[test]
    fn test_text_wrap_variants() {
        assert_ne!(TextWrap::Overflow, TextWrap::Wrap);
        assert_ne!(TextWrap::Wrap, TextWrap::Clip);
        assert_ne!(TextWrap::Overflow, TextWrap::Clip);
    }

    // ── Underline & Strikethrough ──────────────────────────────────────

    #[test]
    fn test_cell_format_default_underline_strikethrough() {
        let fmt = CellFormat::default();
        assert!(!fmt.underline);
        assert!(!fmt.strikethrough);
    }

    #[test]
    fn test_cell_format_with_underline_strikethrough() {
        let fmt = CellFormat {
            underline: true,
            strikethrough: true,
            ..CellFormat::default()
        };
        assert!(fmt.underline);
        assert!(fmt.strikethrough);
    }

    // ── CellFormat default includes new fields ─────────────────────────

    #[test]
    fn test_cell_format_default_has_borders_and_wrap() {
        let fmt = CellFormat::default();
        assert_eq!(fmt.borders, CellBorders::default());
        assert_eq!(fmt.text_wrap, TextWrap::Overflow);
    }

    #[test]
    fn test_cell_format_with_borders() {
        let fmt = CellFormat {
            borders: CellBorders {
                top: Some(Border { style: BorderStyle::Medium, color: "#333333".into() }),
                ..CellBorders::default()
            },
            ..CellFormat::default()
        };
        assert!(fmt.borders.top.is_some());
        assert!(fmt.borders.bottom.is_none());
    }

    #[test]
    fn test_cell_format_with_text_wrap() {
        let fmt = CellFormat {
            text_wrap: TextWrap::Wrap,
            ..CellFormat::default()
        };
        assert_eq!(fmt.text_wrap, TextWrap::Wrap);
    }

    // -- Checkbox formatting ---------------------------------------------------

    #[test]
    fn test_format_checkbox_true() {
        assert_eq!(
            format_value(&CellValue::Checkbox(true), &NumberFormat::General),
            "TRUE"
        );
    }

    #[test]
    fn test_format_checkbox_false() {
        assert_eq!(
            format_value(&CellValue::Checkbox(false), &NumberFormat::General),
            "FALSE"
        );
    }

    // -- Array formatting ------------------------------------------------------

    #[test]
    fn test_format_array_shows_first_element() {
        let arr = CellValue::Array(vec![
            vec![CellValue::Number(42.0), CellValue::Number(99.0)],
        ]);
        assert_eq!(format_value(&arr, &NumberFormat::General), "42");
    }

    #[test]
    fn test_format_array_empty_shows_placeholder() {
        let arr = CellValue::Array(vec![]);
        assert_eq!(format_value(&arr, &NumberFormat::General), "{array}");
    }

    #[test]
    fn test_format_array_empty_row_shows_placeholder() {
        let arr = CellValue::Array(vec![vec![]]);
        assert_eq!(format_value(&arr, &NumberFormat::General), "{array}");
    }

    #[test]
    fn test_format_array_with_text_first() {
        let arr = CellValue::Array(vec![
            vec![CellValue::Text("hello".into())],
        ]);
        assert_eq!(format_value(&arr, &NumberFormat::General), "hello");
    }

    #[test]
    fn test_format_array_respects_number_format() {
        let arr = CellValue::Array(vec![
            vec![CellValue::Number(1234.5)],
        ]);
        assert_eq!(
            format_value(&arr, &NumberFormat::Currency { symbol: "$".into(), decimal_places: 2 }),
            "$1,234.50"
        );
    }
}
