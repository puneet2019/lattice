//! Clipboard operations: copy, cut, paste, and paste-special.
//!
//! Supports copying cell ranges (values + formulas + formatting), pasting
//! with automatic formula reference adjustment, and paste-special modes
//! (values only, formulas only, formatting only).

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::error::Result;
use crate::sheet::Sheet;

/// Content stored in the internal clipboard after a copy or cut operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardContent {
    /// 2-D grid of cells that were copied (rows x cols).
    pub cells: Vec<Vec<Option<Cell>>>,
    /// Whether this was a *cut* (source cells should be cleared on paste).
    pub is_cut: bool,
    /// Original top-left position of the copied range (row, col).
    pub source_origin: (u32, u32),
}

impl ClipboardContent {
    /// Create a new empty clipboard entry.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            is_cut: false,
            source_origin: (0, 0),
        }
    }

    /// Return the dimensions of the clipboard content (rows, cols).
    pub fn dimensions(&self) -> (usize, usize) {
        if self.cells.is_empty() {
            return (0, 0);
        }
        let rows = self.cells.len();
        let cols = self.cells[0].len();
        (rows, cols)
    }
}

impl Default for ClipboardContent {
    fn default() -> Self {
        Self::new()
    }
}

/// What to paste.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PasteMode {
    /// Paste everything (values, formulas, formatting).
    All,
    /// Paste values only (formulas are evaluated and pasted as values).
    ValuesOnly,
    /// Paste formulas only (no formatting).
    FormulasOnly,
    /// Paste formatting only (no values or formulas).
    FormattingOnly,
    /// Paste with rows and columns transposed (swapped).
    ///
    /// A cell at source position `(row, col)` is pasted to
    /// `(dest_start_row + col, dest_start_col + row)`.
    Transposed,
}

/// Copy a rectangular range of cells from a sheet into a [`ClipboardContent`].
///
/// The range is from `(start_row, start_col)` to `(end_row, end_col)` inclusive.
pub fn copy_range(
    sheet: &Sheet,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    is_cut: bool,
) -> ClipboardContent {
    let mut cells = Vec::new();

    for row in start_row..=end_row {
        let mut row_data = Vec::new();
        for col in start_col..=end_col {
            row_data.push(sheet.get_cell(row, col).cloned());
        }
        cells.push(row_data);
    }

    ClipboardContent {
        cells,
        is_cut,
        source_origin: (start_row, start_col),
    }
}

