//! Sheet data structure with sparse cell storage.
//!
//! A `Sheet` holds cells in a `HashMap<(u32, u32), Cell>` keyed by
//! (row, col), along with column widths, row heights, merged regions,
//! and comment management.

use std::collections::{HashMap, HashSet};

use crate::cell::{Cell, CellValue};
use crate::error::{LatticeError, Result};

use serde::{Deserialize, Serialize};

/// A merged region defined by its top-left and bottom-right corners
/// (inclusive, 0-based).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergedRegion {
    /// Start row (0-based).
    pub start_row: u32,
    /// Start column (0-based).
    pub start_col: u32,
    /// End row (0-based, inclusive).
    pub end_row: u32,
    /// End column (0-based, inclusive).
    pub end_col: u32,
}

/// Protection settings for a sheet.
///
/// Controls what operations are allowed when the sheet is protected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetProtection {
    /// Whether the sheet is currently protected.
    pub is_protected: bool,
    /// Optional password hash (SHA-256 hex digest) used to lock/unlock.
    pub password_hash: Option<String>,
    /// Allow users to select cells even when protected.
    pub allow_select: bool,
    /// Allow users to sort even when protected.
    pub allow_sort: bool,
    /// Allow users to use auto-filter even when protected.
    pub allow_filter: bool,
}

impl Default for SheetProtection {
    fn default() -> Self {
        Self {
            is_protected: true,
            password_hash: None,
            allow_select: true,
            allow_sort: false,
            allow_filter: false,
        }
    }
}

/// A protected range within a sheet that cannot be edited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtectedRange {
    /// Start row (0-based).
    pub start_row: u32,
    /// Start column (0-based).
    pub start_col: u32,
    /// End row (0-based, inclusive).
    pub end_row: u32,
    /// End column (0-based, inclusive).
    pub end_col: u32,
    /// Optional human-readable description.
    pub description: Option<String>,
}

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
    /// Merged cell regions.
    merged_regions: Vec<MergedRegion>,
    /// Set of hidden row indices (0-based).
    pub hidden_rows: HashSet<u32>,
    /// Set of hidden column indices (0-based).
    pub hidden_cols: HashSet<u32>,
    /// Optional sheet-level protection settings.
    pub protection: Option<SheetProtection>,
    /// Protected ranges that cannot be edited.
    protected_ranges: Vec<ProtectedRange>,
    /// Optional tab color as a CSS hex string (e.g. `"#FF0000"`).
    pub tab_color: Option<String>,
}

