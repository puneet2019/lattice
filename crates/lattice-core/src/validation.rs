//! Data validation rules for spreadsheet cells.
//!
//! Supports validating cell values against rules such as dropdown lists,
//! number ranges, date ranges, text length constraints, and custom
//! formulas (stored as strings for future evaluation).

use std::collections::HashMap;

use crate::cell::CellValue;

/// The type of validation to apply to a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationType {
    /// A dropdown list of allowed string values.
    List(Vec<String>),
    /// A numeric range with optional min and max bounds.
    NumberRange { min: Option<f64>, max: Option<f64> },
    /// A date range with optional min and max bounds (ISO 8601 strings).
    DateRange {
        min: Option<String>,
        max: Option<String>,
    },
    /// A text length constraint with optional min and max character counts.
    TextLength {
        min: Option<usize>,
        max: Option<usize>,
    },
    /// A custom formula string that must evaluate to true. The formula
    /// text is stored here; actual evaluation is delegated to the formula
    /// engine at a higher layer.
    Custom(String),
}

/// A validation rule attached to a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationRule {
    /// The type of validation.
    pub validation_type: ValidationType,
    /// Whether blank (empty) values are allowed regardless of the rule.
    pub allow_blank: bool,
    /// Optional error message shown when validation fails.
    pub error_message: Option<String>,
}

/// A store of validation rules keyed by `(sheet_name, row, col)`.
#[derive(Debug, Clone, Default)]
pub struct ValidationStore {
    rules: HashMap<(String, u32, u32), ValidationRule>,
}

impl ValidationStore {
    /// Create an empty validation store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a validation rule on a cell. Replaces any existing rule.
    pub fn set_rule(&mut self, sheet: &str, row: u32, col: u32, rule: ValidationRule) {
        self.rules.insert((sheet.to_string(), row, col), rule);
    }

    /// Get the validation rule for a cell, if any.
    pub fn get_rule(&self, sheet: &str, row: u32, col: u32) -> Option<&ValidationRule> {
        self.rules.get(&(sheet.to_string(), row, col))
    }

    /// Remove the validation rule from a cell.
    pub fn remove_rule(&mut self, sheet: &str, row: u32, col: u32) {
        self.rules.remove(&(sheet.to_string(), row, col));
    }

    /// List all validation rules for a given sheet, returning
    /// `((row, col), &ValidationRule)` pairs sorted by position.
    pub fn list_rules(&self, sheet: &str) -> Vec<((u32, u32), &ValidationRule)> {
        let mut result: Vec<((u32, u32), &ValidationRule)> = self
            .rules
            .iter()
            .filter(|((s, _, _), _)| s == sheet)
            .map(|((_, r, c), rule)| ((*r, *c), rule))
            .collect();
        result.sort_by_key(|((r, c), _)| (*r, *c));
        result
    }

