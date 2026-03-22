use serde::{Deserialize, Serialize};

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

/// Visual formatting for a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellFormat {
    /// Whether the cell text is bold.
    pub bold: bool,
    /// Whether the cell text is italic.
    pub italic: bool,
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
}

impl Default for CellFormat {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            font_size: 11.0,
            font_color: "#000000".to_string(),
            bg_color: None,
            h_align: HAlign::default(),
            v_align: VAlign::default(),
            number_format: None,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
