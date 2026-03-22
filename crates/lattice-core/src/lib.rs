//! `lattice-core` — the pure-Rust spreadsheet engine for Lattice.
//!
//! This crate contains the core data model (cells, sheets, workbooks),
//! formula evaluation, formatting, selection, clipboard, undo/redo,
//! sorting, and filtering logic. It has **no I/O**, **no UI**, and
//! **no async** dependencies.

pub mod cell;
pub mod clipboard;
pub mod error;
pub mod filter;
pub mod format;
pub mod formula;
pub mod history;
pub mod selection;
pub mod sheet;
pub mod sort;
pub mod workbook;

// Re-export key types at the crate root for ergonomic imports.
pub use cell::{Cell, CellError, CellValue};
pub use clipboard::{ClipboardContent, PasteMode};
pub use error::{LatticeError, Result};
pub use filter::{AutoFilter, ColumnFilter, FilterCondition};
pub use format::{CellFormat, HAlign, NumberFormat, VAlign};
pub use formula::FormulaEngine;
pub use history::{Operation, UndoStack};
pub use selection::{CellRef, Range, Selection, col_to_letter, parse_cell_ref};
pub use sheet::{MergedRegion, Sheet};
pub use sort::{SortDirection, SortKey};
pub use workbook::Workbook;
