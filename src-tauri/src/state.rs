use std::sync::Arc;

use lattice_core::{UndoStack, Workbook};
use tokio::sync::RwLock;

/// Shared application state accessible by all Tauri commands.
pub struct AppState {
    /// The current workbook, protected by an async-aware read-write lock.
    pub workbook: Arc<RwLock<Workbook>>,
    /// Undo/redo stack for the workbook.
    pub undo_stack: Arc<RwLock<UndoStack>>,
}

impl AppState {
    /// Create a new `AppState` with a default empty workbook.
    pub fn new() -> Self {
        Self {
            workbook: Arc::new(RwLock::new(Workbook::new())),
            undo_stack: Arc::new(RwLock::new(UndoStack::new(1000))),
        }
    }

    /// Replace the current workbook with a new one and reset the undo stack.
    pub async fn replace_workbook(&self, wb: Workbook) {
        let mut workbook = self.workbook.write().await;
        *workbook = wb;
        let mut stack = self.undo_stack.write().await;
        *stack = UndoStack::new(1000);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
