//! Named filter view management.
//!
//! A filter view is a saved, named set of column filter configurations
//! that can be applied to a sheet to show/hide rows. Users can switch
//! between different filter views to quickly change which data is visible.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cell::CellValue;
use crate::error::{LatticeError, Result};
use crate::sheet::Sheet;

/// A named filter view that stores column-level filter criteria.
///
/// Each column maps to a list of allowed string values. When applied,
/// rows whose cell text (case-insensitive) for a filtered column is NOT
/// in the allowed list are hidden.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterView {
    /// Human-readable name for this filter view.
    pub name: String,
    /// Column filters: maps 0-based column index to allowed values.
    /// Only rows where the cell's text representation matches one of
    /// the allowed values (case-insensitive) are shown.
    pub column_filters: HashMap<u32, Vec<String>>,
}

/// A store of named filter views.
#[derive(Debug, Clone, Default)]
pub struct FilterViewStore {
    views: Vec<FilterView>,
}

impl FilterViewStore {
    /// Create a new empty filter view store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new filter view.
    ///
    /// Returns an error if a view with the same name already exists
    /// (case-insensitive).
    pub fn add(&mut self, name: impl Into<String>, column_filters: HashMap<u32, Vec<String>>) -> Result<()> {
        let name = name.into();
        if name.is_empty() {
            return Err(LatticeError::Internal(
                "filter view name cannot be empty".into(),
            ));
        }
        let lower = name.to_lowercase();
        if self.views.iter().any(|v| v.name.to_lowercase() == lower) {
            return Err(LatticeError::Internal(format!(
                "filter view '{}' already exists",
                name
            )));
        }
        self.views.push(FilterView {
            name,
            column_filters,
        });
        Ok(())
    }

    /// Remove a filter view by name (case-insensitive).
    ///
    /// Returns an error if the view does not exist.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let lower = name.to_lowercase();
        let len_before = self.views.len();
        self.views.retain(|v| v.name.to_lowercase() != lower);
        if self.views.len() == len_before {
            return Err(LatticeError::Internal(format!(
                "filter view '{}' not found",
                name
            )));
        }
        Ok(())
    }

    /// Get a filter view by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&FilterView> {
        let lower = name.to_lowercase();
        self.views.iter().find(|v| v.name.to_lowercase() == lower)
    }

    /// List all filter views.
    pub fn list(&self) -> &[FilterView] {
        &self.views
    }

    /// Apply a named filter view to a sheet.
    ///
    /// First unhides all rows, then hides rows that do not match the
    /// filter criteria. The header row (row 0) is never hidden.
    ///
    /// Returns the number of rows hidden, or an error if the view is not found.
    pub fn apply(&self, sheet: &mut Sheet, view_name: &str) -> Result<u32> {
        let view = self.get(view_name).ok_or_else(|| {
            LatticeError::Internal(format!("filter view '{}' not found", view_name))
        })?;
        apply_filter_view(sheet, view)
    }
}

/// Apply a filter view to a sheet, hiding non-matching rows.
///
/// Row 0 is treated as the header row and is never hidden.
/// All data rows (1..=max_row) are first unhidden, then rows that
/// fail any column filter are hidden.
///
/// Returns the number of rows hidden.
pub fn apply_filter_view(sheet: &mut Sheet, view: &FilterView) -> Result<u32> {
    let (max_row, _) = sheet.used_range();
    if max_row == 0 {
        return Ok(0);
    }

    // Unhide all data rows first.
    sheet.unhide_rows(1, max_row);

    if view.column_filters.is_empty() {
        return Ok(0);
    }

    let mut hidden_count = 0u32;

    for row in 1..=max_row {
        let mut passes = true;

        for (col, allowed_values) in &view.column_filters {
            let cell_text = sheet
                .get_cell(row, *col)
                .map(|c| cell_value_to_lower(&c.value))
                .unwrap_or_default();

            let allowed_lower: Vec<String> =
                allowed_values.iter().map(|v| v.to_lowercase()).collect();

            // Check for blanks handling
            let is_blank = cell_text.is_empty();
            let allow_blanks = allowed_lower.iter().any(|v| v == "(blanks)");

            let matches = if is_blank {
                allow_blanks
            } else {
                allowed_lower.iter().any(|v| v != "(blanks)" && *v == cell_text)
            };

            if !matches {
                passes = false;
                break;
            }
        }

        if !passes {
            sheet.hide_rows(row, 1);
            hidden_count += 1;
        }
    }

    Ok(hidden_count)
}

