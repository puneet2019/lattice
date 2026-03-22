use crate::cell::{Cell, CellValue};
use crate::format::CellFormat;

/// A reversible operation recorded in the undo/redo stack.
#[derive(Debug, Clone)]
pub enum Operation {
    /// A cell value was changed.
    SetCell {
        sheet: String,
        row: u32,
        col: u32,
        old_value: CellValue,
        new_value: CellValue,
    },
    /// A sheet was added.
    AddSheet { name: String },
    /// A sheet was removed (with its data snapshot).
    RemoveSheet { name: String },
    /// A sheet was renamed.
    RenameSheet { old_name: String, new_name: String },
    /// Cell formatting was changed.
    FormatCells {
        sheet: String,
        /// Each entry stores (row, col, old_format, new_format).
        cells: Vec<(u32, u32, CellFormat, CellFormat)>,
    },
    /// Rows were inserted.
    InsertRows {
        sheet: String,
        row: u32,
        count: u32,
    },
    /// Rows were deleted (with the deleted cell data for undo).
    DeleteRows {
        sheet: String,
        row: u32,
        count: u32,
        /// Deleted cells: Vec<(row, col, Cell)>.
        deleted_cells: Vec<(u32, u32, Cell)>,
    },
    /// Columns were inserted.
    InsertCols {
        sheet: String,
        col: u32,
        count: u32,
    },
    /// Columns were deleted (with the deleted cell data for undo).
    DeleteCols {
        sheet: String,
        col: u32,
        count: u32,
        /// Deleted cells: Vec<(row, col, Cell)>.
        deleted_cells: Vec<(u32, u32, Cell)>,
    },
}

/// A fixed-capacity undo/redo stack.
#[derive(Debug, Clone)]
pub struct UndoStack {
    undo: Vec<Operation>,
    redo: Vec<Operation>,
    capacity: usize,
}

impl UndoStack {
    /// Create a new stack with the given maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            capacity,
        }
    }

    /// Push a new operation. Clears the redo stack (because the timeline has
    /// diverged). If the stack exceeds capacity, the oldest entry is dropped.
    pub fn push(&mut self, op: Operation) {
        self.redo.clear();
        if self.undo.len() >= self.capacity {
            self.undo.remove(0);
        }
        self.undo.push(op);
    }

    /// Pop the most recent operation for undoing. Returns `None` if empty.
    pub fn undo(&mut self) -> Option<Operation> {
        let op = self.undo.pop()?;
        self.redo.push(op.clone());
        Some(op)
    }

    /// Pop the most recent redo operation. Returns `None` if empty.
    pub fn redo(&mut self) -> Option<Operation> {
        let op = self.redo.pop()?;
        self.undo.push(op.clone());
        Some(op)
    }

    /// Number of operations available to undo.
    pub fn undo_count(&self) -> usize {
        self.undo.len()
    }

    /// Number of operations available to redo.
    pub fn redo_count(&self) -> usize {
        self.redo.len()
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_undo_redo() {
        let mut stack = UndoStack::new(100);

        stack.push(Operation::SetCell {
            sheet: "Sheet1".into(),
            row: 0,
            col: 0,
            old_value: CellValue::Empty,
            new_value: CellValue::Number(1.0),
        });

        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 0);

        let op = stack.undo().unwrap();
        assert!(matches!(op, Operation::SetCell { .. }));
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 1);

        let op = stack.redo().unwrap();
        assert!(matches!(op, Operation::SetCell { .. }));
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn test_push_clears_redo() {
        let mut stack = UndoStack::new(100);
        stack.push(Operation::AddSheet { name: "A".into() });
        stack.undo();
        assert_eq!(stack.redo_count(), 1);

        // A new push should clear redo.
        stack.push(Operation::AddSheet { name: "B".into() });
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn test_capacity() {
        let mut stack = UndoStack::new(2);
        stack.push(Operation::AddSheet { name: "A".into() });
        stack.push(Operation::AddSheet { name: "B".into() });
        stack.push(Operation::AddSheet { name: "C".into() });
        assert_eq!(stack.undo_count(), 2);
    }

    #[test]
    fn test_format_cells_operation() {
        use crate::format::CellFormat;
        let mut stack = UndoStack::new(100);

        let old_fmt = CellFormat::default();
        let mut new_fmt = CellFormat::default();
        new_fmt.bold = true;

        stack.push(Operation::FormatCells {
            sheet: "Sheet1".into(),
            cells: vec![(0, 0, old_fmt.clone(), new_fmt.clone())],
        });

        assert_eq!(stack.undo_count(), 1);
        let op = stack.undo().unwrap();
        assert!(matches!(op, Operation::FormatCells { .. }));
    }

    #[test]
    fn test_insert_rows_operation() {
        let mut stack = UndoStack::new(100);
        stack.push(Operation::InsertRows {
            sheet: "Sheet1".into(),
            row: 5,
            count: 3,
        });
        assert_eq!(stack.undo_count(), 1);
        let op = stack.undo().unwrap();
        assert!(matches!(op, Operation::InsertRows { row: 5, count: 3, .. }));
    }

    #[test]
    fn test_delete_rows_operation() {
        use crate::cell::Cell;
        let mut stack = UndoStack::new(100);
        let cell = Cell::default();
        stack.push(Operation::DeleteRows {
            sheet: "Sheet1".into(),
            row: 2,
            count: 1,
            deleted_cells: vec![(2, 0, cell)],
        });
        assert_eq!(stack.undo_count(), 1);
        let op = stack.undo().unwrap();
        assert!(matches!(op, Operation::DeleteRows { row: 2, count: 1, .. }));
    }

    #[test]
    fn test_insert_cols_operation() {
        let mut stack = UndoStack::new(100);
        stack.push(Operation::InsertCols {
            sheet: "Sheet1".into(),
            col: 3,
            count: 2,
        });
        let op = stack.undo().unwrap();
        assert!(matches!(op, Operation::InsertCols { col: 3, count: 2, .. }));
    }

    #[test]
    fn test_delete_cols_operation() {
        use crate::cell::Cell;
        let mut stack = UndoStack::new(100);
        stack.push(Operation::DeleteCols {
            sheet: "Sheet1".into(),
            col: 1,
            count: 1,
            deleted_cells: vec![(0, 1, Cell::default())],
        });
        let op = stack.undo().unwrap();
        assert!(matches!(op, Operation::DeleteCols { col: 1, count: 1, .. }));
    }
}
