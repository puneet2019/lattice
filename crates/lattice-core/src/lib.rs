//! `lattice-core` — the pure-Rust spreadsheet engine for Lattice.
//!
//! This crate contains the core data model (cells, sheets, workbooks),
//! formula evaluation, formatting, selection, clipboard, undo/redo,
//! sorting, and filtering logic. It has **no I/O**, **no UI**, and
//! **no async** dependencies.

pub mod autofill;
pub mod autosave;
pub mod cell;
pub mod clipboard;
pub mod conditional_format;
pub mod error;
pub mod filter;
pub mod find_replace;
pub mod format;
pub mod formula;
pub mod named_range;
pub mod history;
pub mod pivot;
pub mod selection;
pub mod sheet;
pub mod sort;
pub mod validation;
pub mod workbook;

// Re-export key types at the crate root for ergonomic imports.
pub use autofill::{FillDirection, FillPattern, detect_pattern, fill_range};
pub use autosave::AutoSaveConfig;
pub use cell::{Cell, CellError, CellValue, DropdownConfig};
pub use clipboard::{ClipboardContent, PasteMode};
pub use conditional_format::{
    ComparisonOperator, ConditionalFormatRange, ConditionalFormatStore, ConditionalRule,
    ConditionalRuleType, ConditionalStyle,
};
pub use error::{LatticeError, Result};
pub use filter::{AutoFilter, ColumnFilter, FilterCondition};
pub use find_replace::{FindOptions, MatchLocation, ReplaceResult};
pub use format::{
    Border, BorderStyle, CellBorders, CellFormat, HAlign, NumberFormat, TextWrap, VAlign,
    format_value,
};
pub use named_range::{NamedRange, NamedRangeStore};
pub use formula::{FormulaEngine, SheetResolver};
pub use pivot::{PivotAggregation, PivotConfig, PivotResult, PivotValue, generate_pivot};
pub use history::{Operation, UndoStack};
pub use selection::{CellRef, Range, Selection, col_to_letter, parse_cell_ref};
pub use sheet::{BandedRows, MergedRegion, ProtectedRange, Sheet, SheetProtection};
pub use sort::{SortDirection, SortKey};
pub use validation::{ValidationRule, ValidationStore, ValidationType};
pub use workbook::Workbook;
