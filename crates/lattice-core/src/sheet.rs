use std::collections::HashMap;

use crate::cell::{Cell, CellValue};

/// A single sheet inside a workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    /// Sheet name.
    pub name: String,
    /// Sparse cell storage keyed by (row, col) — both 0-based.
    cells: HashMap<(u32, u32), Cell>,
    /// Per-column width overrides.
    pub col_widths: HashMap<u32, f64>,
    /// Per-row height overrides.
    pub row_heights: HashMap<u32, f64>,
}

use serde::{Deserialize, Serialize};

impl Sheet {
    /// Create a new empty sheet with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cells: HashMap::new(),
            col_widths: HashMap::new(),
            row_heights: HashMap::new(),
        }
    }

    /// Return a reference to the cell at `(row, col)`, if it exists.
    pub fn get_cell(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.get(&(row, col))
    }

    /// Insert or replace a full `Cell` at `(row, col)`.
    pub fn set_cell(&mut self, row: u32, col: u32, cell: Cell) {
        self.cells.insert((row, col), cell);
    }

    /// Convenience: set only the value of a cell (creates a default cell if needed).
    pub fn set_value(&mut self, row: u32, col: u32, value: CellValue) {
        let cell = self.cells.entry((row, col)).or_default();
        cell.value = value;
    }

    /// Remove a cell entirely.
    pub fn clear_cell(&mut self, row: u32, col: u32) {
        self.cells.remove(&(row, col));
    }

    /// Return the bounding rectangle `(max_row, max_col)` that contains all
    /// non-empty cells. Returns `(0, 0)` if the sheet is empty.
    pub fn used_range(&self) -> (u32, u32) {
        if self.cells.is_empty() {
            return (0, 0);
        }
        let max_row = self.cells.keys().map(|(r, _)| *r).max().unwrap_or(0);
        let max_col = self.cells.keys().map(|(_, c)| *c).max().unwrap_or(0);
        (max_row, max_col)
    }

    /// Return a reference to the underlying cell map (read-only).
    pub fn cells(&self) -> &HashMap<(u32, u32), Cell> {
        &self.cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_used_range_empty() {
        let sheet = Sheet::new("S1");
        assert_eq!(sheet.used_range(), (0, 0));
    }

    #[test]
    fn test_used_range_with_cells() {
        let mut sheet = Sheet::new("S1");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(5, 3, CellValue::Number(2.0));
        assert_eq!(sheet.used_range(), (5, 3));
    }

    #[test]
    fn test_set_get_clear() {
        let mut sheet = Sheet::new("S1");
        sheet.set_value(1, 2, CellValue::Text("hello".into()));
        assert_eq!(
            sheet.get_cell(1, 2).unwrap().value,
            CellValue::Text("hello".into())
        );
        sheet.clear_cell(1, 2);
        assert!(sheet.get_cell(1, 2).is_none());
    }
}
