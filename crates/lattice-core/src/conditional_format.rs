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
            ConditionalRuleType::CellValue {
                operator,
                value1,
                value2,
            } => {
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
        self.ranges.iter().filter(|r| r.sheet == sheet).collect()
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
        CellValue::Boolean(true) | CellValue::Checkbox(true) => "TRUE".to_string(),
        CellValue::Boolean(false) | CellValue::Checkbox(false) => "FALSE".to_string(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Empty => String::new(),
        CellValue::Array(_) => "{array}".to_string(),
        CellValue::Lambda { .. } => "{lambda}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellError;

    fn make_rule(rule_type: ConditionalRuleType, priority: u32) -> ConditionalRule {
        ConditionalRule {
            rule_type,
            style: ConditionalStyle {
                bold: Some(true),
                ..Default::default()
            },
            priority,
            stop_if_true: false,
        }
    }

    fn cmp_rule(op: ComparisonOperator, v1: f64, v2: Option<f64>, p: u32) -> ConditionalRule {
        make_rule(
            ConditionalRuleType::CellValue {
                operator: op,
                value1: v1,
                value2: v2,
            },
            p,
        )
    }

    fn gt_rule(val: f64, priority: u32) -> ConditionalRule {
        cmp_rule(ComparisonOperator::GreaterThan, val, None, priority)
    }

    fn eval(val: &CellValue, rule: &ConditionalRule) -> bool {
        ConditionalFormatStore::evaluate(val, rule)
    }

    // ── Store basics ────────────────────────────────────────────────

    #[test]
    fn test_store_new_is_empty() {
        let store = ConditionalFormatStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_add_and_get_rule() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        assert_eq!(store.len(), 1);
        assert_eq!(store.get_rules("S1", 5, 5).len(), 1);
    }

    #[test]
    fn test_add_same_range_appends() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(20.0, 2));
        assert_eq!(store.len(), 1); // still one range
        assert_eq!(store.get_rules("S1", 0, 0).len(), 2);
    }

    #[test]
    fn test_remove_rule() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(20.0, 2));
        assert!(store.remove_rule("S1", 0, 0, 9, 9, 0));
        assert_eq!(store.get_rules("S1", 0, 0).len(), 1);
    }

    #[test]
    fn test_remove_last_rule_removes_range() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        assert!(store.remove_rule("S1", 0, 0, 9, 9, 0));
        assert!(store.is_empty());
    }

    #[test]
    fn test_remove_rule_invalid_index() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        assert!(!store.remove_rule("S1", 0, 0, 9, 9, 5));
    }

    #[test]
    fn test_remove_rule_wrong_range() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        assert!(!store.remove_rule("S1", 0, 0, 5, 5, 0));
    }

    #[test]
    fn test_cell_outside_range_gets_no_rules() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 5, 5, gt_rule(10.0, 1));
        assert!(store.get_rules("S1", 6, 6).is_empty());
    }

    #[test]
    fn test_list_ranges() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 5, 5, gt_rule(10.0, 1));
        store.add_rule("S1", 10, 10, 20, 20, gt_rule(10.0, 1));
        store.add_rule("S2", 0, 0, 5, 5, gt_rule(10.0, 1));
        assert_eq!(store.list_ranges("S1").len(), 2);
        assert_eq!(store.list_ranges("S2").len(), 1);
        assert_eq!(store.list_ranges("S3").len(), 0);
    }

    #[test]
    fn test_clear_sheet() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 5, 5, gt_rule(10.0, 1));
        store.add_rule("S2", 0, 0, 5, 5, gt_rule(10.0, 1));
        store.clear("S1");
        assert!(store.list_ranges("S1").is_empty());
        assert_eq!(store.list_ranges("S2").len(), 1);
    }

    // ── ComparisonOperator tests ────────────────────────────────────

    #[test]
    fn test_greater_than() {
        let r = gt_rule(10.0, 1);
        assert!(eval(&CellValue::Number(11.0), &r));
        assert!(!eval(&CellValue::Number(10.0), &r));
        assert!(!eval(&CellValue::Number(9.0), &r));
    }

    #[test]
    fn test_less_than() {
        let r = cmp_rule(ComparisonOperator::LessThan, 10.0, None, 1);
        assert!(eval(&CellValue::Number(9.0), &r));
        assert!(!eval(&CellValue::Number(10.0), &r));
    }

    #[test]
    fn test_gte_lte() {
        let gte = cmp_rule(ComparisonOperator::GreaterThanOrEqual, 10.0, None, 1);
        assert!(eval(&CellValue::Number(10.0), &gte));
        assert!(!eval(&CellValue::Number(9.99), &gte));
        let lte = cmp_rule(ComparisonOperator::LessThanOrEqual, 10.0, None, 1);
        assert!(eval(&CellValue::Number(10.0), &lte));
        assert!(!eval(&CellValue::Number(10.01), &lte));
    }

    #[test]
    fn test_equal_not_equal() {
        let eq = cmp_rule(ComparisonOperator::Equal, 42.0, None, 1);
        assert!(eval(&CellValue::Number(42.0), &eq));
        assert!(!eval(&CellValue::Number(42.1), &eq));
        let neq = cmp_rule(ComparisonOperator::NotEqual, 42.0, None, 1);
        assert!(!eval(&CellValue::Number(42.0), &neq));
        assert!(eval(&CellValue::Number(43.0), &neq));
    }

    #[test]
    fn test_between() {
        let r = cmp_rule(ComparisonOperator::Between, 10.0, Some(20.0), 1);
        assert!(eval(&CellValue::Number(10.0), &r));
        assert!(eval(&CellValue::Number(15.0), &r));
        assert!(eval(&CellValue::Number(20.0), &r));
        assert!(!eval(&CellValue::Number(9.0), &r));
        assert!(!eval(&CellValue::Number(21.0), &r));
    }

    #[test]
    fn test_not_between() {
        let r = cmp_rule(ComparisonOperator::NotBetween, 10.0, Some(20.0), 1);
        assert!(eval(&CellValue::Number(9.0), &r));
        assert!(eval(&CellValue::Number(21.0), &r));
        assert!(!eval(&CellValue::Number(15.0), &r));
    }

    #[test]
    fn test_cell_value_with_boolean() {
        let r = gt_rule(0.5, 1);
        assert!(eval(&CellValue::Boolean(true), &r));
        assert!(!eval(&CellValue::Boolean(false), &r));
    }

    #[test]
    fn test_cell_value_rejects_text() {
        let r = gt_rule(10.0, 1);
        assert!(!eval(&CellValue::Text("hello".into()), &r));
    }

    // ── Text rules ──────────────────────────────────────────────────

    #[test]
    fn test_text_contains() {
        let r = make_rule(ConditionalRuleType::TextContains("hello".into()), 1);
        assert!(eval(&CellValue::Text("say HELLO world".into()), &r));
        assert!(!eval(&CellValue::Text("goodbye".into()), &r));
    }

    #[test]
    fn test_text_starts_with() {
        let r = make_rule(ConditionalRuleType::TextStartsWith("Pre".into()), 1);
        assert!(eval(&CellValue::Text("prefix".into()), &r));
        assert!(!eval(&CellValue::Text("not prefix".into()), &r));
    }

    #[test]
    fn test_text_ends_with() {
        let r = make_rule(ConditionalRuleType::TextEndsWith("FIX".into()), 1);
        assert!(eval(&CellValue::Text("suffix".into()), &r));
        assert!(!eval(&CellValue::Text("fixme".into()), &r));
    }

    #[test]
    fn test_text_contains_number_value() {
        let r = make_rule(ConditionalRuleType::TextContains("42".into()), 1);
        assert!(eval(&CellValue::Number(42.0), &r));
        assert!(!eval(&CellValue::Number(43.0), &r));
    }

    #[test]
    fn test_text_contains_empty_needle() {
        let r = make_rule(ConditionalRuleType::TextContains("".into()), 1);
        assert!(eval(&CellValue::Text("anything".into()), &r));
        assert!(eval(&CellValue::Empty, &r));
    }

    // ── IsBlank / IsNotBlank / IsError ──────────────────────────────

    #[test]
    fn test_is_blank() {
        let r = make_rule(ConditionalRuleType::IsBlank, 1);
        assert!(eval(&CellValue::Empty, &r));
        assert!(!eval(&CellValue::Text("hi".into()), &r));
        assert!(!eval(&CellValue::Number(0.0), &r));
    }

    #[test]
    fn test_is_not_blank() {
        let r = make_rule(ConditionalRuleType::IsNotBlank, 1);
        assert!(!eval(&CellValue::Empty, &r));
        assert!(eval(&CellValue::Number(0.0), &r));
        assert!(eval(&CellValue::Text("".into()), &r));
    }

    #[test]
    fn test_is_error() {
        let r = make_rule(ConditionalRuleType::IsError, 1);
        assert!(eval(&CellValue::Error(CellError::DivZero), &r));
        assert!(eval(&CellValue::Error(CellError::Ref), &r));
        assert!(!eval(&CellValue::Number(1.0), &r));
        assert!(!eval(&CellValue::Empty, &r));
    }

    // ── Visual rules ────────────────────────────────────────────────

    #[test]
    fn test_color_scale_always_matches() {
        let r = make_rule(
            ConditionalRuleType::ColorScale {
                min_color: "#FF0000".into(),
                max_color: "#00FF00".into(),
                mid_color: None,
            },
            1,
        );
        assert!(eval(&CellValue::Number(50.0), &r));
        assert!(eval(&CellValue::Empty, &r));
    }

    #[test]
    fn test_data_bar_always_matches() {
        let r = make_rule(
            ConditionalRuleType::DataBar {
                color: "#0000FF".into(),
                max_length_percent: 100,
            },
            1,
        );
        assert!(eval(&CellValue::Number(0.0), &r));
    }

    #[test]
    fn test_icon_set_always_matches() {
        let r = make_rule(
            ConditionalRuleType::IconSet {
                icons: vec!["up".into(), "down".into()],
                thresholds: vec![50.0],
            },
            1,
        );
        assert!(eval(&CellValue::Number(0.0), &r));
    }

    // ── DuplicateValues / UniqueValues / Formula return false ────────

    #[test]
    fn test_duplicate_unique_formula_return_false() {
        let dup = make_rule(ConditionalRuleType::DuplicateValues, 1);
        let uniq = make_rule(ConditionalRuleType::UniqueValues, 1);
        let formula = make_rule(ConditionalRuleType::Formula("=A1>0".into()), 1);
        let val = CellValue::Number(42.0);
        assert!(!eval(&val, &dup));
        assert!(!eval(&val, &uniq));
        assert!(!eval(&val, &formula));
    }

    // ── Priority ordering ───────────────────────────────────────────

    #[test]
    fn test_rules_sorted_by_priority() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 5, 5, gt_rule(10.0, 3));
        store.add_rule("S1", 0, 0, 5, 5, gt_rule(20.0, 1));
        store.add_rule("S1", 0, 0, 5, 5, gt_rule(15.0, 2));
        let rules = store.get_rules("S1", 0, 0);
        assert_eq!(rules[0].priority, 1);
        assert_eq!(rules[1].priority, 2);
        assert_eq!(rules[2].priority, 3);
    }

    // ── stop_if_true ────────────────────────────────────────────────

    #[test]
    fn test_stop_if_true_prevents_merging() {
        let mut store = ConditionalFormatStore::new();
        let mut r1 = cmp_rule(ComparisonOperator::GreaterThan, 0.0, None, 1);
        r1.style = ConditionalStyle {
            bold: Some(true),
            ..Default::default()
        };
        r1.stop_if_true = true;
        let mut r2 = cmp_rule(ComparisonOperator::GreaterThan, 0.0, None, 2);
        r2.style = ConditionalStyle {
            italic: Some(true),
            ..Default::default()
        };
        store.add_rule("S1", 0, 0, 9, 9, r1);
        store.add_rule("S1", 0, 0, 9, 9, r2);
        let style = store
            .get_effective_style("S1", 0, 0, &CellValue::Number(5.0))
            .unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.italic, None); // r2 was not evaluated
    }

    // ── get_effective_style merging ─────────────────────────────────

    #[test]
    fn test_effective_style_merges_fields() {
        let mut store = ConditionalFormatStore::new();
        let mut r1 = cmp_rule(ComparisonOperator::GreaterThan, 0.0, None, 1);
        r1.style = ConditionalStyle {
            bold: Some(true),
            italic: None,
            font_color: Some("#FF0000".into()),
            bg_color: None,
        };
        let mut r2 = cmp_rule(ComparisonOperator::GreaterThan, 0.0, None, 2);
        r2.style = ConditionalStyle {
            bold: Some(false),
            italic: Some(true),
            font_color: Some("#0000FF".into()),
            bg_color: Some("#FFFF00".into()),
        };
        store.add_rule("S1", 0, 0, 9, 9, r1);
        store.add_rule("S1", 0, 0, 9, 9, r2);
        let style = store
            .get_effective_style("S1", 0, 0, &CellValue::Number(5.0))
            .unwrap();
        assert_eq!(style.bold, Some(true)); // from r1 (first match)
        assert_eq!(style.italic, Some(true)); // from r2 (r1 had None)
        assert_eq!(style.font_color, Some("#FF0000".into())); // from r1
        assert_eq!(style.bg_color, Some("#FFFF00".into())); // from r2
    }

    #[test]
    fn test_effective_style_no_match_returns_none() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(100.0, 1));
        assert!(
            store
                .get_effective_style("S1", 0, 0, &CellValue::Number(5.0))
                .is_none()
        );
    }

    #[test]
    fn test_effective_style_empty_cell_no_match() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 9, 9, gt_rule(10.0, 1));
        assert!(
            store
                .get_effective_style("S1", 0, 0, &CellValue::Empty)
                .is_none()
        );
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn test_between_without_value2_uses_value1() {
        let r = cmp_rule(ComparisonOperator::Between, 10.0, None, 1);
        assert!(eval(&CellValue::Number(10.0), &r)); // degrades to Equal
        assert!(!eval(&CellValue::Number(11.0), &r));
    }

    #[test]
    fn test_comparison_with_negative_numbers() {
        let r = gt_rule(-5.0, 1);
        assert!(eval(&CellValue::Number(-4.0), &r));
        assert!(!eval(&CellValue::Number(-5.0), &r));
        assert!(!eval(&CellValue::Number(-6.0), &r));
    }

    #[test]
    fn test_comparison_with_zero() {
        let eq = cmp_rule(ComparisonOperator::Equal, 0.0, None, 1);
        assert!(eval(&CellValue::Number(0.0), &eq));
        assert!(eval(&CellValue::Boolean(false), &eq));
    }

    #[test]
    fn test_overlapping_ranges_return_all_rules() {
        let mut store = ConditionalFormatStore::new();
        store.add_rule("S1", 0, 0, 10, 10, gt_rule(5.0, 1));
        store.add_rule("S1", 5, 5, 15, 15, gt_rule(10.0, 2));
        // Cell (7, 7) is inside both ranges
        let rules = store.get_rules("S1", 7, 7);
        assert_eq!(rules.len(), 2);
    }
}
