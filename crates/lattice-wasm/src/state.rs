//! Synchronous application state for the WASM build.
//!
//! This mirrors `src-tauri/src/state.rs` (`AppState`) but is **sync**: WASM
//! runs single-threaded in a browser tab, so there is no need for
//! `Arc<RwLock>` / `tokio`. All fields are owned directly and the whole
//! `AppState` lives behind a single `thread_local! RefCell`.

use std::collections::HashMap;

use lattice_core::{AutoSaveConfig, ConditionalFormatStore, UndoStack, Workbook};
use lattice_mcp::tools::ToolRegistry;

use crate::chart_store::ChartStore;

/// Shared application state for all WASM dispatch commands.
pub struct AppState {
    /// The current workbook.
    pub workbook: Workbook,
    /// Undo/redo stack for the workbook.
    pub undo_stack: UndoStack,
    /// In-memory chart definitions.
    pub chart_store: ChartStore,
    /// Auto-save configuration (browser autosave targets OPFS, handled by the
    /// frontend bridge; the engine just keeps the config).
    ///
    /// Not read by any `invoke` command — autosave is driven entirely by the
    /// JS bridge in the browser build — but kept so the field set mirrors the
    /// desktop `AppState`.
    #[allow(dead_code)]
    pub autosave_config: AutoSaveConfig,
    /// Path / handle name of the currently open file (None for unsaved
    /// workbooks). In the browser this is informational only.
    pub file_path: Option<String>,
    /// Conditional formatting rules.
    pub conditional_formats: ConditionalFormatStore,
    /// Per-chart stacked flag (chart_id -> stacked bool).
    pub chart_stacked: HashMap<String, bool>,
    /// Active column filters per sheet: sheet_name -> (column_index -> allowed_values).
    ///
    /// Filters accumulate across columns and are ANDed together: a row is
    /// visible only if it passes every column filter.
    pub active_filters: HashMap<String, HashMap<u32, Vec<String>>>,
    /// Whether the in-page MCP server has been initialized by a client.
    /// Lives here (not as a function-local) so the MCP `initialize` handshake
    /// is durable across `mcp_request()` calls.
    pub mcp_initialized: bool,
    /// MCP tool registry built once at startup; borrowed into every
    /// `McpState` for `tools/list` and `tools/call` dispatch.
    pub mcp_tools: ToolRegistry,
}

impl AppState {
    /// Create a new `AppState` with a default empty workbook.
    pub fn new() -> Self {
        Self {
            workbook: Workbook::new(),
            undo_stack: UndoStack::new(1000),
            chart_store: ChartStore::new(),
            autosave_config: AutoSaveConfig::default(),
            file_path: None,
            conditional_formats: ConditionalFormatStore::new(),
            chart_stacked: HashMap::new(),
            active_filters: HashMap::new(),
            mcp_initialized: false,
            mcp_tools: ToolRegistry::default_registry(),
        }
    }

    /// Replace the current workbook with a new one and reset the undo stack.
    ///
    /// Mirrors `AppState::replace_workbook` in the desktop app.
    pub fn replace_workbook(&mut self, wb: Workbook) {
        self.workbook = wb;
        self.undo_stack = UndoStack::new(1000);
        self.chart_store = ChartStore::new();
        self.chart_stacked.clear();
        self.active_filters.clear();
        self.conditional_formats = ConditionalFormatStore::new();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
