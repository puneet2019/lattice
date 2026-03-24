//! Dependency graph for formula recalculation.
//!
//! Tracks which cells depend on which other cells. Supports topological
//! sorting for correct recalculation order and cycle detection.

use std::collections::{HashMap, HashSet, VecDeque};

/// A list of cell coordinates (row, col).
type CellCoords = Vec<(u32, u32)>;

/// A dependency graph that tracks which cells depend on which other cells.
///
/// When cell X contains a formula referencing cell Y, we record that X depends
/// on Y. When Y changes, all dependents of Y must be recalculated.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Maps a cell `(row, col)` to the set of cells that depend on it
    /// (i.e., cells whose formulas reference this cell).
    dependents: HashMap<(u32, u32), HashSet<(u32, u32)>>,
    /// Maps a cell `(row, col)` to the set of cells it references (its
    /// precedents — the cells appearing in its formula).
    precedents: HashMap<(u32, u32), HashSet<(u32, u32)>>,
}

impl DependencyGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `cell` depends on each of `references`.
    ///
    /// Clears any previous dependencies for `cell` before setting the new ones.
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

    /// Return the set of cells that `cell` references (its precedents).
    pub fn get_precedents(&self, cell: &(u32, u32)) -> Option<&HashSet<(u32, u32)>> {
        self.precedents.get(cell)
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

    /// Return all cells that need to be recalculated when `changed_cell`
    /// changes, in correct topological order (leaves first, roots last).
    ///
    /// Returns `Err(vec_of_cycle_cells)` if a circular reference is detected.
    pub fn recalc_order(&self, changed_cell: &(u32, u32)) -> Result<CellCoords, CellCoords> {
        // Collect all transitively affected cells via BFS.
        let mut affected: HashSet<(u32, u32)> = HashSet::new();
        let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
        let mut has_cycle = false;

        if let Some(deps) = self.dependents.get(changed_cell) {
            for d in deps {
                queue.push_back(*d);
            }
        }

        while let Some(cell) = queue.pop_front() {
            // If BFS reaches back to the changed cell, there's a cycle.
            if cell == *changed_cell {
                has_cycle = true;
                continue;
            }
            if !affected.insert(cell) {
                continue; // Already visited
            }
            if let Some(deps) = self.dependents.get(&cell) {
                for d in deps {
                    if !affected.contains(d) {
                        queue.push_back(*d);
                    }
                }
            }
        }

        if has_cycle {
            let mut cycle_cells: Vec<(u32, u32)> = affected.into_iter().collect();
            cycle_cells.push(*changed_cell);
            cycle_cells.sort();
            return Err(cycle_cells);
        }

        if affected.is_empty() {
            return Ok(Vec::new());
        }

        // Topological sort (Kahn's algorithm) on the affected sub-graph.
        // We need: for each affected cell, how many of its precedents are also
        // in the affected set?
        let mut in_degree: HashMap<(u32, u32), usize> = HashMap::new();
        for &cell in &affected {
            let mut deg: usize = 0;
            if let Some(precs) = self.precedents.get(&cell) {
                for p in precs {
                    if affected.contains(p) || *p == *changed_cell {
                        deg += 1;
                    }
                }
            }
            // Also count the changed_cell itself as a "resolved" precedent
            // (it's already been updated).
            if let Some(precs) = self.precedents.get(&cell)
                && precs.contains(changed_cell)
            {
                deg = deg.saturating_sub(1);
            }
            in_degree.insert(cell, deg);
        }

        let mut sorted = Vec::new();
        let mut ready: VecDeque<(u32, u32)> = in_degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(&c, _)| c)
            .collect();

        while let Some(cell) = ready.pop_front() {
            sorted.push(cell);
            if let Some(deps) = self.dependents.get(&cell) {
                for d in deps {
                    if let Some(deg) = in_degree.get_mut(d) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            ready.push_back(*d);
                        }
                    }
                }
            }
        }

        if sorted.len() < affected.len() {
            // Cycle detected — return the cells involved in the cycle.
            let cycle_cells: Vec<(u32, u32)> = affected
                .iter()
                .filter(|c| !sorted.contains(c))
                .copied()
                .collect();
            return Err(cycle_cells);
        }

        Ok(sorted)
    }

    /// Detect whether adding `cell -> references` would create a cycle.
    ///
    /// Returns `true` if a cycle would be created.
    pub fn would_create_cycle(&self, cell: (u32, u32), references: &[(u32, u32)]) -> bool {
        // Check if any reference transitively depends on `cell`.
        for &r in references {
            if r == cell {
                return true; // Self-reference
            }
            // BFS from `r` through dependents to see if we reach `cell`
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(r);
            while let Some(current) = queue.pop_front() {
                if current == cell {
                    // Wait — we're checking if `r` *depends on* `cell`...
                    // Actually we need to check if `cell` depends on any ref
                    // transitively. That means: does `r` have `cell` as a
                    // transitive dependent?
                    // No — we need: does `cell` appear in `r`'s transitive
                    // precedents? i.e., can we reach `cell` by following
                    // precedent links from `r`?
                }
                if !visited.insert(current) {
                    continue;
                }
                if let Some(precs) = self.precedents.get(&current) {
                    for &p in precs {
                        if p == cell {
                            return true; // cycle!
                        }
                        queue.push_back(p);
                    }
                }
            }
        }
        false
    }

    /// Return the total number of cells tracked in the graph.
    pub fn cell_count(&self) -> usize {
        let mut cells: HashSet<(u32, u32)> = HashSet::new();
        cells.extend(self.precedents.keys());
        cells.extend(self.dependents.keys());
        cells.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_dependents() {
        let mut graph = DependencyGraph::new();
        // B1 depends on A1
        graph.set_dependencies((0, 1), &[(0, 0)]);

        let deps = graph.get_dependents(&(0, 0)).unwrap();
        assert!(deps.contains(&(0, 1)));
    }

    #[test]
    fn test_remove_cell() {
        let mut graph = DependencyGraph::new();
        graph.set_dependencies((0, 1), &[(0, 0)]);
        graph.remove_cell(&(0, 1));

        assert!(
            graph.get_dependents(&(0, 0)).is_none()
                || graph.get_dependents(&(0, 0)).unwrap().is_empty()
        );
    }

    #[test]
    fn test_recalc_order_simple() {
        let mut graph = DependencyGraph::new();
        // B1 = A1 + 1
        graph.set_dependencies((0, 1), &[(0, 0)]);
        // C1 = B1 + 1
        graph.set_dependencies((0, 2), &[(0, 1)]);

        let order = graph.recalc_order(&(0, 0)).unwrap();
        assert_eq!(order.len(), 2);
        // B1 should come before C1
        let b1_pos = order.iter().position(|c| *c == (0, 1)).unwrap();
        let c1_pos = order.iter().position(|c| *c == (0, 2)).unwrap();
        assert!(b1_pos < c1_pos);
    }

    #[test]
    fn test_recalc_order_cycle_detection() {
        let mut graph = DependencyGraph::new();
        // A1 = B1
        graph.set_dependencies((0, 0), &[(0, 1)]);
        // B1 = A1 (circular!)
        graph.set_dependencies((0, 1), &[(0, 0)]);

        let result = graph.recalc_order(&(0, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_would_create_cycle() {
        let mut graph = DependencyGraph::new();
        // A1 = B1
        graph.set_dependencies((0, 0), &[(0, 1)]);

        // Would B1 = A1 create a cycle? Yes.
        assert!(graph.would_create_cycle((0, 1), &[(0, 0)]));
    }

    #[test]
    fn test_would_not_create_cycle() {
        let mut graph = DependencyGraph::new();
        // A1 = B1
        graph.set_dependencies((0, 0), &[(0, 1)]);

        // Would C1 = B1 create a cycle? No.
        assert!(!graph.would_create_cycle((0, 2), &[(0, 1)]));
    }

    #[test]
    fn test_recalc_order_diamond() {
        let mut graph = DependencyGraph::new();
        // B1 = A1
        graph.set_dependencies((0, 1), &[(0, 0)]);
        // C1 = A1
        graph.set_dependencies((0, 2), &[(0, 0)]);
        // D1 = B1 + C1
        graph.set_dependencies((0, 3), &[(0, 1), (0, 2)]);

        let order = graph.recalc_order(&(0, 0)).unwrap();
        assert_eq!(order.len(), 3);
        // D1 must come after both B1 and C1
        let d1_pos = order.iter().position(|c| *c == (0, 3)).unwrap();
        let b1_pos = order.iter().position(|c| *c == (0, 1)).unwrap();
        let c1_pos = order.iter().position(|c| *c == (0, 2)).unwrap();
        assert!(d1_pos > b1_pos);
        assert!(d1_pos > c1_pos);
    }

    #[test]
    fn test_update_dependencies() {
        let mut graph = DependencyGraph::new();
        // B1 = A1
        graph.set_dependencies((0, 1), &[(0, 0)]);
        assert!(graph.get_dependents(&(0, 0)).unwrap().contains(&(0, 1)));

        // Update: B1 = C1 (no longer depends on A1)
        graph.set_dependencies((0, 1), &[(0, 2)]);
        // A1 should no longer have B1 as dependent
        assert!(
            graph.get_dependents(&(0, 0)).is_none()
                || !graph.get_dependents(&(0, 0)).unwrap().contains(&(0, 1))
        );
        // C1 should now have B1 as dependent
        assert!(graph.get_dependents(&(0, 2)).unwrap().contains(&(0, 1)));
    }

    #[test]
    fn test_self_reference_cycle() {
        let graph = DependencyGraph::new();
        // Would A1 = A1 create a cycle? Yes.
        assert!(graph.would_create_cycle((0, 0), &[(0, 0)]));
    }

    #[test]
    fn test_cell_count() {
        let mut graph = DependencyGraph::new();
        graph.set_dependencies((0, 1), &[(0, 0)]);
        assert!(graph.cell_count() >= 2);
    }
}
