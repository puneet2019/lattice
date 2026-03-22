//! Auto-fill pattern detection and range filling.
//!
//! Detects patterns in a source range of cell values and extends them
//! to fill a target range. Supports constant, linear numeric, text
//! with trailing numbers, and repeating-cycle patterns.

use crate::cell::CellValue;
use crate::selection::Range;
use crate::sheet::Sheet;

/// A detected fill pattern that can generate new values.
#[derive(Debug, Clone, PartialEq)]
pub enum FillPattern {
    /// A single constant value repeated.
    Constant(CellValue),
    /// Linear numeric sequence with a start value and step.
    LinearNumber(f64, f64),
    /// Text with a trailing number: prefix, start number, step.
    /// E.g. "Item 1", "Item 2" => TextWithNumber("Item ", 1, 1)
    TextWithNumber(String, i64, i64),
    /// Repeating cycle of values (e.g. "A","B","C","A","B","C"...).
    Repeating(Vec<CellValue>),
}

/// The direction in which to fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FillDirection {
    /// Fill downward (increasing row).
    Down,
    /// Fill to the right (increasing column).
    Right,
    /// Fill upward (decreasing row).
    Up,
    /// Fill to the left (decreasing column).
    Left,
}

/// Detect the fill pattern from a slice of cell values.
///
/// Returns `None` if the input is empty. For a single value, returns
/// `Constant`. For multiple values, tries to detect linear numeric,
/// text-with-number, or repeating patterns.
///
/// # Examples
/// ```
/// use lattice_core::cell::CellValue;
/// use lattice_core::autofill::{detect_pattern, FillPattern};
///
/// let vals = vec![CellValue::Number(1.0), CellValue::Number(2.0), CellValue::Number(3.0)];
/// let pat = detect_pattern(&vals).unwrap();
/// assert_eq!(pat, FillPattern::LinearNumber(1.0, 1.0));
/// ```
pub fn detect_pattern(values: &[CellValue]) -> Option<FillPattern> {
    if values.is_empty() {
        return None;
    }

    if values.len() == 1 {
        return match &values[0] {
            CellValue::Number(n) => Some(FillPattern::Constant(CellValue::Number(*n))),
            other => Some(FillPattern::Constant(other.clone())),
        };
    }

    // Try linear number pattern
    if let Some(pat) = try_linear_number(values) {
        return Some(pat);
    }

    // Try text-with-number pattern
    if let Some(pat) = try_text_with_number(values) {
        return Some(pat);
    }

    // Fall back to repeating cycle
    Some(FillPattern::Repeating(values.to_vec()))
}

