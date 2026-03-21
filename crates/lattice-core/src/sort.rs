use serde::{Deserialize, Serialize};

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
/// This is currently a stub; the full implementation will sort in-place,
/// respecting multi-key ordering and cell types.
pub fn sort_range(
    _sheet: &mut crate::sheet::Sheet,
    _start_row: u32,
    _end_row: u32,
    _keys: &[SortKey],
) {
    // TODO: implement multi-key stable sort
}
