//! Filter and auto-filter operations for spreadsheet ranges.
//!
//! Supports filtering rows based on column conditions including equals,
//! not equals, contains, greater than, less than, is empty, is not empty.

use serde::{Deserialize, Serialize};

use crate::cell::CellValue;
use crate::error::Result;
use crate::sheet::Sheet;

/// A filter condition for an auto-filter column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterCondition {
    /// Show cells whose text equals the given string (case-insensitive).
    Equals(String),
    /// Show cells whose text does NOT equal the given string.
    NotEquals(String),
    /// Show cells whose text contains the given substring (case-insensitive).
    Contains(String),
    /// Show cells whose numeric value is greater than the threshold.
    GreaterThan(f64),
    /// Show cells whose numeric value is less than the threshold.
    LessThan(f64),
    /// Show cells whose numeric value is greater than or equal to the threshold.
    GreaterThanOrEqual(f64),
    /// Show cells whose numeric value is less than or equal to the threshold.
    LessThanOrEqual(f64),
    /// Show non-empty cells.
    NonEmpty,
    /// Show empty cells.
    IsEmpty,
}

/// Per-column filter state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnFilter {
    /// 0-based column index.
    pub col: u32,
    /// Condition to apply.
    pub condition: FilterCondition,
}

/// Auto-filter applied to a range of columns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoFilter {
    /// Individual column filters.
    pub filters: Vec<ColumnFilter>,
}

impl AutoFilter {
    /// Create a new empty auto-filter.
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Add a filter condition for a column.
    pub fn add_filter(&mut self, col: u32, condition: FilterCondition) {
        self.filters.push(ColumnFilter { col, condition });
    }

    /// Remove all filters for a specific column.
    pub fn remove_filter(&mut self, col: u32) {
        self.filters.retain(|f| f.col != col);
    }

    /// Clear all filters.
    pub fn clear(&mut self) {
        self.filters.clear();
    }

    /// Check if a row passes all filter conditions.
    ///
    /// `row_values` maps column index to the cell value in that column for
    /// the row being tested. Missing columns are treated as empty.
    pub fn matches_row(&self, row_values: &[(u32, &CellValue)]) -> bool {
        for filter in &self.filters {
            let cell_val = row_values
                .iter()
                .find(|(c, _)| *c == filter.col)
                .map(|(_, v)| *v)
                .unwrap_or(&CellValue::Empty);

            if !matches_condition(cell_val, &filter.condition) {
                return false;
            }
        }
        true
    }
}

impl Default for AutoFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a cell value matches a filter condition.
fn matches_condition(val: &CellValue, condition: &FilterCondition) -> bool {
    match condition {
        FilterCondition::Equals(s) => {
            let cell_str = cell_value_to_string(val);
            cell_str.to_lowercase() == s.to_lowercase()
        }
        FilterCondition::NotEquals(s) => {
            let cell_str = cell_value_to_string(val);
            cell_str.to_lowercase() != s.to_lowercase()
        }
        FilterCondition::Contains(s) => {
            let cell_str = cell_value_to_string(val);
            cell_str.to_lowercase().contains(&s.to_lowercase())
        }
        FilterCondition::GreaterThan(threshold) => {
            if let Some(n) = cell_value_as_number(val) {
                n > *threshold
            } else {
                false
            }
        }
        FilterCondition::LessThan(threshold) => {
            if let Some(n) = cell_value_as_number(val) {
                n < *threshold
            } else {
                false
            }
        }
        FilterCondition::GreaterThanOrEqual(threshold) => {
            if let Some(n) = cell_value_as_number(val) {
                n >= *threshold
            } else {
                false
            }
        }
        FilterCondition::LessThanOrEqual(threshold) => {
            if let Some(n) = cell_value_as_number(val) {
                n <= *threshold
            } else {
                false
            }
        }
        FilterCondition::NonEmpty => !matches!(val, CellValue::Empty),
        FilterCondition::IsEmpty => matches!(val, CellValue::Empty),
    }
}

/// Convert a CellValue to a display string.
fn cell_value_to_string(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) => b.to_string(),
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
    }
}

