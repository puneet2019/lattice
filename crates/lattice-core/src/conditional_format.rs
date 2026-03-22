//! Conditional formatting rules and evaluation.
//!
//! Allows applying visual styles to cells based on their values.
//! Rules are grouped by range and evaluated in priority order.
//! Supports cell value comparisons, text matching, blank/error checks,
//! color scales, data bars, icon sets, and custom formulas.

use crate::cell::CellValue;

/// Comparison operators for cell-value conditional rules.
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    NotEqual,
    /// Between value1 and value2 (inclusive).
    Between,
    /// Not between value1 and value2 (inclusive).
    NotBetween,
}

/// The type of conditional formatting rule.
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionalRuleType {
    /// Compare the cell's numeric value against one or two thresholds.
    CellValue {
        operator: ComparisonOperator,
        value1: f64,
        value2: Option<f64>,
    },
    /// Cell text contains the given substring (case-insensitive).
    TextContains(String),
    /// Cell text starts with the given prefix (case-insensitive).
    TextStartsWith(String),
    /// Cell text ends with the given suffix (case-insensitive).
    TextEndsWith(String),
    /// Cell is blank (empty).
    IsBlank,
    /// Cell is not blank.
    IsNotBlank,
    /// Cell contains an error value.
    IsError,
    /// Highlight duplicate values in the range (evaluation requires range
    /// context; single-cell evaluate always returns false).
    DuplicateValues,
    /// Highlight unique values in the range (evaluation requires range
    /// context; single-cell evaluate always returns false).
    UniqueValues,
    /// Two- or three-color gradient scale. Colors are CSS hex strings.
    ColorScale {
        min_color: String,
        max_color: String,
        mid_color: Option<String>,
    },
    /// Data bar visualisation.
    DataBar {
        color: String,
        max_length_percent: u8,
    },
    /// Icon set with threshold values. `icons` and `thresholds` define
    /// the mapping (thresholds.len() == icons.len() - 1).
    IconSet {
        icons: Vec<String>,
        thresholds: Vec<f64>,
    },
    /// A custom formula string. Stored as text; actual evaluation is
    /// delegated to the formula engine at a higher layer.
    Formula(String),
}

/// Style overrides applied when a conditional rule matches.
/// Each field is `Option`; unset fields inherit from the cell's base format.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConditionalStyle {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font_color: Option<String>,
    pub bg_color: Option<String>,
}

/// A single conditional formatting rule with its style and control flags.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalRule {
    pub rule_type: ConditionalRuleType,
    pub style: ConditionalStyle,
    /// Lower number = higher priority.
    pub priority: u32,
    pub stop_if_true: bool,
}

/// A range of cells with associated conditional formatting rules.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalFormatRange {
    pub sheet: String,
    pub start_row: u32,
    pub start_col: u32,
    pub end_row: u32,
    pub end_col: u32,
    pub rules: Vec<ConditionalRule>,
}

/// Storage for all conditional formatting ranges in a workbook.
#[derive(Debug, Clone, Default)]
pub struct ConditionalFormatStore {
    ranges: Vec<ConditionalFormatRange>,
}

impl ConditionalFormatStore {
    /// Create an empty conditional format store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a conditional formatting rule to the specified range.
    ///
    /// If a `ConditionalFormatRange` already exists for the exact same
    /// sheet and coordinates, the rule is appended to it. Otherwise a
    /// new range entry is created.
    pub fn add_rule(
        &mut self,
        sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
        rule: ConditionalRule,
    ) {
        // Try to find an existing range with the same bounds.
        if let Some(range) = self.ranges.iter_mut().find(|r| {
            r.sheet == sheet
                && r.start_row == start_row
                && r.start_col == start_col
                && r.end_row == end_row
                && r.end_col == end_col
        }) {
            range.rules.push(rule);
        } else {
            self.ranges.push(ConditionalFormatRange {
                sheet: sheet.to_string(),
                start_row,
                start_col,
                end_row,
                end_col,
                rules: vec![rule],
            });
        }
    }

    /// Remove a rule by index from the range matching the given coordinates.
    ///
    /// Returns `true` if the rule was removed, `false` if the range or
    /// index was not found. If the range has no remaining rules after
    /// removal, the range entry itself is removed.
    pub fn remove_rule(
        &mut self,
        sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
        rule_index: usize,
    ) -> bool {
        if let Some(pos) = self.ranges.iter().position(|r| {
            r.sheet == sheet
                && r.start_row == start_row
                && r.start_col == start_col
                && r.end_row == end_row
                && r.end_col == end_col
        }) {
            let range = &mut self.ranges[pos];
            if rule_index < range.rules.len() {
                range.rules.remove(rule_index);
                if range.rules.is_empty() {
                    self.ranges.remove(pos);
                }
                return true;
            }
        }
        false
    }

    /// Get all rules that apply to a specific cell, sorted by priority
    /// (lower priority number first = higher priority).
    pub fn get_rules(&self, sheet: &str, row: u32, col: u32) -> Vec<&ConditionalRule> {
        let mut result: Vec<&ConditionalRule> = self
            .ranges
            .iter()
            .filter(|r| {
                r.sheet == sheet
                    && row >= r.start_row
                    && row <= r.end_row
                    && col >= r.start_col
                    && col <= r.end_col
            })
            .flat_map(|r| r.rules.iter())
            .collect();
        result.sort_by_key(|r| r.priority);
        result
    }

