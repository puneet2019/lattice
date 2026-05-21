//! In-memory chart store, ported from `src-tauri/src/commands/chart.rs`.
//!
//! WASM is single-threaded, so unlike the desktop `ChartStore` (which uses a
//! `Mutex<HashMap>`), this store is a plain `HashMap` owned by the single
//! `AppState`. It lives in a `RefCell` at the `AppState` level.

use std::collections::HashMap;

use lattice_charts::Chart;

/// In-app chart store. Charts live here until they are persisted to a file.
#[derive(Default)]
pub struct ChartStore {
    /// Chart definitions keyed by chart ID.
    pub charts: HashMap<String, Chart>,
}

impl ChartStore {
    /// Create an empty chart store.
    pub fn new() -> Self {
        Self {
            charts: HashMap::new(),
        }
    }

    /// Insert a chart into the store.
    pub fn insert(&mut self, id: String, chart: Chart) {
        self.charts.insert(id, chart);
    }
}