/// Paste clipboard content into a sheet at the given position.
///
/// Formula references are adjusted by the row/column offset from the
/// original position (for relative references).
///
/// For [`PasteMode::Transposed`], rows and columns are swapped: a cell at
/// source position `(r, c)` is pasted to `(dest_row + c, dest_col + r)`.
pub fn paste(
    sheet: &mut Sheet,
    clipboard: &ClipboardContent,
    dest_row: u32,
    dest_col: u32,
    mode: &PasteMode,
) -> Result<()> {
    // Transposed paste swaps row/col indices.
    if *mode == PasteMode::Transposed {
        return paste_transposed(sheet, clipboard, dest_row, dest_col);
    }

    let row_offset = dest_row as i32 - clipboard.source_origin.0 as i32;
    let col_offset = dest_col as i32 - clipboard.source_origin.1 as i32;

    for (r_idx, row_data) in clipboard.cells.iter().enumerate() {
        for (c_idx, cell_opt) in row_data.iter().enumerate() {
            let target_row = dest_row + r_idx as u32;
            let target_col = dest_col + c_idx as u32;

            match (cell_opt, mode) {
                (Some(cell), PasteMode::All) => {
                    let mut new_cell = cell.clone();
                    if let Some(ref formula) = cell.formula {
                        new_cell.formula =
                            Some(adjust_formula_references(formula, row_offset, col_offset));
                    }
                    sheet.set_cell(target_row, target_col, new_cell);
                }
                (Some(cell), PasteMode::ValuesOnly) => {
                    let new_cell = Cell {
                        value: cell.value.clone(),
                        format: cell.format.clone(),
                        ..Default::default()
                    };
                    // No formula
                    sheet.set_cell(target_row, target_col, new_cell);
                }
                (Some(cell), PasteMode::FormulasOnly) => {
                    let new_cell = Cell {
                        value: cell.value.clone(),
                        formula: cell
                            .formula
                            .as_ref()
                            .map(|f| adjust_formula_references(f, row_offset, col_offset)),
                        ..Default::default()
                    };
                    sheet.set_cell(target_row, target_col, new_cell);
                }
                (Some(cell), PasteMode::FormattingOnly) => {
                    // Apply formatting to existing cell or create one
                    if let Some(existing) = sheet.get_cell(target_row, target_col) {
                        let mut updated = existing.clone();
                        updated.format = cell.format.clone();
                        updated.style_id = cell.style_id;
                        sheet.set_cell(target_row, target_col, updated);
                    } else {
                        let new_cell = Cell {
                            format: cell.format.clone(),
                            style_id: cell.style_id,
                            ..Default::default()
                        };
                        sheet.set_cell(target_row, target_col, new_cell);
                    }
                }
                (None, PasteMode::All | PasteMode::ValuesOnly | PasteMode::FormulasOnly) => {
                    sheet.clear_cell(target_row, target_col);
                }
                (None, PasteMode::FormattingOnly) => {
                    // Don't clear cells when pasting formatting only
                }
                // Transposed is handled above via early return.
                (_, PasteMode::Transposed) => unreachable!(),
            }
        }
    }

    Ok(())
}

/// Paste clipboard content with rows and columns transposed.
///
/// Source cell at `(r_idx, c_idx)` is placed at `(dest_row + c_idx, dest_col + r_idx)`.
/// Formulas and formatting are preserved. Formula references are adjusted
/// relative to the transposed destination.
fn paste_transposed(
    sheet: &mut Sheet,
    clipboard: &ClipboardContent,
    dest_row: u32,
    dest_col: u32,
) -> Result<()> {
    for (r_idx, row_data) in clipboard.cells.iter().enumerate() {
        for (c_idx, cell_opt) in row_data.iter().enumerate() {
            // Swap: source row index becomes dest column offset,
            //        source col index becomes dest row offset.
            let target_row = dest_row + c_idx as u32;
            let target_col = dest_col + r_idx as u32;

            match cell_opt {
                Some(cell) => {
                    let mut new_cell = cell.clone();
                    if let Some(ref formula) = cell.formula {
                        let row_offset = target_row as i32 - clipboard.source_origin.0 as i32;
                        let col_offset = target_col as i32 - clipboard.source_origin.1 as i32;
                        new_cell.formula =
                            Some(adjust_formula_references(formula, row_offset, col_offset));
                    }
                    sheet.set_cell(target_row, target_col, new_cell);
                }
                None => {
                    sheet.clear_cell(target_row, target_col);
                }
            }
        }
    }

    Ok(())
}

