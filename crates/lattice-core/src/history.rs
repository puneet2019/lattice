use crate::cell::CellValue;

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
}
