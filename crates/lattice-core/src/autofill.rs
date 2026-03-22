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