/// Adjust cell references in a formula string by the given row and column offsets.
///
/// This handles basic A1-style references (including absolute references with `$`).
/// Absolute row/column references (prefixed with `$`) are NOT adjusted.
///
/// For example, with row_offset=1, col_offset=0:
/// - `A1` becomes `A2`
/// - `$A1` becomes `$A2` (column is absolute, row is relative)
/// - `A$1` becomes `A$1` (row is absolute)
/// - `$A$1` becomes `$A$1` (both are absolute)
pub fn adjust_formula_references(formula: &str, row_offset: i32, col_offset: i32) -> String {
    let chars: Vec<char> = formula.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        // Check if we're inside a string literal
        if chars[i] == '"' {
            result.push(chars[i]);
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                result.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Try to parse a cell reference at position i
        if let Some((ref_str, new_ref, consumed)) =
            try_parse_and_adjust_ref(&chars, i, row_offset, col_offset)
        {
            let _ = ref_str;
            result.push_str(&new_ref);
            i += consumed;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Try to parse a cell reference at the given position and return the adjusted version.
///
/// Returns `Some((original, adjusted, chars_consumed))` or `None`.
fn try_parse_and_adjust_ref(
    chars: &[char],
    start: usize,
    row_offset: i32,
    col_offset: i32,
) -> Option<(String, String, usize)> {
    let mut i = start;
    let mut col_absolute = false;
    let mut row_absolute = false;

    // Check for $ before column
    if i < chars.len() && chars[i] == '$' {
        col_absolute = true;
        i += 1;
    }

    // Require at least one letter (column)
    if i >= chars.len() || !chars[i].is_ascii_alphabetic() {
        return None;
    }

    let col_start = i;
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        i += 1;
    }
    let col_letters: String = chars[col_start..i].iter().collect();

    // Check for $ before row
    if i < chars.len() && chars[i] == '$' {
        row_absolute = true;
        i += 1;
    }

    // Require at least one digit (row)
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }

    let row_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    let row_str: String = chars[row_start..i].iter().collect();
    let row_num: i32 = row_str.parse().ok()?;

    // Make sure this isn't followed by another alphanumeric (part of a function name)
    if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '_') {
        return None;
    }

    // Also check that the character before start isn't alphanumeric (would be a function name)
    if start > 0 && chars[start - 1].is_ascii_alphanumeric() && !col_absolute {
        return None;
    }

    let original: String = chars[start..i].iter().collect();

    // Build adjusted reference
    let new_col = if col_absolute {
        col_letters.clone()
    } else {
        let col_idx = col_letters_to_index(&col_letters)? as i32;
        let new_col_idx = (col_idx + col_offset).max(0) as u32;
        index_to_col_letters(new_col_idx)
    };

    let new_row = if row_absolute {
        row_num
    } else {
        (row_num + row_offset).max(1)
    };

    let mut adjusted = String::new();
    if col_absolute {
        adjusted.push('$');
    }
    adjusted.push_str(&new_col);
    if row_absolute {
        adjusted.push('$');
    }
    adjusted.push_str(&new_row.to_string());

    Some((original, adjusted, i - start))
}

/// Convert column letters to 0-based index. "A" -> 0, "Z" -> 25, "AA" -> 26.
fn col_letters_to_index(letters: &str) -> Option<u32> {
    let mut index: u32 = 0;
    for ch in letters.chars() {
        let c = ch.to_ascii_uppercase();
        if !c.is_ascii_uppercase() {
            return None;
        }
        index = index * 26 + (c as u32 - 'A' as u32 + 1);
    }
    Some(index - 1)
}