    /// Evaluate whether a single cell value matches a conditional rule.
    /// Range-context rules (`DuplicateValues`, `UniqueValues`) and `Formula`
    /// return `false`; visual rules (`ColorScale`, `DataBar`, `IconSet`)
    /// return `true`.
    pub fn evaluate(cell_value: &CellValue, rule: &ConditionalRule) -> bool {
        match &rule.rule_type {
            ConditionalRuleType::CellValue { operator, value1, value2 } => {
                let n = match cell_value {
                    CellValue::Number(n) => *n,
                    CellValue::Boolean(true) => 1.0,
                    CellValue::Boolean(false) => 0.0,
                    _ => return false,
                };
                match operator {
                    ComparisonOperator::GreaterThan => n > *value1,
                    ComparisonOperator::LessThan => n < *value1,
                    ComparisonOperator::GreaterThanOrEqual => n >= *value1,
                    ComparisonOperator::LessThanOrEqual => n <= *value1,
                    ComparisonOperator::Equal => (n - *value1).abs() < f64::EPSILON,
                    ComparisonOperator::NotEqual => (n - *value1).abs() >= f64::EPSILON,
                    ComparisonOperator::Between => {
                        let v2 = value2.unwrap_or(*value1);
                        n >= *value1 && n <= v2
                    }
                    ComparisonOperator::NotBetween => {
                        let v2 = value2.unwrap_or(*value1);
                        n < *value1 || n > v2
                    }
                }
            }

            ConditionalRuleType::TextContains(needle) => {
                let text = cell_value_as_text(cell_value);
                text.to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
            }

            ConditionalRuleType::TextStartsWith(prefix) => {
                let text = cell_value_as_text(cell_value);
                text.to_ascii_lowercase()
                    .starts_with(&prefix.to_ascii_lowercase())
            }

            ConditionalRuleType::TextEndsWith(suffix) => {
                let text = cell_value_as_text(cell_value);
                text.to_ascii_lowercase()
                    .ends_with(&suffix.to_ascii_lowercase())
            }

            ConditionalRuleType::IsBlank => matches!(cell_value, CellValue::Empty),

            ConditionalRuleType::IsNotBlank => !matches!(cell_value, CellValue::Empty),

            ConditionalRuleType::IsError => matches!(cell_value, CellValue::Error(_)),

            // Range-context rules cannot be evaluated per-cell alone.
            ConditionalRuleType::DuplicateValues | ConditionalRuleType::UniqueValues => false,

            // Visual-only rules always "match" (they apply styling unconditionally).
            ConditionalRuleType::ColorScale { .. }
            | ConditionalRuleType::DataBar { .. }
            | ConditionalRuleType::IconSet { .. } => true,

            // Formula rules need the formula engine.
            ConditionalRuleType::Formula(_) => false,
        }
    }

    /// Evaluate all rules for a cell and return the merged effective style.
    ///
    /// Rules are evaluated in priority order (lower number = higher priority).
    /// The first matching rule's style is used as the base. If `stop_if_true`
    /// is set on a matching rule, no further rules are evaluated.
    /// Otherwise, subsequent matching rules fill in any style fields that
    /// are still `None`.
    pub fn get_effective_style(
        &self,
        sheet: &str,
        row: u32,
        col: u32,
        cell_value: &CellValue,
    ) -> Option<ConditionalStyle> {
        let rules = self.get_rules(sheet, row, col);
        let mut result: Option<ConditionalStyle> = None;

        for rule in &rules {
            if Self::evaluate(cell_value, rule) {
                let style = &rule.style;
                match result {
                    None => {
                        result = Some(style.clone());
                        if rule.stop_if_true {
                            break;
                        }
                    }
                    Some(ref mut current) => {
                        // Merge: only fill in fields that are still None.
                        if current.bold.is_none() {
                            current.bold = style.bold;
                        }
                        if current.italic.is_none() {
                            current.italic = style.italic;
                        }
                        if current.font_color.is_none() {
                            current.font_color.clone_from(&style.font_color);
                        }
                        if current.bg_color.is_none() {
                            current.bg_color.clone_from(&style.bg_color);
                        }
                        if rule.stop_if_true {
                            break;
                        }
                    }
                }
            }
        }

        result
    }

    /// List all conditional format ranges for a given sheet.
    pub fn list_ranges(&self, sheet: &str) -> Vec<&ConditionalFormatRange> {
        self.ranges
            .iter()
            .filter(|r| r.sheet == sheet)
            .collect()
    }

    /// Remove all conditional formatting for a given sheet.
    pub fn clear(&mut self, sheet: &str) {
        self.ranges.retain(|r| r.sheet != sheet);
    }

    /// Return the total number of conditional format ranges.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Return `true` if no conditional format ranges exist.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}
/// Convert a `CellValue` to its text representation for text-matching rules.
fn cell_value_as_text(value: &CellValue) -> String {
    match value {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(true) => "TRUE".to_string(),
        CellValue::Boolean(false) => "FALSE".to_string(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Empty => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_new_is_empty() {
        let store = ConditionalFormatStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_add_and_get_rule() {
        let mut store = ConditionalFormatStore::new();
        let rule = ConditionalRule {
            rule_type: ConditionalRuleType::CellValue {
                operator: ComparisonOperator::GreaterThan,
                value1: 10.0,
                value2: None,
            },
            style: ConditionalStyle {
                bold: Some(true),
                ..Default::default()
            },
            priority: 1,
            stop_if_true: false,
        };
        store.add_rule("Sheet1", 0, 0, 9, 9, rule);
        assert_eq!(store.len(), 1);

        let rules = store.get_rules("Sheet1", 5, 5);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].priority, 1);
    }
}