/// Try to detect a linear numeric pattern (all values are numbers with
/// equal step between consecutive entries).
fn try_linear_number(values: &[CellValue]) -> Option<FillPattern> {
    let numbers: Vec<f64> = values
        .iter()
        .map(|v| match v {
            CellValue::Number(n) => Some(*n),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;

    if numbers.len() < 2 {
        return None;
    }

    let step = numbers[1] - numbers[0];
    for i in 2..numbers.len() {
        let expected = numbers[0] + step * (i as f64);
        if (numbers[i] - expected).abs() > 1e-10 {
            return None;
        }
    }

    Some(FillPattern::LinearNumber(numbers[0], step))
}

/// Try to detect a text-with-trailing-number pattern.
/// E.g. "Item 1", "Item 2", "Item 3" or "Q1", "Q2", "Q3".
fn try_text_with_number(values: &[CellValue]) -> Option<FillPattern> {
    let texts: Vec<&str> = values
        .iter()
        .map(|v| match v {
            CellValue::Text(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;

    if texts.len() < 2 {
        return None;
    }

    // Extract (prefix, number) from each text
    let parts: Vec<(&str, i64)> = texts
        .iter()
        .map(|t| split_text_number(t))
        .collect::<Option<Vec<_>>>()?;

    // All prefixes must be the same
    let prefix = parts[0].0;
    if !parts.iter().all(|(p, _)| *p == prefix) {
        return None;
    }

    // Check for constant step in the numbers
    let step = parts[1].1 - parts[0].1;
    for i in 2..parts.len() {
        let expected = parts[0].1 + step * (i as i64);
        if parts[i].1 != expected {
            return None;
        }
    }

    Some(FillPattern::TextWithNumber(
        prefix.to_string(),
        parts[0].1,
        step,
    ))
}

/// Split a string into a text prefix and a trailing integer.
/// Returns `None` if there is no trailing number.
fn split_text_number(s: &str) -> Option<(&str, i64)> {
    // Find where the trailing digits start
    let digit_start = s
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .map(|(i, _)| i)?;

    let prefix = &s[..digit_start];
    let num_str = &s[digit_start..];
    let num: i64 = num_str.parse().ok()?;

    Some((prefix, num))
}

/// Generate the next value from a `FillPattern` at a given index
/// (0-based offset from the end of the source range).
fn generate_value(pattern: &FillPattern, index: usize) -> CellValue {
    match pattern {
        FillPattern::Constant(v) => v.clone(),
        FillPattern::LinearNumber(start, step) => {
            CellValue::Number(start + step * ((index) as f64))
        }
        FillPattern::TextWithNumber(prefix, start, step) => {
            let n = start + step * (index as i64);
            CellValue::Text(format!("{}{}", prefix, n))
        }
        FillPattern::Repeating(values) => {
            let i = index % values.len();
            values[i].clone()
        }
    }
}

/// Fill a target range on a sheet based on the pattern detected from a
/// source range.
///
/// The source range provides the values used for pattern detection. The
/// target range is then filled with the continuation of that pattern.
///
/// # Direction semantics
/// - `Down`/`Right` — the target range is *after* the source range.
/// - `Up`/`Left` — the target range is *before* the source range.
///   Values are generated in reverse so that the cell closest to the
///   source gets the next value in the sequence.
pub fn fill_range(
    sheet: &mut Sheet,
    source: &Range,
    target: &Range,
    direction: FillDirection,
) {
    let source_values = read_values(sheet, source, direction);
    let pattern = match detect_pattern(&source_values) {
        Some(p) => p,
        None => return,
    };

    let source_len = source_values.len();
    let target_cells = target_positions(target, direction);

    for (i, (row, col)) in target_cells.iter().enumerate() {
        let value = generate_value(&pattern, source_len + i);
        sheet.set_value(*row, *col, value);
    }
}

/// Read values from the source range in the order appropriate for the
/// fill direction.
fn read_values(sheet: &Sheet, range: &Range, direction: FillDirection) -> Vec<CellValue> {
    let positions = source_positions(range, direction);
    positions
        .iter()
        .map(|(r, c)| {
            sheet
                .get_cell(*r, *c)
                .map(|cell| cell.value.clone())
                .unwrap_or(CellValue::Empty)
        })
        .collect()
}

/// Return (row, col) pairs for the source range in natural order for
/// the given fill direction.
fn source_positions(range: &Range, direction: FillDirection) -> Vec<(u32, u32)> {
    match direction {
        FillDirection::Down | FillDirection::Up => {
            // Read column by column, top to bottom
            let col = range.start.col; // For a single-column source
            (range.start.row..=range.end.row)
                .map(|r| (r, col))
                .collect()
        }
        FillDirection::Right | FillDirection::Left => {
            // Read row by row, left to right
            let row = range.start.row;
            (range.start.col..=range.end.col)
                .map(|c| (row, c))
                .collect()
        }
    }
}

/// Return (row, col) pairs for the target range in natural order.
fn target_positions(range: &Range, direction: FillDirection) -> Vec<(u32, u32)> {
    match direction {
        FillDirection::Down => {
            let col = range.start.col;
            (range.start.row..=range.end.row)
                .map(|r| (r, col))
                .collect()
        }
        FillDirection::Up => {
            let col = range.start.col;
            // Fill from bottom to top so the row closest to source gets
            // the next value.
            (range.start.row..=range.end.row)
                .rev()
                .map(|r| (r, col))
                .collect()
        }
        FillDirection::Right => {
            let row = range.start.row;
            (range.start.col..=range.end.col)
                .map(|c| (row, c))
                .collect()
        }
        FillDirection::Left => {
            let row = range.start.row;
            (range.start.col..=range.end.col)
                .rev()
                .map(|c| (row, c))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::CellRef;

    // --- Pattern detection ---

    #[test]
    fn test_detect_empty() {
        assert!(detect_pattern(&[]).is_none());
    }

    #[test]
    fn test_detect_single_number_constant() {
        let vals = vec![CellValue::Number(5.0)];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::Constant(CellValue::Number(5.0)))
        );
    }

    #[test]
    fn test_detect_single_text_constant() {
        let vals = vec![CellValue::Text("hello".into())];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::Constant(CellValue::Text("hello".into())))
        );
    }

    #[test]
    fn test_detect_linear_number() {
        let vals = vec![
            CellValue::Number(1.0),
            CellValue::Number(2.0),
            CellValue::Number(3.0),
        ];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::LinearNumber(1.0, 1.0))
        );
    }

    #[test]
    fn test_detect_linear_number_step_10() {
        let vals = vec![CellValue::Number(10.0), CellValue::Number(20.0)];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::LinearNumber(10.0, 10.0))
        );
    }

    #[test]
    fn test_detect_linear_number_negative_step() {
        let vals = vec![
            CellValue::Number(10.0),
            CellValue::Number(7.0),
            CellValue::Number(4.0),
        ];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::LinearNumber(10.0, -3.0))
        );
    }

    #[test]
    fn test_detect_text_with_number() {
        let vals = vec![
            CellValue::Text("Item 1".into()),
            CellValue::Text("Item 2".into()),
        ];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::TextWithNumber("Item ".into(), 1, 1))
        );
    }

    #[test]
    fn test_detect_text_with_number_q() {
        let vals = vec![
            CellValue::Text("Q1".into()),
            CellValue::Text("Q2".into()),
            CellValue::Text("Q3".into()),
        ];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::TextWithNumber("Q".into(), 1, 1))
        );
    }

    #[test]
    fn test_detect_repeating_text() {
        let vals = vec![
            CellValue::Text("A".into()),
            CellValue::Text("B".into()),
            CellValue::Text("C".into()),
        ];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::Repeating(vals.clone()))
        );
    }

    #[test]
    fn test_detect_mixed_types_repeating() {
        let vals = vec![CellValue::Number(1.0), CellValue::Text("X".into())];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::Repeating(vals.clone()))
        );
    }

    // --- Value generation ---

    #[test]
    fn test_generate_constant() {
        let pat = FillPattern::Constant(CellValue::Number(5.0));
        assert_eq!(generate_value(&pat, 0), CellValue::Number(5.0));
        assert_eq!(generate_value(&pat, 5), CellValue::Number(5.0));
    }

    #[test]
    fn test_generate_linear() {
        let pat = FillPattern::LinearNumber(1.0, 1.0);
        assert_eq!(generate_value(&pat, 3), CellValue::Number(4.0));
    }

    #[test]
    fn test_generate_text_with_number() {
        let pat = FillPattern::TextWithNumber("Item ".into(), 1, 1);
        assert_eq!(generate_value(&pat, 2), CellValue::Text("Item 3".into()));
    }

    #[test]
    fn test_generate_repeating() {
        let pat = FillPattern::Repeating(vec![
            CellValue::Text("A".into()),
            CellValue::Text("B".into()),
            CellValue::Text("C".into()),
        ]);
        assert_eq!(generate_value(&pat, 0), CellValue::Text("A".into()));
        assert_eq!(generate_value(&pat, 1), CellValue::Text("B".into()));
        assert_eq!(generate_value(&pat, 2), CellValue::Text("C".into()));
        assert_eq!(generate_value(&pat, 3), CellValue::Text("A".into()));
    }

    // --- End-to-end fill_range tests ---

    #[test]
    fn test_fill_constant_down() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(5.0));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 0, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 1, col: 0 },
            end: CellRef { row: 4, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        for r in 1..=4 {
            assert_eq!(
                sheet.get_cell(r, 0).unwrap().value,
                CellValue::Number(5.0),
                "row {r} should be 5.0"
            );
        }
    }

    #[test]
    fn test_fill_linear_down() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(1, 0, CellValue::Number(2.0));
        sheet.set_value(2, 0, CellValue::Number(3.0));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 2, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 3, col: 0 },
            end: CellRef { row: 6, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        assert_eq!(sheet.get_cell(3, 0).unwrap().value, CellValue::Number(4.0));
        assert_eq!(sheet.get_cell(4, 0).unwrap().value, CellValue::Number(5.0));
        assert_eq!(sheet.get_cell(5, 0).unwrap().value, CellValue::Number(6.0));
        assert_eq!(sheet.get_cell(6, 0).unwrap().value, CellValue::Number(7.0));
    }

    #[test]
    fn test_fill_step_down() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(10.0));
        sheet.set_value(1, 0, CellValue::Number(20.0));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 1, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 2, col: 0 },
            end: CellRef { row: 4, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        assert_eq!(sheet.get_cell(2, 0).unwrap().value, CellValue::Number(30.0));
        assert_eq!(sheet.get_cell(3, 0).unwrap().value, CellValue::Number(40.0));
        assert_eq!(sheet.get_cell(4, 0).unwrap().value, CellValue::Number(50.0));
    }

    #[test]
    fn test_fill_text_with_number() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("Item 1".into()));
        sheet.set_value(1, 0, CellValue::Text("Item 2".into()));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 1, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 2, col: 0 },
            end: CellRef { row: 3, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        assert_eq!(
            sheet.get_cell(2, 0).unwrap().value,
            CellValue::Text("Item 3".into())
        );
        assert_eq!(
            sheet.get_cell(3, 0).unwrap().value,
            CellValue::Text("Item 4".into())
        );
    }

    #[test]
    fn test_fill_repeating_cycle() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("A".into()));
        sheet.set_value(1, 0, CellValue::Text("B".into()));
        sheet.set_value(2, 0, CellValue::Text("C".into()));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 2, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 3, col: 0 },
            end: CellRef { row: 8, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        assert_eq!(sheet.get_cell(3, 0).unwrap().value, CellValue::Text("A".into()));
        assert_eq!(sheet.get_cell(4, 0).unwrap().value, CellValue::Text("B".into()));
        assert_eq!(sheet.get_cell(5, 0).unwrap().value, CellValue::Text("C".into()));
        assert_eq!(sheet.get_cell(6, 0).unwrap().value, CellValue::Text("A".into()));
    }

    #[test]
    fn test_fill_single_text_constant() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Text("hello".into()));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 0, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 1, col: 0 },
            end: CellRef { row: 3, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        for r in 1..=3 {
            assert_eq!(
                sheet.get_cell(r, 0).unwrap().value,
                CellValue::Text("hello".into())
            );
        }
    }

    #[test]
    fn test_fill_right() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 0, CellValue::Number(1.0));
        sheet.set_value(0, 1, CellValue::Number(3.0));
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 0, col: 1 },
        };
        let target = Range {
            start: CellRef { row: 0, col: 2 },
            end: CellRef { row: 0, col: 4 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Right);
        assert_eq!(sheet.get_cell(0, 2).unwrap().value, CellValue::Number(5.0));
        assert_eq!(sheet.get_cell(0, 3).unwrap().value, CellValue::Number(7.0));
        assert_eq!(sheet.get_cell(0, 4).unwrap().value, CellValue::Number(9.0));
    }

    #[test]
    fn test_fill_empty_source() {
        let mut sheet = Sheet::new("T");
        let source = Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 0, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 1, col: 0 },
            end: CellRef { row: 3, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Down);
        for r in 1..=3 {
            assert_eq!(sheet.get_cell(r, 0).unwrap().value, CellValue::Empty);
        }
    }

    #[test]
    fn test_split_text_number() {
        assert_eq!(split_text_number("Item 3"), Some(("Item ", 3)));
        assert_eq!(split_text_number("Q1"), Some(("Q", 1)));
        assert_eq!(split_text_number("abc"), None);
        assert_eq!(split_text_number("123"), Some(("", 123)));
    }

    #[test]
    fn test_fill_up() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(5, 0, CellValue::Number(10.0));
        sheet.set_value(6, 0, CellValue::Number(20.0));
        let source = Range {
            start: CellRef { row: 5, col: 0 },
            end: CellRef { row: 6, col: 0 },
        };
        let target = Range {
            start: CellRef { row: 3, col: 0 },
            end: CellRef { row: 4, col: 0 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Up);
        assert_eq!(sheet.get_cell(4, 0).unwrap().value, CellValue::Number(30.0));
        assert_eq!(sheet.get_cell(3, 0).unwrap().value, CellValue::Number(40.0));
    }

    #[test]
    fn test_fill_left() {
        let mut sheet = Sheet::new("T");
        sheet.set_value(0, 5, CellValue::Number(2.0));
        sheet.set_value(0, 6, CellValue::Number(4.0));
        let source = Range {
            start: CellRef { row: 0, col: 5 },
            end: CellRef { row: 0, col: 6 },
        };
        let target = Range {
            start: CellRef { row: 0, col: 3 },
            end: CellRef { row: 0, col: 4 },
        };
        fill_range(&mut sheet, &source, &target, FillDirection::Left);
        assert_eq!(sheet.get_cell(0, 4).unwrap().value, CellValue::Number(6.0));
        assert_eq!(sheet.get_cell(0, 3).unwrap().value, CellValue::Number(8.0));
    }

    #[test]
    fn test_detect_two_equal_numbers_zero_step() {
        let vals = vec![CellValue::Number(5.0), CellValue::Number(5.0)];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::LinearNumber(5.0, 0.0))
        );
    }

    #[test]
    fn test_detect_boolean_constant() {
        let vals = vec![CellValue::Boolean(true)];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::Constant(CellValue::Boolean(true)))
        );
    }

    #[test]
    fn test_detect_non_uniform_numbers_repeating() {
        let vals = vec![
            CellValue::Number(1.0),
            CellValue::Number(2.0),
            CellValue::Number(4.0),
        ];
        assert_eq!(
            detect_pattern(&vals),
            Some(FillPattern::Repeating(vals.clone()))
        );
    }
}