/// Convert 0-based column index to letters. 0 -> "A", 25 -> "Z", 26 -> "AA".
fn index_to_col_letters(mut col: u32) -> String {
    let mut result = String::new();
    loop {
        let rem = col % 26;
        result.push((b'A' + rem as u8) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::format::CellFormat;

    #[test]
    fn test_copy_range() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(1, 0, CellValue::Number(3.0));
        sheet.set_value(1, 1, CellValue::Number(4.0));

        let clipboard = copy_range(&sheet, 0, 0, 1, 1, false);
        assert_eq!(clipboard.dimensions(), (2, 2));
        assert!(!clipboard.is_cut);
    }

    #[test]
    fn test_paste_values() {
        let mut src_sheet = Sheet::new("Src");
        src_sheet.set_value(0, 0, CellValue::Number(10.0));
        src_sheet.set_value(0, 1, CellValue::Number(20.0));

        let clipboard = copy_range(&src_sheet, 0, 0, 0, 1, false);

        let mut dest_sheet = Sheet::new("Dest");
        paste(&mut dest_sheet, &clipboard, 5, 5, &PasteMode::All).unwrap();

        assert_eq!(
            dest_sheet.get_cell(5, 5).unwrap().value,
            CellValue::Number(10.0)
        );
        assert_eq!(
            dest_sheet.get_cell(5, 6).unwrap().value,
            CellValue::Number(20.0)
        );
    }

    #[test]
    fn test_paste_values_only() {
        let mut sheet = Sheet::new("T");
        let mut cell = Cell::default();
        cell.value = CellValue::Number(42.0);
        cell.formula = Some("A1+B1".to_string());
        sheet.set_cell(0, 0, cell);

        let clipboard = copy_range(&sheet, 0, 0, 0, 0, false);
        let mut dest = Sheet::new("D");
        paste(&mut dest, &clipboard, 0, 0, &PasteMode::ValuesOnly).unwrap();

        let pasted = dest.get_cell(0, 0).unwrap();
        assert_eq!(pasted.value, CellValue::Number(42.0));
        assert!(pasted.formula.is_none());
    }

    #[test]
    fn test_adjust_formula_references() {
        // Moving down 2 rows, right 1 column
        assert_eq!(adjust_formula_references("A1+B2", 2, 1), "B3+C4");
    }

    #[test]
    fn test_adjust_absolute_references() {
        // $A$1 should not change
        assert_eq!(adjust_formula_references("$A$1+B2", 2, 1), "$A$1+C4");
    }

    #[test]
    fn test_adjust_mixed_references() {
        // $A1 -> $A3 (col absolute, row relative)
        // A$1 -> B$1 (col relative, row absolute)
        assert_eq!(adjust_formula_references("$A1", 2, 1), "$A3");
        assert_eq!(adjust_formula_references("A$1", 2, 1), "B$1");
    }

    #[test]
    fn test_adjust_with_function_names() {
        // Function names like SUM should not be adjusted
        let result = adjust_formula_references("SUM(A1:A5)", 1, 0);
        assert_eq!(result, "SUM(A2:A6)");
    }

    #[test]
    fn test_adjust_string_literals_untouched() {
        let result = adjust_formula_references(r#"CONCATENATE("A1", B1)"#, 1, 0);
        assert!(result.contains("\"A1\""));
        assert!(result.contains("B2"));
    }

    #[test]
    fn test_clipboard_dimensions() {
        let cb = ClipboardContent::new();
        assert_eq!(cb.dimensions(), (0, 0));
    }

    #[test]
    fn test_paste_formatting_only() {
        let mut sheet = Sheet::new("T");
        let mut cell = Cell::default();
        cell.value = CellValue::Number(42.0);
        cell.format = CellFormat {
            bold: true,
            ..CellFormat::default()
        };
        sheet.set_cell(0, 0, cell);

        let clipboard = copy_range(&sheet, 0, 0, 0, 0, false);

        // Destination has existing value
        let mut dest = Sheet::new("D");
        dest.set_value(0, 0, CellValue::Text("keep me".into()));

        paste(&mut dest, &clipboard, 0, 0, &PasteMode::FormattingOnly).unwrap();

        let pasted = dest.get_cell(0, 0).unwrap();
        assert_eq!(pasted.value, CellValue::Text("keep me".into()));
        assert!(pasted.format.bold);
    }

    #[test]
    fn test_paste_transposed_basic() {
        // Source: 2 rows x 3 cols
        //   (0,0)=1  (0,1)=2  (0,2)=3
        //   (1,0)=4  (1,1)=5  (1,2)=6
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(2.0));
        sheet.set_value(0, 2, CellValue::Number(3.0));
        sheet.set_value(1, 0, CellValue::Number(4.0));
        sheet.set_value(1, 1, CellValue::Number(5.0));
        sheet.set_value(1, 2, CellValue::Number(6.0));

        let clipboard = copy_range(&sheet, 0, 0, 1, 2, false);
        assert_eq!(clipboard.dimensions(), (2, 3));

        // Paste transposed at (0, 0): should become 3 rows x 2 cols
        //   (0,0)=1  (0,1)=4
        //   (1,0)=2  (1,1)=5
        //   (2,0)=3  (2,1)=6
        let mut dest = Sheet::new("D");
        paste(&mut dest, &clipboard, 0, 0, &PasteMode::Transposed).unwrap();

        assert_eq!(dest.get_cell(0, 0).unwrap().value, CellValue::Number(1.0));
        assert_eq!(dest.get_cell(0, 1).unwrap().value, CellValue::Number(4.0));
        assert_eq!(dest.get_cell(1, 0).unwrap().value, CellValue::Number(2.0));
        assert_eq!(dest.get_cell(1, 1).unwrap().value, CellValue::Number(5.0));
        assert_eq!(dest.get_cell(2, 0).unwrap().value, CellValue::Number(3.0));
        assert_eq!(dest.get_cell(2, 1).unwrap().value, CellValue::Number(6.0));
    }

    #[test]
    fn test_paste_transposed_with_offset() {
        // Source: 1 row x 3 cols at row=0 col=0
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("a".into()));
        sheet.set_value(0, 1, CellValue::Text("b".into()));
        sheet.set_value(0, 2, CellValue::Text("c".into()));

        let clipboard = copy_range(&sheet, 0, 0, 0, 2, false);

        // Paste transposed at (5, 5): should become 3 rows x 1 col
        let mut dest = Sheet::new("D");
        paste(&mut dest, &clipboard, 5, 5, &PasteMode::Transposed).unwrap();

        assert_eq!(
            dest.get_cell(5, 5).unwrap().value,
            CellValue::Text("a".into())
        );
        assert_eq!(
            dest.get_cell(6, 5).unwrap().value,
            CellValue::Text("b".into())
        );
        assert_eq!(
            dest.get_cell(7, 5).unwrap().value,
            CellValue::Text("c".into())
        );
        // No data in the original column direction
        assert!(dest.get_cell(5, 6).is_none());
    }

    #[test]
    fn test_paste_transposed_single_cell() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(99.0));

        let clipboard = copy_range(&sheet, 0, 0, 0, 0, false);
        let mut dest = Sheet::new("D");
        paste(&mut dest, &clipboard, 3, 3, &PasteMode::Transposed).unwrap();

        assert_eq!(dest.get_cell(3, 3).unwrap().value, CellValue::Number(99.0));
    }

    #[test]
    fn test_paste_transposed_preserves_formatting() {
        let mut sheet = Sheet::new("T");
        let mut cell = Cell::default();
        cell.value = CellValue::Number(42.0);
        cell.format = CellFormat {
            bold: true,
            ..CellFormat::default()
        };
        sheet.set_cell(0, 0, cell);
        sheet.set_value(0, 1, CellValue::Number(7.0));

        let clipboard = copy_range(&sheet, 0, 0, 0, 1, false);
        let mut dest = Sheet::new("D");
        paste(&mut dest, &clipboard, 0, 0, &PasteMode::Transposed).unwrap();

        // (0,0) -> (0,0), bold should be preserved
        let pasted = dest.get_cell(0, 0).unwrap();
        assert!(pasted.format.bold);
        assert_eq!(pasted.value, CellValue::Number(42.0));

        // (0,1) -> (1,0), no bold
        let pasted2 = dest.get_cell(1, 0).unwrap();
        assert!(!pasted2.format.bold);
        assert_eq!(pasted2.value, CellValue::Number(7.0));
    }

    #[test]
    fn test_paste_transposed_clears_empty() {
        // When a source cell is None (empty), transposed paste should clear
        // the target cell.
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        // (0,1) is empty

        let clipboard = copy_range(&sheet, 0, 0, 0, 1, false);

        let mut dest = Sheet::new("D");
        // Pre-fill destination so we can verify clearing
        dest.set_value(1, 0, CellValue::Text("should be cleared".into()));

        paste(&mut dest, &clipboard, 0, 0, &PasteMode::Transposed).unwrap();

        assert_eq!(dest.get_cell(0, 0).unwrap().value, CellValue::Number(1.0));
        // (0,1) was empty -> transposed to (1,0), should be cleared
        assert!(dest.get_cell(1, 0).is_none());
    }
}
