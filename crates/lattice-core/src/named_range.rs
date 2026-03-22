//! Named range management for workbooks.
//!
//! A named range maps a user-defined name (e.g. `"Revenue"`) to a
//! rectangular cell range, optionally scoped to a specific sheet. Names
//! are looked up case-insensitively and must start with a letter or
//! underscore, followed by alphanumeric characters or underscores.

use std::collections::HashMap;

use crate::error::{LatticeError, Result};
use crate::selection::Range;

/// A named range binding a human-readable name to a cell range.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedRange {
    /// The canonical (original-case) name, e.g. `"Revenue"`.
    pub name: String,
    /// If `Some`, the range is scoped to this sheet. If `None`, the range
    /// is workbook-scoped.
    pub sheet: Option<String>,
    /// The cell range this name resolves to.
    pub range: Range,
}

/// A store of named ranges with case-insensitive lookup.
#[derive(Debug, Clone, Default)]
pub struct NamedRangeStore {
    /// Maps lowercase name -> NamedRange.
    ranges: HashMap<String, NamedRange>,
}

/// Validate that `name` is a legal named-range identifier.
///
/// Rules: must start with a letter or underscore; remaining characters
/// must be alphanumeric or underscore; must not be empty.
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(LatticeError::InvalidRange(
            "named range name cannot be empty".into(),
        ));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(LatticeError::InvalidRange(format!(
            "named range name must start with a letter or underscore, got '{name}'"
        )));
    }

    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return Err(LatticeError::InvalidRange(format!(
                "named range name contains invalid character '{ch}' in '{name}'"
            )));
        }
    }

    Ok(())
}

impl NamedRangeStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a named range.
    ///
    /// Returns an error if the name is invalid or already exists
    /// (case-insensitive).
    pub fn add(
        &mut self,
        name: impl Into<String>,
        sheet: Option<String>,
        range: Range,
    ) -> Result<()> {
        let name = name.into();
        validate_name(&name)?;

        let key = name.to_lowercase();
        if self.ranges.contains_key(&key) {
            return Err(LatticeError::InvalidRange(format!(
                "named range '{name}' already exists"
            )));
        }

        self.ranges.insert(
            key,
            NamedRange {
                name,
                sheet,
                range,
            },
        );
        Ok(())
    }

    /// Remove a named range by name (case-insensitive).
    ///
    /// Returns an error if the name does not exist.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let key = name.to_lowercase();
        if self.ranges.remove(&key).is_none() {
            return Err(LatticeError::InvalidRange(format!(
                "named range '{name}' not found"
            )));
        }
        Ok(())
    }

    /// Look up a named range by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&NamedRange> {
        self.ranges.get(&name.to_lowercase())
    }

    /// Return all named ranges in arbitrary order.
    pub fn list(&self) -> Vec<&NamedRange> {
        self.ranges.values().collect()
    }

    /// Resolve a name to its `(sheet_name, Range)` tuple.
    ///
    /// If the named range is workbook-scoped (sheet is `None`), the
    /// returned sheet name is `"Sheet1"` as a default — callers should
    /// handle this based on context.
    pub fn resolve(&self, name: &str) -> Option<(Option<&str>, &Range)> {
        self.ranges
            .get(&name.to_lowercase())
            .map(|nr| (nr.sheet.as_deref(), &nr.range))
    }

    /// Return the number of named ranges in the store.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Return `true` if there are no named ranges.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::CellRef;

    fn range(r1: u32, c1: u32, r2: u32, c2: u32) -> Range {
        Range {
            start: CellRef { row: r1, col: c1 },
            end: CellRef { row: r2, col: c2 },
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut store = NamedRangeStore::new();
        store.add("Revenue", None, range(0, 0, 9, 0)).unwrap();
        let nr = store.get("Revenue").unwrap();
        assert_eq!(nr.name, "Revenue");
        assert_eq!(nr.sheet, None);
        assert_eq!(nr.range.start.row, 0);
        assert_eq!(nr.range.end.row, 9);
    }

    #[test]
    fn test_case_insensitive_lookup() {
        let mut store = NamedRangeStore::new();
        store.add("Revenue", None, range(0, 0, 9, 0)).unwrap();
        assert!(store.get("revenue").is_some());
        assert!(store.get("REVENUE").is_some());
        assert!(store.get("ReVeNuE").is_some());
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let mut store = NamedRangeStore::new();
        store.add("Revenue", None, range(0, 0, 9, 0)).unwrap();
        let err = store.add("revenue", None, range(1, 1, 5, 5));
        assert!(err.is_err());
    }

    #[test]
    fn test_remove() {
        let mut store = NamedRangeStore::new();
        store.add("Revenue", None, range(0, 0, 9, 0)).unwrap();
        store.remove("revenue").unwrap();
        assert!(store.get("Revenue").is_none());
        assert!(store.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_errors() {
        let mut store = NamedRangeStore::new();
        assert!(store.remove("nothing").is_err());
    }

    #[test]
    fn test_list() {
        let mut store = NamedRangeStore::new();
        store.add("Alpha", None, range(0, 0, 0, 0)).unwrap();
        store.add("Beta", Some("Sheet2".into()), range(1, 1, 5, 5)).unwrap();
        assert_eq!(store.list().len(), 2);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_resolve() {
        let mut store = NamedRangeStore::new();
        store.add("Sales", Some("Data".into()), range(0, 0, 99, 3)).unwrap();
        let (sheet, r) = store.resolve("sales").unwrap();
        assert_eq!(sheet, Some("Data"));
        assert_eq!(r.end.row, 99);
    }

    #[test]
    fn test_resolve_workbook_scoped() {
        let mut store = NamedRangeStore::new();
        store.add("Total", None, range(0, 0, 0, 0)).unwrap();
        let (sheet, _) = store.resolve("total").unwrap();
        assert_eq!(sheet, None);
    }

    #[test]
    fn test_resolve_nonexistent() {
        let store = NamedRangeStore::new();
        assert!(store.resolve("nothing").is_none());
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("Revenue").is_ok());
        assert!(validate_name("_private").is_ok());
        assert!(validate_name("x1").is_ok());
        assert!(validate_name("A_B_C").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("1abc").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("no-dash").is_err());
        assert!(validate_name("no.dot").is_err());
    }

    #[test]
    fn test_sheet_scoped_range() {
        let mut store = NamedRangeStore::new();
        store.add("Header", Some("Sheet1".into()), range(0, 0, 0, 5)).unwrap();
        let nr = store.get("header").unwrap();
        assert_eq!(nr.sheet.as_deref(), Some("Sheet1"));
    }
}
