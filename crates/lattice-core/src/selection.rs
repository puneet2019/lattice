use serde::{Deserialize, Serialize};

use crate::error::{LatticeError, Result};

/// A reference to a single cell, e.g. A1 is `{ row: 0, col: 0 }`.
/// Both row and col are 0-based internally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellRef {
    /// 0-based row index.
    pub row: u32,
    /// 0-based column index.
    pub col: u32,
}

impl CellRef {
    /// Parse an A1-style cell reference string into a [`CellRef`].
    pub fn parse(s: &str) -> Result<Self> {
        parse_cell_ref(s)
    }
}

/// A rectangular range between two cell references (inclusive).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Range {
    /// Top-left corner.
    pub start: CellRef,
    /// Bottom-right corner.
    pub end: CellRef,
}

/// A selection that can be a single cell, a range, or multiple ranges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Selection {
    /// A single cell.
    Cell(CellRef),
    /// A contiguous rectangular range.
    Range(Range),
    /// Multiple discontiguous ranges (Cmd+Click).
    Multi(Vec<Range>),
}

/// Parse an A1-style cell reference into a [`CellRef`].
///
/// Supports columns from `A` through arbitrary multi-letter columns (e.g. `AA`,
/// `AZ`, `BA`, ...) and 1-based row numbers.
///
/// # Examples
/// ```
/// use lattice_core::selection::parse_cell_ref;
/// let r = parse_cell_ref("A1").unwrap();
/// assert_eq!(r.row, 0);
/// assert_eq!(r.col, 0);
/// ```
pub fn parse_cell_ref(s: &str) -> Result<CellRef> {
    let s = s.trim();
    // Strip any dollar signs for absolute references.
    let s: String = s.chars().filter(|c| *c != '$').collect();

    let first_digit = s
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| LatticeError::InvalidCellRef(s.clone()))?;

    if first_digit == 0 {
        return Err(LatticeError::InvalidCellRef(s));
    }

    let col_part = &s[..first_digit];
    let row_part = &s[first_digit..];

    if !col_part.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(LatticeError::InvalidCellRef(s));
    }

    let col = col_letters_to_index(col_part)?;
    let row: u32 = row_part
        .parse::<u32>()
        .map_err(|_| LatticeError::InvalidCellRef(s.clone()))?;

    if row == 0 {
        return Err(LatticeError::InvalidCellRef(s));
    }

    Ok(CellRef {
        row: row - 1, // 1-based -> 0-based
        col,
    })
}

/// Convert column letters (e.g. `"A"` -> 0, `"Z"` -> 25, `"AA"` -> 26) to a
/// 0-based column index.
fn col_letters_to_index(letters: &str) -> Result<u32> {
    let mut index: u32 = 0;
    for ch in letters.chars() {
        let c = ch.to_ascii_uppercase();
        if !c.is_ascii_uppercase() {
            return Err(LatticeError::InvalidCellRef(letters.to_string()));
        }
        index = index * 26 + (c as u32 - 'A' as u32 + 1);
    }
    Ok(index - 1) // convert from 1-based to 0-based
}

/// Convert a 0-based column index to spreadsheet column letters.
///
/// # Examples
/// ```
/// use lattice_core::selection::col_to_letter;
/// assert_eq!(col_to_letter(0), "A");
/// assert_eq!(col_to_letter(25), "Z");
/// assert_eq!(col_to_letter(26), "AA");
/// ```
pub fn col_to_letter(mut col: u32) -> String {
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

    #[test]
    fn test_parse_a1() {
        let r = parse_cell_ref("A1").unwrap();
        assert_eq!(r, CellRef { row: 0, col: 0 });
    }

    #[test]
    fn test_parse_z99() {
        let r = parse_cell_ref("Z99").unwrap();
        assert_eq!(r, CellRef { row: 98, col: 25 });
    }

    #[test]
    fn test_parse_aa1() {
        let r = parse_cell_ref("AA1").unwrap();
        assert_eq!(r, CellRef { row: 0, col: 26 });
    }

    #[test]
    fn test_col_to_letter_a() {
        assert_eq!(col_to_letter(0), "A");
    }

    #[test]
    fn test_col_to_letter_z() {
        assert_eq!(col_to_letter(25), "Z");
    }

    #[test]
    fn test_col_to_letter_aa() {
        assert_eq!(col_to_letter(26), "AA");
    }

    #[test]
    fn test_roundtrip() {
        for i in 0..100 {
            let letters = col_to_letter(i);
            let idx = col_letters_to_index(&letters).unwrap();
            assert_eq!(idx, i, "roundtrip failed for col {i} -> {letters}");
        }
    }
}
