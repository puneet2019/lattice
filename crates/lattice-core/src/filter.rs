use serde::{Deserialize, Serialize};

/// A filter condition for an auto-filter column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterCondition {
    /// Show cells whose text equals the given string.
    Equals(String),
    /// Show cells whose text contains the given substring.
    Contains(String),
    /// Show cells whose numeric value is greater than the threshold.
    GreaterThan(f64),
    /// Show cells whose numeric value is less than the threshold.
    LessThan(f64),
    /// Show non-empty cells.
    NonEmpty,
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
}

impl Default for AutoFilter {
    fn default() -> Self {
        Self::new()
    }
}