impl Sheet {
    /// Create a new empty sheet with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cells: HashMap::new(),
            col_widths: HashMap::new(),
            row_heights: HashMap::new(),
            merged_regions: Vec::new(),
            hidden_rows: HashSet::new(),
            hidden_cols: HashSet::new(),
            protection: None,
            protected_ranges: Vec::new(),
            tab_color: None,
        }
    }

    // ----- Cell access -----

    /// Return a reference to the cell at `(row, col)`, if it exists.
    pub fn get_cell(&self, row: u32, col: u32) -> Option<&Cell> {
        self.cells.get(&(row, col))
    }

    /// Return a mutable reference to the cell at `(row, col)`, if it exists.
    pub fn get_cell_mut(&mut self, row: u32, col: u32) -> Option<&mut Cell> {
        self.cells.get_mut(&(row, col))
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

    /// Return a mutable reference to the underlying cell map.
    pub fn cells_mut(&mut self) -> &mut HashMap<(u32, u32), Cell> {
        &mut self.cells
    }

    // ----- Comments -----

    /// Set a comment on a cell. Creates the cell if it does not exist.
    pub fn set_comment(&mut self, row: u32, col: u32, comment: impl Into<String>) {
        let cell = self.cells.entry((row, col)).or_default();
        cell.comment = Some(comment.into());
    }

    /// Get the comment on a cell, if any.
    pub fn get_comment(&self, row: u32, col: u32) -> Option<&str> {
        self.cells
            .get(&(row, col))
            .and_then(|c| c.comment.as_deref())
    }

    /// Remove the comment from a cell.
    pub fn remove_comment(&mut self, row: u32, col: u32) {
        if let Some(cell) = self.cells.get_mut(&(row, col)) {
            cell.comment = None;
        }
    }

    // ----- Merged regions -----

    /// Merge a rectangular region. The value of the merged cell comes from
    /// the top-left cell. All other cells in the region are cleared.
    ///
    /// Returns an error if the region overlaps an existing merged region.
    pub fn merge_cells(
        &mut self,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Result<()> {
        // Check for overlap with existing merged regions
        for region in &self.merged_regions {
            if regions_overlap(
                start_row, start_col, end_row, end_col, region.start_row, region.start_col,
                region.end_row, region.end_col,
            ) {
                return Err(LatticeError::InvalidRange(
                    "merge region overlaps an existing merged region".into(),
                ));
            }
        }

        // Clear all cells except the top-left
        for r in start_row..=end_row {
            for c in start_col..=end_col {
                if r != start_row || c != start_col {
                    self.cells.remove(&(r, c));
                }
            }
        }

        self.merged_regions.push(MergedRegion {
            start_row,
            start_col,
            end_row,
            end_col,
        });

        Ok(())
    }

    /// Unmerge a previously merged region that contains the given cell.
    ///
    /// Returns `Ok(true)` if a region was unmerged, `Ok(false)` if the cell
    /// was not part of any merged region.
    pub fn unmerge_cell(&mut self, row: u32, col: u32) -> Result<bool> {
        let idx = self.merged_regions.iter().position(|r| {
            row >= r.start_row
                && row <= r.end_row
                && col >= r.start_col
                && col <= r.end_col
        });

        match idx {
            Some(i) => {
                self.merged_regions.remove(i);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Return a reference to the list of merged regions.
    pub fn merged_regions(&self) -> &[MergedRegion] {
        &self.merged_regions
    }

    /// Check if a cell is part of a merged region. If so, return the region.
    pub fn get_merged_region(&self, row: u32, col: u32) -> Option<&MergedRegion> {
        self.merged_regions.iter().find(|r| {
            row >= r.start_row
                && row <= r.end_row
                && col >= r.start_col
                && col <= r.end_col
        })
    }

    // ----- Hidden Rows / Columns -----

    /// Hide a range of rows starting at `start` for `count` rows.
    ///
    /// Already-hidden rows in the range are silently ignored.
    pub fn hide_rows(&mut self, start: u32, count: u32) {
        for r in start..start + count {
            self.hidden_rows.insert(r);
        }
    }

    /// Unhide a range of rows starting at `start` for `count` rows.
    ///
    /// Rows that are not hidden are silently ignored.
    pub fn unhide_rows(&mut self, start: u32, count: u32) {
        for r in start..start + count {
            self.hidden_rows.remove(&r);
        }
    }

    /// Hide a range of columns starting at `start` for `count` columns.
    ///
    /// Already-hidden columns in the range are silently ignored.
    pub fn hide_cols(&mut self, start: u32, count: u32) {
        for c in start..start + count {
            self.hidden_cols.insert(c);
        }
    }

    /// Unhide a range of columns starting at `start` for `count` columns.
    ///
    /// Columns that are not hidden are silently ignored.
    pub fn unhide_cols(&mut self, start: u32, count: u32) {
        for c in start..start + count {
            self.hidden_cols.remove(&c);
        }
    }

    /// Check if a row is hidden.
    pub fn is_row_hidden(&self, row: u32) -> bool {
        self.hidden_rows.contains(&row)
    }

    /// Check if a column is hidden.
    pub fn is_col_hidden(&self, col: u32) -> bool {
        self.hidden_cols.contains(&col)
    }

    /// Return all visible row indices in the range `[start, end]` (inclusive).
    pub fn visible_rows(&self, start: u32, end: u32) -> Vec<u32> {
        (start..=end)
            .filter(|r| !self.hidden_rows.contains(r))
            .collect()
    }

    /// Return all visible column indices in the range `[start, end]` (inclusive).
    pub fn visible_cols(&self, start: u32, end: u32) -> Vec<u32> {
        (start..=end)
            .filter(|c| !self.hidden_cols.contains(c))
            .collect()
    }

    // ----- Insert / Delete Rows and Columns -----

    /// Insert `count` rows at the given position, shifting existing rows down.
    ///
    /// All cells at `row >= at_row` are shifted down by `count`.
    pub fn insert_rows(&mut self, at_row: u32, count: u32) {
        // Collect all cells that need to move (in reverse order to avoid collisions)
        let mut keys: Vec<(u32, u32)> = self
            .cells
            .keys()
            .filter(|(r, _)| *r >= at_row)
            .copied()
            .collect();
        keys.sort_by(|a, b| b.0.cmp(&a.0)); // sort descending by row

        for key in keys {
            if let Some(cell) = self.cells.remove(&key) {
                self.cells.insert((key.0 + count, key.1), cell);
            }
        }

        // Shift merged regions
        for region in &mut self.merged_regions {
            if region.start_row >= at_row {
                region.start_row += count;
                region.end_row += count;
            } else if region.end_row >= at_row {
                region.end_row += count;
            }
        }

        // Shift row heights
        let mut new_heights = HashMap::new();
        for (row, height) in self.row_heights.drain() {
            if row >= at_row {
                new_heights.insert(row + count, height);
            } else {
                new_heights.insert(row, height);
            }
        }
        self.row_heights = new_heights;

        // Shift hidden rows
        let new_hidden: HashSet<u32> = self
            .hidden_rows
            .iter()
            .map(|&r| if r >= at_row { r + count } else { r })
            .collect();
        self.hidden_rows = new_hidden;
    }

    /// Delete `count` rows starting at the given position, shifting rows up.
    ///
    /// Cells in the deleted rows are removed. Cells below are shifted up.
    pub fn delete_rows(&mut self, at_row: u32, count: u32) {
        let end_row = at_row + count;

        // Remove cells in the deleted rows
        self.cells.retain(|&(r, _), _| r < at_row || r >= end_row);

        // Shift remaining cells up
        let mut keys: Vec<(u32, u32)> = self
            .cells
            .keys()
            .filter(|(r, _)| *r >= end_row)
            .copied()
            .collect();
        keys.sort_by_key(|k| k.0); // sort ascending by row

        for key in keys {
            if let Some(cell) = self.cells.remove(&key) {
                self.cells.insert((key.0 - count, key.1), cell);
            }
        }

        // Update merged regions
        self.merged_regions.retain(|r| {
            !(r.start_row >= at_row && r.end_row < end_row)
        });
        for region in &mut self.merged_regions {
            if region.start_row >= end_row {
                region.start_row -= count;
                region.end_row -= count;
            } else if region.end_row >= end_row {
                region.end_row -= count;
            }
        }

        // Shift row heights
        let mut new_heights = HashMap::new();
        for (row, height) in self.row_heights.drain() {
            if row < at_row {
                new_heights.insert(row, height);
            } else if row >= end_row {
                new_heights.insert(row - count, height);
            }
            // Rows in the deleted range are dropped
        }
        self.row_heights = new_heights;

        // Shift hidden rows (remove deleted, shift remaining)
        let new_hidden: HashSet<u32> = self
            .hidden_rows
            .iter()
            .filter_map(|&r| {
                if r < at_row {
                    Some(r)
                } else if r >= end_row {
                    Some(r - count)
                } else {
                    None // deleted range
                }
            })
            .collect();
        self.hidden_rows = new_hidden;
    }

    /// Insert `count` columns at the given position, shifting existing columns right.
    pub fn insert_cols(&mut self, at_col: u32, count: u32) {
        let mut keys: Vec<(u32, u32)> = self
            .cells
            .keys()
            .filter(|(_, c)| *c >= at_col)
            .copied()
            .collect();
        keys.sort_by(|a, b| b.1.cmp(&a.1)); // sort descending by col

        for key in keys {
            if let Some(cell) = self.cells.remove(&key) {
                self.cells.insert((key.0, key.1 + count), cell);
            }
        }

        // Shift merged regions
        for region in &mut self.merged_regions {
            if region.start_col >= at_col {
                region.start_col += count;
                region.end_col += count;
            } else if region.end_col >= at_col {
                region.end_col += count;
            }
        }

        // Shift col widths
        let mut new_widths = HashMap::new();
        for (col, width) in self.col_widths.drain() {
            if col >= at_col {
                new_widths.insert(col + count, width);
            } else {
                new_widths.insert(col, width);
            }
        }
        self.col_widths = new_widths;

        // Shift hidden columns
        let new_hidden: HashSet<u32> = self
            .hidden_cols
            .iter()
            .map(|&c| if c >= at_col { c + count } else { c })
            .collect();
        self.hidden_cols = new_hidden;
    }

    // ----- Sheet Protection -----

    /// Protect the sheet, optionally with a password.
    ///
    /// The password (if provided) is stored as a simple hash. The default
    /// protection allows cell selection but disallows editing, sorting, and
    /// filtering.
    pub fn protect(&mut self, password: Option<&str>) {
        let password_hash = password.map(simple_hash);
        self.protection = Some(SheetProtection {
            password_hash,
            ..SheetProtection::default()
        });
    }

    /// Unprotect the sheet. If the sheet was protected with a password, the
    /// correct password must be supplied.
    ///
    /// Returns `Ok(())` on success. Returns `Err(IncorrectPassword)` if the
    /// password does not match.
    pub fn unprotect(&mut self, password: Option<&str>) -> Result<()> {
        match &self.protection {
            None => Ok(()), // already unprotected
            Some(prot) => {
                if let Some(stored) = &prot.password_hash {
                    let given = password.map(simple_hash).unwrap_or_default();
                    if given != *stored {
                        return Err(LatticeError::IncorrectPassword);
                    }
                }
                self.protection = None;
                Ok(())
            }
        }
    }

    /// Returns `true` if the sheet is currently protected.
    pub fn is_protected(&self) -> bool {
        self.protection
            .as_ref()
            .map_or(false, |p| p.is_protected)
    }

    // ----- Protected Ranges -----

    /// Add a protected range to the sheet.
    pub fn add_protected_range(&mut self, range: ProtectedRange) {
        self.protected_ranges.push(range);
    }

    /// Remove a protected range by index. Returns `Ok(())` on success or
    /// an error if the index is out of bounds.
    pub fn remove_protected_range(&mut self, index: usize) -> Result<()> {
        if index >= self.protected_ranges.len() {
            return Err(LatticeError::InvalidRange(format!(
                "protected range index {index} out of bounds"
            )));
        }
        self.protected_ranges.remove(index);
        Ok(())
    }

    /// Check whether a cell at `(row, col)` falls inside any protected range.
    pub fn is_cell_protected(&self, row: u32, col: u32) -> bool {
        self.protected_ranges.iter().any(|pr| {
            row >= pr.start_row
                && row <= pr.end_row
                && col >= pr.start_col
                && col <= pr.end_col
        })
    }

    /// Return a reference to the list of protected ranges.
    pub fn protected_ranges(&self) -> &[ProtectedRange] {
        &self.protected_ranges
    }

    // ----- Tab Color -----

    /// Set the tab color for the sheet (CSS hex string, e.g. `"#FF0000"`).
    /// Pass `None` to clear the color.
    pub fn set_tab_color(&mut self, color: Option<String>) {
        self.tab_color = color;
    }

    // ----- Insert / Delete -----

    /// Delete `count` columns starting at the given position, shifting columns left.
    pub fn delete_cols(&mut self, at_col: u32, count: u32) {
        let end_col = at_col + count;

        // Remove cells in the deleted columns
        self.cells.retain(|&(_, c), _| c < at_col || c >= end_col);

        // Shift remaining cells left
        let mut keys: Vec<(u32, u32)> = self
            .cells
            .keys()
            .filter(|(_, c)| *c >= end_col)
            .copied()
            .collect();
        keys.sort_by_key(|k| k.1);

        for key in keys {
            if let Some(cell) = self.cells.remove(&key) {
                self.cells.insert((key.0, key.1 - count), cell);
            }
        }

        // Update merged regions
        self.merged_regions.retain(|r| {
            !(r.start_col >= at_col && r.end_col < end_col)
        });
        for region in &mut self.merged_regions {
            if region.start_col >= end_col {
                region.start_col -= count;
                region.end_col -= count;
            } else if region.end_col >= end_col {
                region.end_col -= count;
            }
        }

        // Shift col widths
        let mut new_widths = HashMap::new();
        for (col, width) in self.col_widths.drain() {
            if col < at_col {
                new_widths.insert(col, width);
            } else if col >= end_col {
                new_widths.insert(col - count, width);
            }
        }
        self.col_widths = new_widths;

        // Shift hidden columns (remove deleted, shift remaining)
        let new_hidden: HashSet<u32> = self
            .hidden_cols
            .iter()
            .filter_map(|&c| {
                if c < at_col {
                    Some(c)
                } else if c >= end_col {
                    Some(c - count)
                } else {
                    None // deleted range
                }
            })
            .collect();
        self.hidden_cols = new_hidden;
    }
}

/// Compute a simple deterministic hash of a password string.
///
/// This uses Rust's built-in `DefaultHasher` (SipHash) which is *not*
/// cryptographic, but is sufficient for the spreadsheet-style password
/// protection model (compatible with how Excel/Google Sheets handle sheet
/// passwords — they are a deterrent, not a security boundary).
fn simple_hash(password: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Check if two rectangular regions overlap.
fn regions_overlap(
    r1_sr: u32,
    r1_sc: u32,
    r1_er: u32,
    r1_ec: u32,
    r2_sr: u32,
    r2_sc: u32,
    r2_er: u32,
    r2_ec: u32,
) -> bool {
    r1_sr <= r2_er && r1_er >= r2_sr && r1_sc <= r2_ec && r1_ec >= r2_sc
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

    // --- Comments ---

    #[test]
    fn test_set_get_remove_comment() {
        let mut sheet = Sheet::new("T");
        sheet.set_comment(0, 0, "This is a note");
        assert_eq!(sheet.get_comment(0, 0), Some("This is a note"));
        sheet.remove_comment(0, 0);
        assert_eq!(sheet.get_comment(0, 0), None);
    }

    #[test]
    fn test_comment_on_new_cell() {
        let mut sheet = Sheet::new("T");
        // Setting a comment on a cell that doesn't exist should create it
        sheet.set_comment(5, 5, "hello");
        assert!(sheet.get_cell(5, 5).is_some());
        assert_eq!(sheet.get_comment(5, 5), Some("hello"));
    }

    // --- Merge ---

    #[test]
    fn test_merge_cells() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("main".into()));
        sheet.set_value(0, 1, CellValue::Text("will be cleared".into()));
        sheet.set_value(1, 0, CellValue::Text("will be cleared".into()));
        sheet.set_value(1, 1, CellValue::Text("will be cleared".into()));

        sheet.merge_cells(0, 0, 1, 1).unwrap();

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Text("main".into())
        );
        assert!(sheet.get_cell(0, 1).is_none()); // cleared
        assert!(sheet.get_cell(1, 0).is_none()); // cleared
        assert_eq!(sheet.merged_regions().len(), 1);
    }

    #[test]
    fn test_merge_overlap_rejected() {
        let mut sheet = Sheet::new("T");
        sheet.merge_cells(0, 0, 1, 1).unwrap();
        let err = sheet.merge_cells(1, 1, 2, 2);
        assert!(err.is_err());
    }

    #[test]
    fn test_unmerge_cell() {
        let mut sheet = Sheet::new("T");
        sheet.merge_cells(0, 0, 1, 1).unwrap();
        assert_eq!(sheet.merged_regions().len(), 1);
        let unmerged = sheet.unmerge_cell(0, 0).unwrap();
        assert!(unmerged);
        assert_eq!(sheet.merged_regions().len(), 0);
    }

    #[test]
    fn test_unmerge_not_merged() {
        let mut sheet = Sheet::new("T");
        let unmerged = sheet.unmerge_cell(0, 0).unwrap();
        assert!(!unmerged);
    }

    #[test]
    fn test_get_merged_region() {
        let mut sheet = Sheet::new("T");
        sheet.merge_cells(2, 2, 4, 4).unwrap();
        assert!(sheet.get_merged_region(3, 3).is_some());
        assert!(sheet.get_merged_region(0, 0).is_none());
    }

    // --- Insert/Delete Rows ---

    #[test]
    fn test_insert_rows() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(2, 0, CellValue::Number(3.0));

        sheet.insert_rows(1, 2); // Insert 2 rows at row 1

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Number(1.0)
        );
        assert!(sheet.get_cell(1, 0).is_none()); // new empty row
        assert!(sheet.get_cell(2, 0).is_none()); // new empty row
        assert_eq!(
            sheet.get_cell(3, 0).unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            sheet.get_cell(4, 0).unwrap().value,
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_delete_rows() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(2, 0, CellValue::Number(3.0));
        sheet.set_value(3, 0, CellValue::Number(4.0));

        sheet.delete_rows(1, 2); // Delete rows 1-2

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            sheet.get_cell(1, 0).unwrap().value,
            CellValue::Number(4.0)
        );
        assert!(sheet.get_cell(2, 0).is_none());
    }

    #[test]
    fn test_insert_cols() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(0, 2, CellValue::Number(3.0));

        sheet.insert_cols(1, 2); // Insert 2 columns at col 1

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Number(1.0)
        );
        assert!(sheet.get_cell(0, 1).is_none()); // new empty col
        assert!(sheet.get_cell(0, 2).is_none()); // new empty col
        assert_eq!(
            sheet.get_cell(0, 3).unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            sheet.get_cell(0, 4).unwrap().value,
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_delete_cols() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(0, 2, CellValue::Number(3.0));
        sheet.set_value(0, 3, CellValue::Number(4.0));

        sheet.delete_cols(1, 2); // Delete cols 1-2

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            sheet.get_cell(0, 1).unwrap().value,
            CellValue::Number(4.0)
        );
        assert!(sheet.get_cell(0, 2).is_none());
    }

    #[test]
    fn test_insert_rows_shifts_merged_regions() {
        let mut sheet = Sheet::new("T");
        sheet.merge_cells(2, 0, 3, 1).unwrap();
        sheet.insert_rows(1, 2);

        let region = &sheet.merged_regions()[0];
        assert_eq!(region.start_row, 4);
        assert_eq!(region.end_row, 5);
    }

    #[test]
    fn test_delete_rows_removes_contained_merged_regions() {
        let mut sheet = Sheet::new("T");
        sheet.merge_cells(1, 0, 2, 1).unwrap();
        sheet.delete_rows(1, 2);

        assert!(sheet.merged_regions().is_empty());
    }

    #[test]
    fn test_insert_rows_shifts_row_heights() {
        let mut sheet = Sheet::new("T");
        sheet.row_heights.insert(2, 30.0);
        sheet.insert_rows(1, 2);
        assert_eq!(sheet.row_heights.get(&4), Some(&30.0));
        assert!(sheet.row_heights.get(&2).is_none());
    }

    // --- Hidden Rows / Columns ---

    #[test]
    fn test_hide_rows() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(2, 3); // Hide rows 2, 3, 4
        assert!(sheet.is_row_hidden(2));
        assert!(sheet.is_row_hidden(3));
        assert!(sheet.is_row_hidden(4));
        assert!(!sheet.is_row_hidden(1));
        assert!(!sheet.is_row_hidden(5));
    }

    #[test]
    fn test_unhide_rows() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(0, 5); // Hide rows 0..4
        sheet.unhide_rows(2, 2); // Unhide rows 2, 3
        assert!(sheet.is_row_hidden(0));
        assert!(sheet.is_row_hidden(1));
        assert!(!sheet.is_row_hidden(2));
        assert!(!sheet.is_row_hidden(3));
        assert!(sheet.is_row_hidden(4));
    }

    #[test]
    fn test_hide_cols() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(1, 2); // Hide cols 1, 2
        assert!(!sheet.is_col_hidden(0));
        assert!(sheet.is_col_hidden(1));
        assert!(sheet.is_col_hidden(2));
        assert!(!sheet.is_col_hidden(3));
    }

    #[test]
    fn test_unhide_cols() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(0, 4);
        sheet.unhide_cols(1, 2); // Unhide cols 1, 2
        assert!(sheet.is_col_hidden(0));
        assert!(!sheet.is_col_hidden(1));
        assert!(!sheet.is_col_hidden(2));
        assert!(sheet.is_col_hidden(3));
    }

    #[test]
    fn test_visible_rows() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(2, 2); // Hide rows 2, 3
        let visible = sheet.visible_rows(0, 5);
        assert_eq!(visible, vec![0, 1, 4, 5]);
    }

    #[test]
    fn test_visible_cols() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(1, 1); // Hide col 1
        let visible = sheet.visible_cols(0, 3);
        assert_eq!(visible, vec![0, 2, 3]);
    }

    #[test]
    fn test_visible_rows_none_hidden() {
        let sheet = Sheet::new("T");
        let visible = sheet.visible_rows(0, 4);
        assert_eq!(visible, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_visible_cols_all_hidden() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(0, 3);
        let visible = sheet.visible_cols(0, 2);
        assert!(visible.is_empty());
    }

    #[test]
    fn test_hide_rows_idempotent() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(1, 2);
        sheet.hide_rows(1, 2); // hide again — no error
        assert!(sheet.is_row_hidden(1));
        assert!(sheet.is_row_hidden(2));
    }

    #[test]
    fn test_unhide_rows_no_op() {
        let mut sheet = Sheet::new("T");
        // Unhiding rows that are not hidden should be fine
        sheet.unhide_rows(0, 5);
        assert!(!sheet.is_row_hidden(0));
    }

    #[test]
    fn test_insert_rows_shifts_hidden_rows() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(3, 2); // Hide rows 3, 4
        sheet.insert_rows(2, 2); // Insert 2 rows at row 2
        // Hidden rows 3,4 should shift to 5,6
        assert!(!sheet.is_row_hidden(3));
        assert!(!sheet.is_row_hidden(4));
        assert!(sheet.is_row_hidden(5));
        assert!(sheet.is_row_hidden(6));
    }

    #[test]
    fn test_delete_rows_shifts_hidden_rows() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(4, 1); // Hide row 4
        sheet.delete_rows(1, 2); // Delete rows 1, 2
        // Row 4 should shift to row 2
        assert!(sheet.is_row_hidden(2));
        assert!(!sheet.is_row_hidden(4));
    }

    #[test]
    fn test_delete_rows_removes_hidden_in_deleted_range() {
        let mut sheet = Sheet::new("T");
        sheet.hide_rows(2, 2); // Hide rows 2, 3
        sheet.delete_rows(1, 3); // Delete rows 1, 2, 3
        // Hidden rows 2, 3 were in the deleted range — gone
        assert!(!sheet.is_row_hidden(2));
        assert!(!sheet.is_row_hidden(3));
    }

    #[test]
    fn test_insert_cols_shifts_hidden_cols() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(2, 1); // Hide col 2
        sheet.insert_cols(1, 3); // Insert 3 cols at col 1
        // Col 2 should shift to col 5
        assert!(!sheet.is_col_hidden(2));
        assert!(sheet.is_col_hidden(5));
    }

    #[test]
    fn test_delete_cols_shifts_hidden_cols() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(5, 1); // Hide col 5
        sheet.delete_cols(2, 2); // Delete cols 2, 3
        // Col 5 should shift to col 3
        assert!(sheet.is_col_hidden(3));
        assert!(!sheet.is_col_hidden(5));
    }

    #[test]
    fn test_delete_cols_removes_hidden_in_deleted_range() {
        let mut sheet = Sheet::new("T");
        sheet.hide_cols(1, 2); // Hide cols 1, 2
        sheet.delete_cols(0, 3); // Delete cols 0, 1, 2
        assert!(!sheet.is_col_hidden(0));
        assert!(!sheet.is_col_hidden(1));
        assert!(!sheet.is_col_hidden(2));
    }

    // --- Sheet Protection ---

    #[test]
    fn test_protect_no_password() {
        let mut sheet = Sheet::new("T");
        assert!(!sheet.is_protected());
        sheet.protect(None);
        assert!(sheet.is_protected());
    }

    #[test]
    fn test_unprotect_no_password() {
        let mut sheet = Sheet::new("T");
        sheet.protect(None);
        assert!(sheet.is_protected());
        sheet.unprotect(None).unwrap();
        assert!(!sheet.is_protected());
    }

    #[test]
    fn test_protect_with_password() {
        let mut sheet = Sheet::new("T");
        sheet.protect(Some("secret"));
        assert!(sheet.is_protected());
        // Wrong password
        let err = sheet.unprotect(Some("wrong"));
        assert!(err.is_err());
        assert!(sheet.is_protected());
        // Correct password
        sheet.unprotect(Some("secret")).unwrap();
        assert!(!sheet.is_protected());
    }

    #[test]
    fn test_unprotect_already_unprotected() {
        let mut sheet = Sheet::new("T");
        // Should be a no-op, not an error
        sheet.unprotect(None).unwrap();
        assert!(!sheet.is_protected());
    }

    #[test]
    fn test_protection_defaults() {
        let mut sheet = Sheet::new("T");
        sheet.protect(None);
        let prot = sheet.protection.as_ref().unwrap();
        assert!(prot.allow_select);
        assert!(!prot.allow_sort);
        assert!(!prot.allow_filter);
    }

    // --- Protected Ranges ---

    #[test]
    fn test_add_protected_range() {
        let mut sheet = Sheet::new("T");
        sheet.add_protected_range(ProtectedRange {
            start_row: 0,
            start_col: 0,
            end_row: 5,
            end_col: 3,
            description: Some("Header".into()),
        });
        assert_eq!(sheet.protected_ranges().len(), 1);
        assert!(sheet.is_cell_protected(0, 0));
        assert!(sheet.is_cell_protected(3, 2));
        assert!(!sheet.is_cell_protected(6, 0));
    }

    #[test]
    fn test_remove_protected_range() {
        let mut sheet = Sheet::new("T");
        sheet.add_protected_range(ProtectedRange {
            start_row: 0,
            start_col: 0,
            end_row: 1,
            end_col: 1,
            description: None,
        });
        sheet.add_protected_range(ProtectedRange {
            start_row: 5,
            start_col: 5,
            end_row: 10,
            end_col: 10,
            description: None,
        });
        assert_eq!(sheet.protected_ranges().len(), 2);
        sheet.remove_protected_range(0).unwrap();
        assert_eq!(sheet.protected_ranges().len(), 1);
        // The remaining range should be the second one
        assert!(!sheet.is_cell_protected(0, 0));
        assert!(sheet.is_cell_protected(7, 7));
    }

    #[test]
    fn test_remove_protected_range_out_of_bounds() {
        let mut sheet = Sheet::new("T");
        let err = sheet.remove_protected_range(0);
        assert!(err.is_err());
    }

    #[test]
    fn test_is_cell_protected_no_ranges() {
        let sheet = Sheet::new("T");
        assert!(!sheet.is_cell_protected(0, 0));
    }

    // --- Tab Color ---

    #[test]
    fn test_set_tab_color() {
        let mut sheet = Sheet::new("T");
        assert!(sheet.tab_color.is_none());
        sheet.set_tab_color(Some("#FF0000".into()));
        assert_eq!(sheet.tab_color.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn test_clear_tab_color() {
        let mut sheet = Sheet::new("T");
        sheet.set_tab_color(Some("#00FF00".into()));
        sheet.set_tab_color(None);
        assert!(sheet.tab_color.is_none());
    }
}
