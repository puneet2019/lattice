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
    /// A checkbox value (checked = true, unchecked = false).
    ///
    /// Behaves like a boolean in formulas but renders as a toggleable
    /// checkbox in the UI. Use [`CellValue::is_checkbox`] to distinguish
    /// from plain booleans.
    Checkbox(bool),
    /// A 2-D array of values returned by an array formula (e.g. TRANSPOSE,
    /// SEQUENCE, FILTER, SORT, UNIQUE).
    ///
    /// The outer `Vec` represents rows and the inner `Vec` represents
    /// columns. When displayed in a single cell, the first element is
    /// shown (or `"{array}"`).
    Array(Vec<Vec<CellValue>>),
}

impl CellValue {
    /// Returns `true` if this value is a [`CellValue::Checkbox`].
    pub fn is_checkbox(&self) -> bool {
        matches!(self, Self::Checkbox(_))
    }

    /// Returns `true` if this value is an [`CellValue::Array`].
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
}

/// Configuration for a dropdown chip attached to a cell.
///
/// When a cell has a `DropdownConfig`, the frontend renders it as a dropdown
/// chip. This integrates with [`ValidationRule::List`] — the options list
/// here mirrors the validation list, but the dropdown also controls UI
/// presentation (e.g. whether custom values are allowed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropdownConfig {
    /// The allowed dropdown options.
    pub options: Vec<String>,
    /// Whether the user may type a value that is not in `options`.
    pub allow_custom: bool,
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
    /// Whether this cell is part of an array formula.
    pub is_array_formula: bool,
    /// If this cell is the anchor of an array formula, the spill range
    /// `(start_row, start_col, end_row, end_col)` — all 0-based, inclusive.
    pub array_formula_range: Option<(u32, u32, u32, u32)>,
    /// Optional dropdown chip configuration for this cell.
    pub dropdown: Option<DropdownConfig>,
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
            is_array_formula: false,
            array_formula_range: None,
            dropdown: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- CellValue::Checkbox ---------------------------------------------------

    #[test]
    fn test_checkbox_is_checkbox() {
        assert!(CellValue::Checkbox(true).is_checkbox());
        assert!(CellValue::Checkbox(false).is_checkbox());
        assert!(!CellValue::Boolean(true).is_checkbox());
        assert!(!CellValue::Number(1.0).is_checkbox());
        assert!(!CellValue::Empty.is_checkbox());
    }

    #[test]
    fn test_checkbox_equality() {
        assert_eq!(CellValue::Checkbox(true), CellValue::Checkbox(true));
        assert_ne!(CellValue::Checkbox(true), CellValue::Checkbox(false));
        // Checkbox(true) is distinct from Boolean(true)
        assert_ne!(CellValue::Checkbox(true), CellValue::Boolean(true));
    }

    #[test]
    fn test_checkbox_clone() {
        let v = CellValue::Checkbox(true);
        let c = v.clone();
        assert_eq!(v, c);
    }

    // -- CellValue::Array ------------------------------------------------------

    #[test]
    fn test_array_is_array() {
        let arr = CellValue::Array(vec![vec![CellValue::Number(1.0)]]);
        assert!(arr.is_array());
        assert!(!CellValue::Number(1.0).is_array());
        assert!(!CellValue::Empty.is_array());
    }

    #[test]
    fn test_array_empty() {
        let arr = CellValue::Array(vec![]);
        assert!(arr.is_array());
    }

    #[test]
    fn test_array_multi_row() {
        let arr = CellValue::Array(vec![
            vec![CellValue::Number(1.0), CellValue::Number(2.0)],
            vec![CellValue::Number(3.0), CellValue::Number(4.0)],
        ]);
        if let CellValue::Array(rows) = &arr {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 2);
            assert_eq!(rows[1][1], CellValue::Number(4.0));
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_array_nested_types() {
        // Arrays can hold mixed types
        let arr = CellValue::Array(vec![vec![
            CellValue::Number(1.0),
            CellValue::Text("hello".into()),
            CellValue::Boolean(true),
            CellValue::Checkbox(false),
        ]]);
        assert!(arr.is_array());
    }

    // -- DropdownConfig --------------------------------------------------------

    #[test]
    fn test_dropdown_config_creation() {
        let dd = DropdownConfig {
            options: vec!["Yes".into(), "No".into(), "Maybe".into()],
            allow_custom: false,
        };
        assert_eq!(dd.options.len(), 3);
        assert!(!dd.allow_custom);
    }

    #[test]
    fn test_dropdown_config_allow_custom() {
        let dd = DropdownConfig {
            options: vec!["A".into()],
            allow_custom: true,
        };
        assert!(dd.allow_custom);
    }

    // -- Cell new fields -------------------------------------------------------

    #[test]
    fn test_cell_default_new_fields() {
        let cell = Cell::default();
        assert!(!cell.is_array_formula);
        assert!(cell.array_formula_range.is_none());
        assert!(cell.dropdown.is_none());
    }

    #[test]
    fn test_cell_with_array_formula() {
        let cell = Cell {
            is_array_formula: true,
            array_formula_range: Some((0, 0, 2, 2)),
            ..Default::default()
        };
        assert!(cell.is_array_formula);
        assert_eq!(cell.array_formula_range, Some((0, 0, 2, 2)));
    }

    #[test]
    fn test_cell_with_dropdown() {
        let cell = Cell {
            dropdown: Some(DropdownConfig {
                options: vec!["Red".into(), "Blue".into()],
                allow_custom: false,
            }),
            ..Default::default()
        };
        assert!(cell.dropdown.is_some());
        assert_eq!(cell.dropdown.unwrap().options, vec!["Red", "Blue"]);
    }

    // -- Serialization roundtrip -----------------------------------------------

    #[test]
    fn test_checkbox_serde_roundtrip() {
        let v = CellValue::Checkbox(true);
        let json = serde_json::to_string(&v).unwrap();
        let parsed: CellValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn test_array_serde_roundtrip() {
        let v = CellValue::Array(vec![
            vec![CellValue::Number(1.0), CellValue::Text("a".into())],
        ]);
        let json = serde_json::to_string(&v).unwrap();
        let parsed: CellValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn test_dropdown_config_serde_roundtrip() {
        let dd = DropdownConfig {
            options: vec!["X".into(), "Y".into()],
            allow_custom: true,
        };
        let json = serde_json::to_string(&dd).unwrap();
        let parsed: DropdownConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(dd, parsed);
    }

    #[test]
    fn test_cell_with_new_fields_serde_roundtrip() {
        let cell = Cell {
            value: CellValue::Checkbox(false),
            is_array_formula: true,
            array_formula_range: Some((1, 2, 3, 4)),
            dropdown: Some(DropdownConfig {
                options: vec!["A".into()],
                allow_custom: false,
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&cell).unwrap();
        let parsed: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, parsed);
    }
}
