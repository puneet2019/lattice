use std::collections::{HashMap, HashSet};

/// A dependency graph that tracks which cells depend on which other cells.
///
/// When cell X contains a formula referencing cell Y, we record that X depends
/// on Y. When Y changes, all dependents of Y must be recalculated.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Maps a cell `(row, col)` to the set of cells that depend on it.
    dependents: HashMap<(u32, u32), HashSet<(u32, u32)>>,
    /// Maps a cell `(row, col)` to the set of cells it references (its
    /// precedents).
    precedents: HashMap<(u32, u32), HashSet<(u32, u32)>>,
}

impl DependencyGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `cell` depends on each of `references`.
    pub fn set_dependencies(&mut self, cell: (u32, u32), references: &[(u32, u32)]) {
        // Clear old precedents for this cell.
        if let Some(old_refs) = self.precedents.remove(&cell) {
            for r in &old_refs {
                if let Some(deps) = self.dependents.get_mut(r) {
                    deps.remove(&cell);
                }
            }
        }

        let ref_set: HashSet<(u32, u32)> = references.iter().copied().collect();
        for &r in &ref_set {
            self.dependents.entry(r).or_default().insert(cell);
        }
        self.precedents.insert(cell, ref_set);
    }

    /// Return the set of cells that directly depend on `cell`.
    pub fn get_dependents(&self, cell: &(u32, u32)) -> Option<&HashSet<(u32, u32)>> {
        self.dependents.get(cell)
    }

    /// Clear all dependency information for a cell (e.g. when the cell is
    /// cleared or its formula removed).
    pub fn remove_cell(&mut self, cell: &(u32, u32)) {
        if let Some(refs) = self.precedents.remove(cell) {
            for r in &refs {
                if let Some(deps) = self.dependents.get_mut(r) {
                    deps.remove(cell);
                }
            }
        }
        self.dependents.remove(cell);
    }
}
