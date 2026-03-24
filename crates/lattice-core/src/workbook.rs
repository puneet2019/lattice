use indexmap::IndexMap;

use crate::cell::{Cell, CellValue};
use crate::error::{LatticeError, Result};
use crate::filter_view::FilterViewStore;
use crate::formula::SheetResolver;
use crate::named_function::{NamedFunction, NamedFunctionStore};
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
    /// Named filter views for quickly switching row visibility.
    pub filter_views: FilterViewStore,
    /// Data validation rules defined in this workbook.
    pub validations: ValidationStore,
    /// User-defined named functions (LAMBDA aliases).
    pub named_functions: NamedFunctionStore,
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
            filter_views: FilterViewStore::new(),
            validations: ValidationStore::new(),
            named_functions: NamedFunctionStore::new(),
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

    // ── Named Functions ───────────────────────────────────────────────

    /// Add a user-defined named function (LAMBDA alias).
    pub fn add_named_function(
        &mut self,
        name: impl Into<String>,
        params: Vec<String>,
        body: impl Into<String>,
        description: Option<String>,
    ) -> Result<()> {
        self.named_functions.add(name, params, body, description)
    }

    /// Remove a named function by name.
    pub fn remove_named_function(&mut self, name: &str) -> Result<()> {
        self.named_functions.remove(name)
    }

    /// List all named functions.
    pub fn list_named_functions(&self) -> Vec<&NamedFunction> {
        self.named_functions.list()
    }

    /// Get a named function by name (case-insensitive).
    pub fn get_named_function(&self, name: &str) -> Option<&NamedFunction> {
        self.named_functions.get(name)
    }
}

impl SheetResolver for Workbook {
    fn resolve_cell(&self, sheet_name: &str, row: u32, col: u32) -> Result<CellValue> {
        let sheet = self.get_sheet(sheet_name)?;
        Ok(sheet
            .get_cell(row, col)
            .map(|c| c.value.clone())
            .unwrap_or(CellValue::Empty))
    }

    fn resolve_named_function(&self, name: &str) -> Option<&NamedFunction> {
        self.named_functions.get(name)
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

    // --- Cross-sheet formula evaluation ---

    #[test]
    fn test_cross_sheet_simple_ref() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_sheet("Sheet2").unwrap();
        wb.set_cell("Sheet2", 0, 0, CellValue::Number(42.0))
            .unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval
            .evaluate_with_context("Sheet2!A1", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Number(42.0));
    }

    #[test]
    fn test_cross_sheet_in_expression() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_sheet("Sheet2").unwrap();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(10.0))
            .unwrap();
        wb.set_cell("Sheet2", 0, 0, CellValue::Number(20.0))
            .unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval
            .evaluate_with_context("A1 + Sheet2!A1", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Number(30.0));
    }

    #[test]
    fn test_cross_sheet_nonexistent_returns_error() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let wb = Workbook::new();
        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval.evaluate_with_context("NoSheet!A1", sheet1, Some(&wb));
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_sheet_no_resolver_returns_ref_error() {
        use crate::cell::CellError;
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let wb = Workbook::new();
        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        // Without resolver, cross-sheet refs produce #REF!
        let result = eval
            .evaluate_with_context("Sheet2!A1", sheet1, None)
            .unwrap();
        assert_eq!(result, CellValue::Error(CellError::Ref));
    }

    #[test]
    fn test_cross_sheet_sum_range() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();
        wb.set_cell("Data", 0, 0, CellValue::Number(10.0)).unwrap();
        wb.set_cell("Data", 1, 0, CellValue::Number(20.0)).unwrap();
        wb.set_cell("Data", 2, 0, CellValue::Number(30.0)).unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval
            .evaluate_with_context("SUM(Data!A1:A3)", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Number(60.0));
    }

    #[test]
    fn test_cross_sheet_empty_cell() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_sheet("Sheet2").unwrap();
        // Sheet2!A1 is empty

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval
            .evaluate_with_context("Sheet2!A1", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Empty);
    }

    #[test]
    fn test_sheet_resolver_impl() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(99.0))
            .unwrap();

        let val = wb.resolve_cell("Sheet1", 0, 0).unwrap();
        assert_eq!(val, CellValue::Number(99.0));

        let val = wb.resolve_cell("Sheet1", 5, 5).unwrap();
        assert_eq!(val, CellValue::Empty);

        let err = wb.resolve_cell("NoSuchSheet", 0, 0);
        assert!(err.is_err());
    }

    // --- Named functions ---

    #[test]
    fn test_add_named_function() {
        let mut wb = Workbook::new();
        wb.add_named_function("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();
        let nf = wb.get_named_function("DOUBLE").unwrap();
        assert_eq!(nf.name, "DOUBLE");
        assert_eq!(nf.body, "x * 2");
    }

    #[test]
    fn test_remove_named_function() {
        let mut wb = Workbook::new();
        wb.add_named_function("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();
        wb.remove_named_function("DOUBLE").unwrap();
        assert!(wb.get_named_function("DOUBLE").is_none());
    }

    #[test]
    fn test_list_named_functions() {
        let mut wb = Workbook::new();
        wb.add_named_function("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();
        wb.add_named_function("TRIPLE", vec!["x".into()], "x * 3", None)
            .unwrap();
        assert_eq!(wb.list_named_functions().len(), 2);
    }

    #[test]
    fn test_named_function_formula_evaluation() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_named_function("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(5.0)).unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval
            .evaluate_with_context("DOUBLE(A1)", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Number(10.0));
    }

    #[test]
    fn test_named_function_multi_param() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_named_function(
            "ADDMUL",
            vec!["a".into(), "b".into(), "c".into()],
            "(a + b) * c",
            None,
        )
        .unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval
            .evaluate_with_context("ADDMUL(2, 3, 4)", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Number(20.0));
    }

    #[test]
    fn test_named_function_wrong_arg_count() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_named_function("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        let result = eval.evaluate_with_context("DOUBLE(1, 2)", sheet1, Some(&wb));
        assert!(result.is_err());
    }

    #[test]
    fn test_named_function_case_insensitive() {
        use crate::formula::FormulaEngine;
        use crate::formula::evaluator::SimpleEvaluator;

        let mut wb = Workbook::new();
        wb.add_named_function("MyFunc", vec!["x".into()], "x + 100", None)
            .unwrap();

        let eval = SimpleEvaluator;
        let sheet1 = wb.get_sheet("Sheet1").unwrap();
        // The tokenizer converts function names to uppercase, so test that
        let result = eval
            .evaluate_with_context("MYFUNC(5)", sheet1, Some(&wb))
            .unwrap();
        assert_eq!(result, CellValue::Number(105.0));
    }
}
