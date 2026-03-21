use thiserror::Error;

/// All errors produced by the lattice-core engine.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum LatticeError {
    /// A sheet with the given name was not found.
    #[error("sheet not found: {0}")]
    SheetNotFound(String),

    /// A sheet with the given name already exists.
    #[error("sheet already exists: {0}")]
    SheetAlreadyExists(String),

    /// Cannot remove the last remaining sheet.
    #[error("cannot remove the last sheet")]
    CannotRemoveLastSheet,

    /// Invalid cell reference string (e.g. bad A1 notation).
    #[error("invalid cell reference: {0}")]
    InvalidCellRef(String),

    /// Invalid range string.
    #[error("invalid range: {0}")]
    InvalidRange(String),

    /// A formula error during parsing or evaluation.
    #[error("formula error: {0}")]
    FormulaError(String),

    /// The undo stack is empty — nothing to undo.
    #[error("nothing to undo")]
    NothingToUndo,

    /// The redo stack is empty — nothing to redo.
    #[error("nothing to redo")]
    NothingToRedo,

    /// Generic internal error.
    #[error("{0}")]
    Internal(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, LatticeError>;
