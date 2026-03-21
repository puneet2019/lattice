pub mod dependency;
pub mod evaluator;
pub mod parser;

use crate::cell::CellValue;
use crate::error::Result;
use crate::sheet::Sheet;

/// Trait that all formula-evaluation backends must implement.
pub trait FormulaEngine {
    /// Evaluate `formula` (without the leading `=`) in the context of `sheet`.
    /// Returns the computed [`CellValue`].
    fn evaluate(&self, formula: &str, sheet: &Sheet) -> Result<CellValue>;
}
