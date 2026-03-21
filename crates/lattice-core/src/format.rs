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