/// Convert a CellValue to its lowercase string representation.
fn cell_value_to_lower(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.to_lowercase(),
        CellValue::Number(n) => n.to_string().to_lowercase(),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => b.to_string().to_lowercase(),
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string().to_lowercase(),
        CellValue::Date(s) => s.to_lowercase(),
        CellValue::Array(_) => "{array}".to_string(),
        CellValue::Lambda { .. } => "{lambda}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_filter_view() {
        let mut store = FilterViewStore::new();
        let mut filters = HashMap::new();
        filters.insert(0, vec!["apple".to_string(), "banana".to_string()]);
        store.add("Fruits", filters).unwrap();
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].name, "Fruits");
    }

    #[test]
    fn test_add_duplicate_name_errors() {
        let mut store = FilterViewStore::new();
        store.add("Test", HashMap::new()).unwrap();
        let err = store.add("test", HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn test_add_empty_name_errors() {
        let mut store = FilterViewStore::new();
        let err = store.add("", HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_remove_filter_view() {
        let mut store = FilterViewStore::new();
        store.add("A", HashMap::new()).unwrap();
        store.add("B", HashMap::new()).unwrap();
        store.remove("a").unwrap(); // case-insensitive
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].name, "B");
    }

    #[test]
    fn test_remove_nonexistent_errors() {
        let mut store = FilterViewStore::new();
        let err = store.remove("nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_get_filter_view() {
        let mut store = FilterViewStore::new();
        let mut filters = HashMap::new();
        filters.insert(1, vec!["x".to_string()]);
        store.add("MyView", filters).unwrap();

        let view = store.get("myview").unwrap();
        assert_eq!(view.name, "MyView");
        assert_eq!(view.column_filters.get(&1).unwrap(), &vec!["x".to_string()]);
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let store = FilterViewStore::new();
        assert!(store.get("nope").is_none());
    }

    #[test]
    fn test_apply_filter_view_hides_rows() {
        let mut sheet = Sheet::new("Test");
        // Row 0: header
        sheet.set_value(0, 0, CellValue::Text("Name".into()));
        // Row 1: apple
        sheet.set_value(1, 0, CellValue::Text("apple".into()));
        // Row 2: banana
        sheet.set_value(2, 0, CellValue::Text("banana".into()));
        // Row 3: apple
        sheet.set_value(3, 0, CellValue::Text("apple".into()));

        let view = FilterView {
            name: "Apples".into(),
            column_filters: {
                let mut m = HashMap::new();
                m.insert(0, vec!["apple".to_string()]);
                m
            },
        };

        let hidden = apply_filter_view(&mut sheet, &view).unwrap();
        assert_eq!(hidden, 1); // banana hidden
        assert!(!sheet.is_row_hidden(0)); // header never hidden
        assert!(!sheet.is_row_hidden(1)); // apple visible
        assert!(sheet.is_row_hidden(2));  // banana hidden
        assert!(!sheet.is_row_hidden(3)); // apple visible
    }

    #[test]
    fn test_apply_multi_col_and_unhide() {
        // Multi-column filter
        let mut sheet = Sheet::new("Test");
        sheet.set_value(0, 0, CellValue::Text("Fruit".into()));
        sheet.set_value(0, 1, CellValue::Text("Color".into()));
        sheet.set_value(1, 0, CellValue::Text("apple".into()));
        sheet.set_value(1, 1, CellValue::Text("red".into()));
        sheet.set_value(2, 0, CellValue::Text("banana".into()));
        sheet.set_value(2, 1, CellValue::Text("yellow".into()));
        sheet.set_value(3, 0, CellValue::Text("apple".into()));
        sheet.set_value(3, 1, CellValue::Text("green".into()));

        let view = FilterView {
            name: "RedApples".into(),
            column_filters: {
                let mut m = HashMap::new();
                m.insert(0, vec!["apple".to_string()]);
                m.insert(1, vec!["red".to_string()]);
                m
            },
        };
        let hidden = apply_filter_view(&mut sheet, &view).unwrap();
        assert_eq!(hidden, 2);
        assert!(!sheet.is_row_hidden(1));
        assert!(sheet.is_row_hidden(2));

        // Applying empty filter should unhide all rows
        let show_all = FilterView { name: "All".into(), column_filters: HashMap::new() };
        apply_filter_view(&mut sheet, &show_all).unwrap();
        assert!(!sheet.is_row_hidden(2));
        assert!(!sheet.is_row_hidden(3));
    }

    #[test]
    fn test_apply_blanks_and_store() {
        // Blanks filter
        let mut sheet = Sheet::new("Test");
        sheet.set_value(0, 0, CellValue::Text("Val".into()));
        sheet.set_value(1, 0, CellValue::Text("x".into()));
        sheet.set_value(3, 0, CellValue::Text("y".into()));

        let view = FilterView {
            name: "BlanksOnly".into(),
            column_filters: {
                let mut m = HashMap::new();
                m.insert(0, vec!["(Blanks)".to_string()]);
                m
            },
        };
        let hidden = apply_filter_view(&mut sheet, &view).unwrap();
        assert_eq!(hidden, 2);
        assert!(!sheet.is_row_hidden(2));

        // Store.apply
        let mut store = FilterViewStore::new();
        store.add("BlanksOnly", view.column_filters.clone()).unwrap();
        let hidden = store.apply(&mut sheet, "BlanksOnly").unwrap();
        assert_eq!(hidden, 2);

        // Nonexistent view
        let err = store.apply(&mut sheet, "nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_apply_empty_sheet_and_case_and_numeric() {
        // Empty sheet
        let mut sheet = Sheet::new("Test");
        let view = FilterView {
            name: "E".into(),
            column_filters: { let mut m = HashMap::new(); m.insert(0, vec!["x".into()]); m },
        };
        assert_eq!(apply_filter_view(&mut sheet, &view).unwrap(), 0);

        // Case-insensitive matching
        let mut sheet2 = Sheet::new("T2");
        sheet2.set_value(0, 0, CellValue::Text("Name".into()));
        sheet2.set_value(1, 0, CellValue::Text("Apple".into()));
        sheet2.set_value(2, 0, CellValue::Text("BANANA".into()));
        let view2 = FilterView {
            name: "C".into(),
            column_filters: { let mut m = HashMap::new(); m.insert(0, vec!["apple".into()]); m },
        };
        assert_eq!(apply_filter_view(&mut sheet2, &view2).unwrap(), 1);
        assert!(!sheet2.is_row_hidden(1));
        assert!(sheet2.is_row_hidden(2));

        // Numeric matching
        let mut sheet3 = Sheet::new("T3");
        sheet3.set_value(0, 0, CellValue::Text("Score".into()));
        sheet3.set_value(1, 0, CellValue::Number(100.0));
        sheet3.set_value(2, 0, CellValue::Number(200.0));
        let view3 = FilterView {
            name: "N".into(),
            column_filters: { let mut m = HashMap::new(); m.insert(0, vec!["100".into()]); m },
        };
        assert_eq!(apply_filter_view(&mut sheet3, &view3).unwrap(), 1);
        assert!(!sheet3.is_row_hidden(1));
        assert!(sheet3.is_row_hidden(2));
    }
}
