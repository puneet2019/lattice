use serde::{Deserialize, Serialize};

use crate::cell::Cell;

/// Content stored in the internal clipboard after a copy or cut operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardContent {
    /// 2-D grid of cells that were copied (rows x cols).
    pub cells: Vec<Vec<Option<Cell>>>,
    /// Whether this was a *cut* (source cells should be cleared on paste).
    pub is_cut: bool,
}

impl ClipboardContent {
    /// Create a new empty clipboard entry.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            is_cut: false,
        }
    }
}

impl Default for ClipboardContent {
    fn default() -> Self {
        Self::new()
    }
}