    /// Return the total number of validation rules across all sheets.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Return `true` if no validation rules exist.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Check if a cell value passes a validation rule.
///
/// Returns `true` if the value is valid, `false` if it violates the rule.
/// `Custom` validation always returns `true` here since formula evaluation
/// requires the formula engine (handled at a higher layer).
pub fn validate(value: &CellValue, rule: &ValidationRule) -> bool {
    // Blank values pass if allow_blank is set.
    if matches!(value, CellValue::Empty) && rule.allow_blank {
        return true;
    }

    match &rule.validation_type {
        ValidationType::List(options) => match value {
            CellValue::Text(t) => options.iter().any(|o| o.eq_ignore_ascii_case(t)),
            CellValue::Number(n) => {
                let s = n.to_string();
                options.contains(&s)
            }
            CellValue::Empty => false,
            _ => false,
        },

        ValidationType::NumberRange { min, max } => {
            let n = match value {
                CellValue::Number(n) => *n,
                CellValue::Boolean(b) => {
                    if *b {
                        1.0
                    } else {
                        0.0
                    }
                }
                _ => return false,
            };
            if let Some(lo) = min
                && n < *lo
            {
                return false;
            }
            if let Some(hi) = max
                && n > *hi
            {
                return false;
            }
            true
        }

        ValidationType::DateRange { min, max } => {
            let date_str = match value {
                CellValue::Date(d) => d.as_str(),
                CellValue::Text(t) => t.as_str(),
                _ => return false,
            };
            if let Some(lo) = min
                && date_str < lo.as_str()
            {
                return false;
            }
            if let Some(hi) = max
                && date_str > hi.as_str()
            {
                return false;
            }
            true
        }

        ValidationType::TextLength { min, max } => {
            let len = match value {
                CellValue::Text(t) => t.len(),
                CellValue::Empty => 0,
                _ => {
                    // Convert to string for length check.
                    cell_value_display_len(value)
                }
            };
            if let Some(lo) = min
                && len < *lo
            {
                return false;
            }
            if let Some(hi) = max
                && len > *hi
            {
                return false;
            }
            true
        }

        ValidationType::Custom(_) => {
            // Custom formulas require the formula engine; always pass here.
            true
        }
    }
}

/// Return the display string length of a CellValue.
fn cell_value_display_len(value: &CellValue) -> usize {
    match value {
        CellValue::Text(s) => s.len(),
        CellValue::Number(n) => n.to_string().len(),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => {
            if *b {
                4
            } else {
                5
            }
        } // TRUE / FALSE
        CellValue::Empty => 0,
        CellValue::Error(e) => e.to_string().len(),
        CellValue::Date(s) => s.len(),
        CellValue::Array(_) => 7,      // "{array}"
        CellValue::Lambda { .. } => 8, // "{lambda}"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number_rule(min: Option<f64>, max: Option<f64>) -> ValidationRule {
        ValidationRule {
            validation_type: ValidationType::NumberRange { min, max },
            allow_blank: false,
            error_message: None,
        }
    }

    fn list_rule(items: &[&str]) -> ValidationRule {
        ValidationRule {
            validation_type: ValidationType::List(items.iter().map(|s| s.to_string()).collect()),
            allow_blank: false,
            error_message: None,
        }
    }

    #[test]
    fn test_number_range_valid() {
        let rule = number_rule(Some(0.0), Some(100.0));
        assert!(validate(&CellValue::Number(50.0), &rule));
        assert!(validate(&CellValue::Number(0.0), &rule));
        assert!(validate(&CellValue::Number(100.0), &rule));
    }

    #[test]
    fn test_number_range_invalid() {
        let rule = number_rule(Some(0.0), Some(100.0));
        assert!(!validate(&CellValue::Number(-1.0), &rule));
        assert!(!validate(&CellValue::Number(101.0), &rule));
    }

    #[test]
    fn test_number_range_open_ended() {
        let min_only = number_rule(Some(10.0), None);
        assert!(validate(&CellValue::Number(100.0), &min_only));
        assert!(!validate(&CellValue::Number(5.0), &min_only));

        let max_only = number_rule(None, Some(50.0));
        assert!(validate(&CellValue::Number(-100.0), &max_only));
        assert!(!validate(&CellValue::Number(51.0), &max_only));
    }

    #[test]
    fn test_number_range_rejects_text() {
        let rule = number_rule(Some(0.0), Some(100.0));
        assert!(!validate(&CellValue::Text("hello".into()), &rule));
    }

    #[test]
    fn test_list_validation() {
        let rule = list_rule(&["Yes", "No", "Maybe"]);
        assert!(validate(&CellValue::Text("Yes".into()), &rule));
        assert!(validate(&CellValue::Text("yes".into()), &rule));
        assert!(!validate(&CellValue::Text("Nope".into()), &rule));
    }

    #[test]
    fn test_text_length_validation() {
        let rule = ValidationRule {
            validation_type: ValidationType::TextLength {
                min: Some(3),
                max: Some(10),
            },
            allow_blank: false,
            error_message: None,
        };
        assert!(validate(&CellValue::Text("hello".into()), &rule));
        assert!(!validate(&CellValue::Text("hi".into()), &rule));
        assert!(!validate(
            &CellValue::Text("this is way too long".into()),
            &rule
        ));
    }

    #[test]
    fn test_date_range_validation() {
        let rule = ValidationRule {
            validation_type: ValidationType::DateRange {
                min: Some("2024-01-01".into()),
                max: Some("2024-12-31".into()),
            },
            allow_blank: false,
            error_message: None,
        };
        assert!(validate(&CellValue::Date("2024-06-15".into()), &rule));
        assert!(!validate(&CellValue::Date("2023-12-31".into()), &rule));
        assert!(!validate(&CellValue::Date("2025-01-01".into()), &rule));
    }

    #[test]
    fn test_allow_blank() {
        let mut rule = number_rule(Some(0.0), Some(100.0));
        assert!(!validate(&CellValue::Empty, &rule));
        rule.allow_blank = true;
        assert!(validate(&CellValue::Empty, &rule));
    }

    #[test]
    fn test_custom_always_passes() {
        let rule = ValidationRule {
            validation_type: ValidationType::Custom("=A1>0".into()),
            allow_blank: false,
            error_message: None,
        };
        assert!(validate(&CellValue::Number(42.0), &rule));
        assert!(validate(&CellValue::Text("anything".into()), &rule));
    }

    #[test]
    fn test_store_set_get_remove() {
        let mut store = ValidationStore::new();
        let rule = number_rule(Some(0.0), Some(100.0));
        store.set_rule("Sheet1", 0, 0, rule.clone());
        assert_eq!(store.get_rule("Sheet1", 0, 0), Some(&rule));
        assert_eq!(store.len(), 1);

        store.remove_rule("Sheet1", 0, 0);
        assert!(store.get_rule("Sheet1", 0, 0).is_none());
        assert!(store.is_empty());
    }

    #[test]
    fn test_store_list_rules() {
        let mut store = ValidationStore::new();
        store.set_rule("Sheet1", 2, 0, number_rule(Some(0.0), None));
        store.set_rule("Sheet1", 0, 1, list_rule(&["A", "B"]));
        store.set_rule("Sheet2", 0, 0, number_rule(None, Some(50.0)));

        let rules = store.list_rules("Sheet1");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].0, (0, 1)); // sorted by position
        assert_eq!(rules[1].0, (2, 0));
    }

    #[test]
    fn test_store_replace_existing_rule() {
        let mut store = ValidationStore::new();
        store.set_rule("S1", 0, 0, number_rule(Some(0.0), Some(10.0)));
        store.set_rule("S1", 0, 0, number_rule(Some(0.0), Some(100.0)));
        assert_eq!(store.len(), 1);
        let rule = store.get_rule("S1", 0, 0).unwrap();
        match &rule.validation_type {
            ValidationType::NumberRange { max, .. } => assert_eq!(*max, Some(100.0)),
            _ => panic!("unexpected type"),
        }
    }
}
