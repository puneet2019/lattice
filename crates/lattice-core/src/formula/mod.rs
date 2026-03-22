pub mod dependency;
pub mod evaluator;
pub mod parser;
pub mod query;

use crate::cell::CellValue;
use crate::error::Result;
use crate::sheet::Sheet;

/// A resolver that provides read-only access to cells across sheets.
///
/// This is used by the formula evaluator to look up cross-sheet references
/// like `Sheet2!A1`. Implementations are typically provided by the `Workbook`.
pub trait SheetResolver {
    /// Look up a cell value by sheet name, row, and column.
    ///
    /// Returns `Ok(CellValue)` on success. Returns an error if the sheet is
    /// not found. Returns `CellValue::Empty` if the cell does not exist.
    fn resolve_cell(&self, sheet_name: &str, row: u32, col: u32) -> Result<CellValue>;
}

/// Trait that all formula-evaluation backends must implement.
pub trait FormulaEngine {
    /// Evaluate `formula` (without the leading `=`) in the context of `sheet`.
    /// Returns the computed [`CellValue`].
    fn evaluate(&self, formula: &str, sheet: &Sheet) -> Result<CellValue>;

    /// Evaluate `formula` with cross-sheet reference support.
    ///
    /// The `resolver` provides access to cells in other sheets. If `None`, any
    /// cross-sheet reference will return `#REF!`.
    ///
    /// The default implementation ignores the resolver and delegates to
    /// [`evaluate`](FormulaEngine::evaluate). Implementations should override
    /// this method to support cross-sheet references.
    fn evaluate_with_context(
        &self,
        formula: &str,
        sheet: &Sheet,
        resolver: Option<&dyn SheetResolver>,
    ) -> Result<CellValue> {
        let _ = resolver;
        self.evaluate(formula, sheet)
    }
}
