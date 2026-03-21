//! Sort operations for spreadsheet ranges.
//!
//! Supports multi-key stable sorting of rows within a rectangular range,
//! with ascending or descending order per key.

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, CellValue};
use crate::error::Result;
use crate::sheet::Sheet;

/// Whether to sort ascending or descending.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    /// Smallest to largest / A to Z.
    Ascending,
    /// Largest to smallest / Z to A.
    Descending,
}

/// A single sort key (column index + direction).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortKey {
    /// 0-based column index to sort by.
    pub col: u32,
    /// Sort direction.
    pub direction: SortDirection,
}

/// Sort a range of rows in a sheet by the given keys.
///
/// Sorts rows `start_row..=end_row` (inclusive) considering columns
/// `start_col..=end_col`. The sort is stable, preserving the relative
/// order of rows that compare equal across all keys.
///
/// # Errors
///
/// Returns an error if the range is invalid.
pub fn sort_range(
    sheet: &mut Sheet,
    start_row: u32,
    end_row: u32,
    start_col: u32,
    end_col: u32,
    keys: &[SortKey],
) -> Result<()> {
    if start_row > end_row || keys.is_empty() {
        return Ok(());
    }

    let num_rows = (end_row - start_row + 1) as usize;
    let num_cols = (end_col - start_col + 1) as usize;

    // Extract rows into a Vec of (original_row_index, row_data).
    let mut rows: Vec<(u32, Vec<Option<Cell>>)> = Vec::with_capacity(num_rows);
    for r in start_row..=end_row {
        let mut row_data = Vec::with_capacity(num_cols);
        for c in start_col..=end_col {
            row_data.push(sheet.get_cell(r, c).cloned());
        }
        rows.push((r, row_data));
    }

    // Stable sort by the keys.
    rows.sort_by(|a, b| {
        for key in keys {
            let col_offset = key.col.saturating_sub(start_col) as usize;
            let val_a = a
                .1
                .get(col_offset)
                .and_then(|c| c.as_ref())
                .map(|c| &c.value)
                .unwrap_or(&CellValue::Empty);
            let val_b = b
                .1
                .get(col_offset)
                .and_then(|c| c.as_ref())
                .map(|c| &c.value)
                .unwrap_or(&CellValue::Empty);

            let cmp = compare_cell_values(val_a, val_b);
            let cmp = match key.direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            };
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });

    // Write sorted rows back to the sheet.
    for (new_row_idx, (_, row_data)) in rows.iter().enumerate() {
        let target_row = start_row + new_row_idx as u32;
        for (col_offset, cell_opt) in row_data.iter().enumerate() {
            let target_col = start_col + col_offset as u32;
            match cell_opt {
                Some(cell) => sheet.set_cell(target_row, target_col, cell.clone()),
                None => sheet.clear_cell(target_row, target_col),
            }
        }
    }

    Ok(())
}

/// Compare two CellValues for sorting purposes.
///
/// Ordering: Empty < Number < Text < Boolean < Error
fn compare_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (CellValue::Empty, CellValue::Empty) => Ordering::Equal,
        (CellValue::Empty, _) => Ordering::Less,
        (_, CellValue::Empty) => Ordering::Greater,

        (CellValue::Number(na), CellValue::Number(nb)) => {
            na.partial_cmp(nb).unwrap_or(Ordering::Equal)
        }
        (CellValue::Number(_), _) => Ordering::Less,
        (_, CellValue::Number(_)) => Ordering::Greater,

        (CellValue::Text(sa), CellValue::Text(sb)) => {
            sa.to_lowercase().cmp(&sb.to_lowercase())
        }
        (CellValue::Text(_), _) => Ordering::Less,
        (_, CellValue::Text(_)) => Ordering::Greater,

        (CellValue::Boolean(ba), CellValue::Boolean(bb)) => ba.cmp(bb),
        (CellValue::Boolean(_), _) => Ordering::Less,
        (_, CellValue::Boolean(_)) => Ordering::Greater,

        (CellValue::Date(da), CellValue::Date(db)) => da.cmp(db),
        (CellValue::Date(_), _) => Ordering::Less,
        (_, CellValue::Date(_)) => Ordering::Greater,

        (CellValue::Error(_), CellValue::Error(_)) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_ascending() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(3.0));
        sheet.set_value(1, 0, CellValue::Number(1.0));
        sheet.set_value(2, 0, CellValue::Number(2.0));

        sort_range(
            &mut sheet,
            0,
            2,
            0,
            0,
            &[SortKey {
                col: 0,
                direction: SortDirection::Ascending,
            }],
        )
        .unwrap();

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            sheet.get_cell(1, 0).unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            sheet.get_cell(2, 0).unwrap().value,
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_sort_descending() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(3.0));
        sheet.set_value(2, 0, CellValue::Number(2.0));

        sort_range(
            &mut sheet,
            0,
            2,
            0,
            0,
            &[SortKey {
                col: 0,
                direction: SortDirection::Descending,
            }],
        )
        .unwrap();

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Number(3.0)
        );
        assert_eq!(
            sheet.get_cell(1, 0).unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            sheet.get_cell(2, 0).unwrap().value,
            CellValue::Number(1.0)
        );
    }

    #[test]
    fn test_sort_multi_column() {
        let mut sheet = Sheet::new("T");
        // Col A: category, Col B: value
        sheet.set_value(0, 0, CellValue::Text("B".into()));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(1, 0, CellValue::Text("A".into()));
        sheet.set_value(1, 1, CellValue::Number(3.0));
        sheet.set_value(2, 0, CellValue::Text("A".into()));
        sheet.set_value(2, 1, CellValue::Number(1.0));

        sort_range(
            &mut sheet,
            0,
            2,
            0,
            1,
            &[
                SortKey {
                    col: 0,
                    direction: SortDirection::Ascending,
                },
                SortKey {
                    col: 1,
                    direction: SortDirection::Ascending,
                },
            ],
        )
        .unwrap();

        // Should be: A/1, A/3, B/2
        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Text("A".into())
        );
        assert_eq!(
            sheet.get_cell(0, 1).unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            sheet.get_cell(1, 0).unwrap().value,
            CellValue::Text("A".into())
        );
        assert_eq!(
            sheet.get_cell(1, 1).unwrap().value,
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_sort_with_empty_cells() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(3.0));
        // row 1 is empty
        sheet.set_value(2, 0, CellValue::Number(1.0));

        sort_range(
            &mut sheet,
            0,
            2,
            0,
            0,
            &[SortKey {
                col: 0,
                direction: SortDirection::Ascending,
            }],
        )
        .unwrap();

        // Empty comes first
        assert!(sheet.get_cell(0, 0).is_none() || sheet.get_cell(0, 0).unwrap().value == CellValue::Empty);
    }

    #[test]
    fn test_sort_strings() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("cherry".into()));
        sheet.set_value(1, 0, CellValue::Text("apple".into()));
        sheet.set_value(2, 0, CellValue::Text("banana".into()));

        sort_range(
            &mut sheet,
            0,
            2,
            0,
            0,
            &[SortKey {
                col: 0,
                direction: SortDirection::Ascending,
            }],
        )
        .unwrap();

        assert_eq!(
            sheet.get_cell(0, 0).unwrap().value,
            CellValue::Text("apple".into())
        );
        assert_eq!(
            sheet.get_cell(1, 0).unwrap().value,
            CellValue::Text("banana".into())
        );
        assert_eq!(
            sheet.get_cell(2, 0).unwrap().value,
            CellValue::Text("cherry".into())
        );
    }
}
