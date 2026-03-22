use serde::{Deserialize, Serialize};

use crate::format::CellFormat;

/// Errors that can appear as a cell value (like `#REF!`, `#VALUE!`, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CellError {
    /// `#REF!` — invalid cell reference.
    Ref,
    /// `#VALUE!` — wrong value type.
    Value,
    /// `#DIV/0!` — division by zero.
    DivZero,
    /// `#NAME?` — unrecognised name.
    Name,
    /// `#N/A` — value not available.
    NA,
    /// `#NULL!` — incorrect range operator.
    Null,
    /// `#NUM!` — invalid numeric value.
    Num,
}

impl std::fmt::Display for CellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ref => write!(f, "#REF!"),
            Self::Value => write!(f, "#VALUE!"),
            Self::DivZero => write!(f, "#DIV/0!"),
            Self::Name => write!(f, "#NAME?"),
            Self::NA => write!(f, "#N/A"),
            Self::Null => write!(f, "#NULL!"),
            Self::Num => write!(f, "#NUM!"),
        }
    }
}

/// The value stored in a cell.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum CellValue {
    /// No value.
    #[default]
    Empty,
    /// Plain text.
    Text(String),
    /// A floating-point number.
    Number(f64),
    /// A boolean.
    Boolean(bool),
    /// An error value such as `#REF!`.
    Error(CellError),
    /// A date/time stored as an ISO 8601 string (we avoid chrono for now).
    Date(String),
}

/// A single cell in the spreadsheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// The cell's computed or entered value.
    pub value: CellValue,
    /// Optional formula text (without the leading `=`).
    pub formula: Option<String>,
    /// Formatting metadata.
    pub format: CellFormat,
    /// Shared style reference id.
    pub style_id: u32,
    /// Optional comment / note.
    pub comment: Option<String>,
    /// Optional hyperlink URL (e.g. `"https://example.com"`).
    pub hyperlink: Option<String>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            value: CellValue::Empty,
            formula: None,
            format: CellFormat::default(),
            style_id: 0,
            comment: None,
            hyperlink: None,
        }
    }
}