/// Try to extract a numeric value from a CellValue.
fn cell_value_as_number(val: &CellValue) -> Option<f64> {
    match val {
        CellValue::Number(n) => Some(*n),
        CellValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Filter rows in a sheet range, returning the 0-based row indices that
/// match all filter conditions.
///
/// Scans rows from `start_row` to `end_row` inclusive and returns the
/// indices of rows that pass all filters.
pub fn filter_rows(
    sheet: &Sheet,
    start_row: u32,
    end_row: u32,
    start_col: u32,
    end_col: u32,
    filter: &AutoFilter,
) -> Result<Vec<u32>> {
    let mut matching_rows = Vec::new();

    for row in start_row..=end_row {
        let mut row_values: Vec<(u32, &CellValue)> = Vec::new();
        for col in start_col..=end_col {
            if let Some(cell) = sheet.get_cell(row, col) {
                row_values.push((col, &cell.value));
            }
        }

        if filter.matches_row(&row_values) {
            matching_rows.push(row);
        }
    }

    Ok(matching_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_equals() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("apple".into()));
        sheet.set_value(1, 0, CellValue::Text("banana".into()));
        sheet.set_value(2, 0, CellValue::Text("apple".into()));

        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::Equals("apple".into()));

        let rows = filter_rows(&sheet, 0, 2, 0, 0, &filter).unwrap();
        assert_eq!(rows, vec![0, 2]);
    }

    #[test]
    fn test_filter_not_equals() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("apple".into()));
        sheet.set_value(1, 0, CellValue::Text("banana".into()));
        sheet.set_value(2, 0, CellValue::Text("apple".into()));

        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::NotEquals("apple".into()));

        let rows = filter_rows(&sheet, 0, 2, 0, 0, &filter).unwrap();
        assert_eq!(rows, vec![1]);
    }

    #[test]
    fn test_filter_greater_than() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(1, 0, CellValue::Number(20.0));
        sheet.set_value(2, 0, CellValue::Number(30.0));

        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::GreaterThan(15.0));

        let rows = filter_rows(&sheet, 0, 2, 0, 0, &filter).unwrap();
        assert_eq!(rows, vec![1, 2]);
    }

    #[test]
    fn test_filter_contains() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("hello world".into()));
        sheet.set_value(1, 0, CellValue::Text("goodbye".into()));
        sheet.set_value(2, 0, CellValue::Text("hello there".into()));

        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::Contains("hello".into()));

        let rows = filter_rows(&sheet, 0, 2, 0, 0, &filter).unwrap();
        assert_eq!(rows, vec![0, 2]);
    }

    #[test]
    fn test_filter_non_empty() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        // row 1 is empty
        sheet.set_value(2, 0, CellValue::Number(3.0));

        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::NonEmpty);

        let rows = filter_rows(&sheet, 0, 2, 0, 0, &filter).unwrap();
        assert_eq!(rows, vec![0, 2]);
    }

    #[test]
    fn test_filter_is_empty() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        // row 1 is empty
        sheet.set_value(2, 0, CellValue::Number(3.0));

        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::IsEmpty);

        let rows = filter_rows(&sheet, 0, 2, 0, 0, &filter).unwrap();
        assert_eq!(rows, vec![1]);
    }

    #[test]
    fn test_filter_multiple_conditions() {
        let mut sheet = Sheet::new("T");
        // Col A: name, Col B: score
        sheet.set_value(0, 0, CellValue::Text("Alice".into()));
        sheet.set_value(0, 1, CellValue::Number(90.0));
        sheet.set_value(1, 0, CellValue::Text("Bob".into()));
        sheet.set_value(1, 1, CellValue::Number(60.0));
        sheet.set_value(2, 0, CellValue::Text("Charlie".into()));
        sheet.set_value(2, 1, CellValue::Number(85.0));

        let mut filter = AutoFilter::new();
        filter.add_filter(1, FilterCondition::GreaterThanOrEqual(80.0));

        let rows = filter_rows(&sheet, 0, 2, 0, 1, &filter).unwrap();
        assert_eq!(rows, vec![0, 2]); // Alice and Charlie
    }

    #[test]
    fn test_add_and_remove_filter() {
        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::Equals("x".into()));
        filter.add_filter(1, FilterCondition::GreaterThan(5.0));
        assert_eq!(filter.filters.len(), 2);

        filter.remove_filter(0);
        assert_eq!(filter.filters.len(), 1);
        assert_eq!(filter.filters[0].col, 1);
    }

    #[test]
    fn test_clear_filters() {
        let mut filter = AutoFilter::new();
        filter.add_filter(0, FilterCondition::Equals("x".into()));
        filter.add_filter(1, FilterCondition::NonEmpty);
        filter.clear();
        assert!(filter.filters.is_empty());
    }
}
