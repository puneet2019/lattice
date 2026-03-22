use indexmap::IndexMap;

use crate::cell::{Cell, CellValue};
use crate::error::{LatticeError, Result};
use crate::named_range::NamedRangeStore;
use crate::sheet::Sheet;
use crate::validation::ValidationStore;

/// A workbook contains one or more ordered sheets.
#[derive(Debug, Clone)]
pub struct Workbook {
    /// Sheets stored in tab order.
    sheets: IndexMap<String, Sheet>,
    /// Name of the currently active sheet.
    pub active_sheet: String,
    /// Named ranges defined in this workbook.
    pub named_ranges: NamedRangeStore,
    /// Data validation rules defined in this workbook.
    pub validations: ValidationStore,
}

impl Workbook {
    /// Create a new workbook with a single default sheet called `"Sheet1"`.
    pub fn new() -> Self {
        let default_name = "Sheet1".to_string();
        let mut sheets = IndexMap::new();
        sheets.insert(default_name.clone(), Sheet::new(&default_name));
        Self {
            sheets,
            active_sheet: default_name,
            named_ranges: NamedRangeStore::new(),
            validations: ValidationStore::new(),
        }
    }

    /// Add a new empty sheet. Returns an error if the name is already taken.
    pub fn add_sheet(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if self.sheets.contains_key(&name) {
            return Err(LatticeError::SheetAlreadyExists(name));
        }
        self.sheets.insert(name.clone(), Sheet::new(&name));
        Ok(())
    }

    /// Remove a sheet by name. Returns an error if it is the last sheet or
    /// if the sheet does not exist.
    pub fn remove_sheet(&mut self, name: &str) -> Result<()> {
        if self.sheets.len() <= 1 {
            return Err(LatticeError::CannotRemoveLastSheet);
        }
        if self.sheets.shift_remove(name).is_none() {
            return Err(LatticeError::SheetNotFound(name.to_string()));
        }
        // If the active sheet was removed, switch to the first remaining one.
        if self.active_sheet == name
            && let Some(first) = self.sheets.keys().next()
        {
            self.active_sheet = first.clone();
        }
        Ok(())
    }

    /// Get an immutable reference to a sheet by name.
    pub fn get_sheet(&self, name: &str) -> Result<&Sheet> {
        self.sheets
            .get(name)
            .ok_or_else(|| LatticeError::SheetNotFound(name.to_string()))
    }

    /// Get a mutable reference to a sheet by name.
    pub fn get_sheet_mut(&mut self, name: &str) -> Result<&mut Sheet> {
        self.sheets
            .get_mut(name)
            .ok_or_else(|| LatticeError::SheetNotFound(name.to_string()))
    }

    /// Return an ordered list of sheet names.
    pub fn sheet_names(&self) -> Vec<String> {
        self.sheets.keys().cloned().collect()
    }

    /// Convenience: set a cell value on a named sheet.
    pub fn set_cell(&mut self, sheet: &str, row: u32, col: u32, value: CellValue) -> Result<()> {
        self.get_sheet_mut(sheet)?.set_value(row, col, value);
        Ok(())
    }

    /// Convenience: get a cell reference from a named sheet.
    pub fn get_cell(&self, sheet: &str, row: u32, col: u32) -> Result<Option<&Cell>> {
        Ok(self.get_sheet(sheet)?.get_cell(row, col))
    }

    /// Rename an existing sheet. Returns an error if `old` does not exist or
    /// `new_name` is already taken.
    pub fn rename_sheet(&mut self, old: &str, new_name: impl Into<String>) -> Result<()> {
        let new_name = new_name.into();
        if !self.sheets.contains_key(old) {
            return Err(LatticeError::SheetNotFound(old.to_string()));
        }
        if self.sheets.contains_key(&new_name) {
            return Err(LatticeError::SheetAlreadyExists(new_name));
        }
        // Remove old entry, update sheet.name, re-insert at same position.
        // IndexMap doesn't support key rename directly, so we rebuild.
        let mut new_sheets = IndexMap::new();
        for (key, mut sheet) in self.sheets.drain(..) {
            if key == old {
                sheet.name = new_name.clone();
                new_sheets.insert(new_name.clone(), sheet);
            } else {
                new_sheets.insert(key, sheet);
            }
        }
        self.sheets = new_sheets;
        if self.active_sheet == old {
            self.active_sheet = new_name;
        }
        Ok(())
    }
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_workbook_has_sheet1() {
        let wb = Workbook::new();
        assert_eq!(wb.sheet_names(), vec!["Sheet1"]);
        assert_eq!(wb.active_sheet, "Sheet1");
    }

    #[test]
    fn test_add_and_remove_sheet() {
        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();
        assert_eq!(wb.sheet_names(), vec!["Sheet1", "Data"]);
        wb.remove_sheet("Sheet1").unwrap();
        assert_eq!(wb.sheet_names(), vec!["Data"]);
    }

    #[test]
    fn test_cannot_remove_last_sheet() {
        let mut wb = Workbook::new();
        let err = wb.remove_sheet("Sheet1").unwrap_err();
        assert_eq!(err, LatticeError::CannotRemoveLastSheet);
    }

    #[test]
    fn test_set_get_cell_via_workbook() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(42.0))
            .unwrap();
        let cell = wb.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        assert_eq!(cell.value, CellValue::Number(42.0));
    }

    #[test]
    fn test_rename_sheet() {
        let mut wb = Workbook::new();
        wb.rename_sheet("Sheet1", "Summary").unwrap();
        assert_eq!(wb.sheet_names(), vec!["Summary"]);
        assert_eq!(wb.active_sheet, "Summary");
    }

    #[test]
    fn test_rename_sheet_not_found() {
        let mut wb = Workbook::new();
        let err = wb.rename_sheet("NoSuch", "X").unwrap_err();
        assert_eq!(err, LatticeError::SheetNotFound("NoSuch".to_string()));
    }

    #[test]
    fn test_rename_sheet_duplicate_name() {
        let mut wb = Workbook::new();
        wb.add_sheet("Other").unwrap();
        let err = wb.rename_sheet("Sheet1", "Other").unwrap_err();
        assert_eq!(err, LatticeError::SheetAlreadyExists("Other".to_string()));
    }
}
