//! WebAssembly bindings for the Lattice spreadsheet engine.
//!
//! This crate compiles the Lattice engine (`lattice-core`), file I/O
//! (`lattice-io`), charts (`lattice-charts`), and analysis
//! (`lattice-analysis`) to `wasm32-unknown-unknown` so the whole
//! spreadsheet can run inside a browser tab with no backend process.
//!
//! # Architecture
//!
//! The browser frontend talks to the engine through a single
//! [`invoke`] function that mirrors Tauri's `invoke(command, args)`
//! protocol. The frontend's `bridge/tauri.ts` is the source of truth for
//! the command names and argument shapes; every command it can send is
//! handled here by a faithful, **synchronous** port of the corresponding
//! Tauri command in `src-tauri/src/commands/*.rs`.
//!
//! # Usage
//!
//! ```js
//! import init, { invoke, open_xlsx, save_xlsx } from './pkg/lattice_wasm.js';
//!
//! await init();          // load the .wasm module
//! init_engine();         // MUST be called once before any invoke()
//! const cell = JSON.parse(invoke('get_cell', JSON.stringify({ sheet: 'Sheet1', row: 0, col: 0 })));
//! ```
//!
//! [`init`] installs the panic hook; it must be called exactly once
//! before the first [`invoke`].

mod cell_parse;
mod chart_store;
mod commands;
mod state;

use std::cell::RefCell;

use serde_json::Value;
use wasm_bindgen::prelude::*;

use lattice_core::Workbook;

use crate::state::AppState;

thread_local! {
    /// The single, process-wide application state.
    ///
    /// WASM in a browser tab is single-threaded, so a `thread_local!`
    /// `RefCell` is sufficient — there is no contention and no need for
    /// `Arc`/`Mutex`/`RwLock` (unlike the desktop `AppState`).
    static STATE: RefCell<AppState> = RefCell::new(AppState::new());
}

/// Install the panic hook so Rust panics surface as readable JS console errors.
///
/// **Must be called exactly once** before the first [`invoke`]. Calling it
/// more than once is harmless (`set_once` is idempotent).
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Dispatch a command, mirroring Tauri's `invoke(command, args)` protocol.
///
/// `command` is the command name string (e.g. `"get_cell"`). `args_json`
/// is a JSON object string of the command's arguments using **camelCase**
/// keys, exactly as the frontend's `bridge/tauri.ts` sends them.
///
/// On success the result is returned as a JSON string (`"null"` for
/// void commands). On failure a [`JsError`] carrying the error message is
/// returned, which surfaces as a rejected promise on the JS side.
///
/// [`init`] must have been called first.
#[wasm_bindgen]
pub fn invoke(command: &str, args_json: &str) -> Result<String, JsError> {
    // Parse the arguments. An empty / missing payload is treated as `{}`
    // so commands with no parameters (e.g. `list_sheets`) can be called
    // with `invoke('list_sheets', '')`.
    let args: Value = if args_json.trim().is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_str(args_json)
            .map_err(|e| JsError::new(&format!("invalid args JSON: {e}")))?
    };

    let result = STATE.with(|state| {
        let mut state = state.borrow_mut();
        dispatch(&mut state, command, args)
    });

    match result {
        Ok(value) => serde_json::to_string(&value).map_err(|e| JsError::new(&e.to_string())),
        Err(msg) => Err(JsError::new(&msg)),
    }
}

/// Route a command name to its handler.
///
/// Every command string the frontend's `bridge/tauri.ts` can pass to
/// `invoke(...)` is covered here. Handlers return `serde_json::Value`
/// (`Value::Null` for void) or an error string.
fn dispatch(state: &mut AppState, command: &str, args: Value) -> Result<Value, String> {
    use commands as c;
    match command {
        // --- Cell commands ---
        "get_cell" => c::get_cell(state, args),
        "set_cell" => c::set_cell(state, args),
        "get_range" => c::get_range(state, args),
        // --- Comment commands ---
        "set_comment" => c::set_comment(state, args),
        "get_comment" => c::get_comment(state, args),
        "remove_comment" => c::remove_comment(state, args),
        // --- Data cleanup ---
        "remove_duplicates" => c::remove_duplicates(state, args),
        "text_to_columns" => c::text_to_columns(state, args),
        // --- Format commands ---
        "format_cells" => c::format_cells(state, args),
        "merge_cells" => c::merge_cells(state, args),
        "unmerge_cells" => c::unmerge_cells(state, args),
        "get_merged_regions" => c::get_merged_regions(state, args),
        "set_banded_rows" => c::set_banded_rows(state, args),
        "get_banded_rows" => c::get_banded_rows(state, args),
        // --- Row / column manipulation ---
        "insert_rows" => c::insert_rows(state, args),
        "delete_rows" => c::delete_rows(state, args),
        "insert_cols" => c::insert_cols(state, args),
        "delete_cols" => c::delete_cols(state, args),
        // --- Column / row sizing ---
        "set_col_width" => c::set_col_width(state, args),
        "set_row_height" => c::set_row_height(state, args),
        "get_col_widths" => c::get_col_widths(state, args),
        "get_row_heights" => c::get_row_heights(state, args),
        // --- Sheet tab color / reorder ---
        "set_sheet_tab_color" => c::set_sheet_tab_color(state, args),
        "move_sheet" => move_sheet_stub(),
        // --- Search ---
        "find_in_sheet" => c::find_in_sheet(state, args),
        // --- Sheet commands ---
        "list_sheets" => c::list_sheets(state, args),
        "add_sheet" => c::add_sheet(state, args),
        "rename_sheet" => c::rename_sheet(state, args),
        "delete_sheet" => c::delete_sheet(state, args),
        "set_active_sheet" => c::set_active_sheet(state, args),
        "duplicate_sheet" => c::duplicate_sheet(state, args),
        // --- File commands ---
        // Path-based open/save are handled out-of-band by the JS bridge
        // (it reads the file into bytes and calls `open_xlsx` / `save_xlsx`).
        "open_file" => path_io_stub("open_file", "open_xlsx"),
        "open_csv" => path_io_stub("open_csv", "open_csv (bytes)"),
        "open_tsv" => path_io_stub("open_tsv", "open_tsv (bytes)"),
        "save_file" => path_io_stub("save_file", "save_xlsx"),
        "export_csv" => path_io_stub("export_csv", "export_csv_bytes"),
        "export_tsv" => path_io_stub("export_tsv", "export_tsv_bytes"),
        "new_workbook" => c::new_workbook(state, args),
        "export_html" => c::export_html(state, args),
        // --- Recent files ---
        "get_recent_files" => c::get_recent_files(state, args),
        "add_recent_file" => c::add_recent_file(state, args),
        // --- Edit commands ---
        "undo" => c::undo(state, args),
        "redo" => c::redo(state, args),
        // --- Chart commands ---
        "create_chart" => c::create_chart(state, args),
        "render_chart_svg" => c::render_chart_svg(state, args),
        "list_charts" => c::list_charts(state, args),
        "delete_chart" => c::delete_chart(state, args),
        "get_chart_config" => c::get_chart_config(state, args),
        "update_chart" => c::update_chart(state, args),
        // --- Validation commands ---
        "set_validation" => c::set_validation(state, args),
        "get_validation" => c::get_validation(state, args),
        "remove_validation" => c::remove_validation(state, args),
        "list_validations" => c::list_validations(state, args),
        // --- Filter commands ---
        "set_auto_filter" => c::set_auto_filter(state, args),
        "get_column_values" => c::get_column_values(state, args),
        "apply_column_filter" => c::apply_column_filter(state, args),
        "clear_filter" => c::clear_filter(state, args),
        "get_filter_info" => c::get_filter_info(state, args),
        "get_hidden_rows" => c::get_hidden_rows(state, args),
        "hide_rows" => c::hide_rows(state, args),
        "unhide_rows" => c::unhide_rows(state, args),
        "hide_cols" => c::hide_cols(state, args),
        "unhide_cols" => c::unhide_cols(state, args),
        "get_hidden_cols" => c::get_hidden_cols(state, args),
        // --- Sort ---
        "sort_range" => c::sort_range(state, args),
        // --- Row group commands ---
        "add_row_group" => c::add_row_group(state, args),
        "remove_row_group" => c::remove_row_group(state, args),
        "toggle_row_group" => c::toggle_row_group(state, args),
        "get_row_groups" => c::get_row_groups(state, args),
        // --- Named range commands ---
        "add_named_range" => c::add_named_range(state, args),
        "list_named_ranges" => c::list_named_ranges(state, args),
        "remove_named_range" => c::remove_named_range(state, args),
        "resolve_named_range" => c::resolve_named_range(state, args),
        // --- Conditional format commands ---
        "add_conditional_format" => c::add_conditional_format(state, args),
        "list_conditional_formats" => c::list_conditional_formats(state, args),
        "remove_conditional_format" => c::remove_conditional_format(state, args),
        // --- Named function commands ---
        "add_named_function" => c::add_named_function(state, args),
        "remove_named_function" => c::remove_named_function(state, args),
        "list_named_functions" => c::list_named_functions(state, args),
        // --- Filter view commands ---
        "save_filter_view" => c::save_filter_view(state, args),
        "list_filter_views" => c::list_filter_views(state, args),
        "apply_filter_view" => c::apply_filter_view(state, args),
        "delete_filter_view" => c::delete_filter_view(state, args),
        // --- Pivot table commands ---
        "get_sheet_headers" => c::get_sheet_headers(state, args),
        "create_pivot_table" => c::create_pivot_table(state, args),
        // --- Print commands ---
        "export_print_html" => c::export_print_html(state, args),
        // --- Column statistics ---
        "get_column_stats" => c::get_column_stats(state, args),
        // --- Version history commands ---
        // Versioning is filesystem-backed on the desktop; the browser has
        // no version store. These are stubs (see the report).
        "save_version" => version_stub("save_version"),
        "list_versions" => Ok(Value::Array(vec![])),
        "restore_version" => version_stub("restore_version"),
        // --- Unknown ---
        other => Err(format!("unknown command: {other}")),
    }
}

/// `move_sheet` cannot be implemented: `Workbook.sheets` is a private
/// `IndexMap` with no public reorder API. See the implementation report.
fn move_sheet_stub() -> Result<Value, String> {
    Err(
        "move_sheet is not supported in the WASM build: Workbook has no \
         public sheet-reorder API (the `sheets` IndexMap is private)"
            .to_string(),
    )
}

/// Path-based file commands are not callable through `invoke` in the
/// browser. The JS bridge must use the byte-based functions instead.
fn path_io_stub(command: &str, replacement: &str) -> Result<Value, String> {
    Err(format!(
        "'{command}' is not available in the browser build (no filesystem); \
         use the byte-based `{replacement}` export instead"
    ))
}

/// Version history is filesystem-backed and unavailable in the browser.
fn version_stub(command: &str) -> Result<Value, String> {
    Err(format!(
        "'{command}' is not available in the browser build: version history \
         requires a filesystem-backed store"
    ))
}

// ===========================================================================
// Byte-based file functions
//
// The browser has no filesystem, so file open/save go through raw bytes:
// JS reads a File into a Uint8Array, passes it here; for saving, JS takes
// the returned Vec<u8> and triggers a download.
// ===========================================================================

/// Open an `.xlsx` workbook from raw bytes, replacing the current workbook.
///
/// Also imports any embedded charts (non-fatal on failure). Returns the
/// `WorkbookInfo` (`{ sheets, active_sheet }`) as a JSON string.
#[wasm_bindgen]
pub fn open_xlsx(bytes: &[u8]) -> Result<String, JsError> {
    let wb = lattice_io::read_xlsx_from_bytes(bytes).map_err(|e| JsError::new(&e.to_string()))?;
    let info = load_workbook(wb);

    // Import embedded charts. Non-fatal: a chart that fails to parse is skipped.
    if let Ok(imported) = lattice_io::read_xlsx_charts_from_bytes(bytes) {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            for ic in imported {
                let chart_id = uuid::Uuid::new_v4().to_string();
                let mut chart =
                    lattice_charts::Chart::new(&chart_id, ic.chart_type, "", &ic.sheet_name);
                if let Some(t) = ic.title {
                    chart = chart.with_title(t);
                }
                state.chart_store.insert(chart_id, chart);
            }
        });
    }

    serde_json::to_string(&info).map_err(|e| JsError::new(&e.to_string()))
}

/// Open a CSV workbook from raw bytes, replacing the current workbook.
///
/// Returns the `WorkbookInfo` as a JSON string.
#[wasm_bindgen]
pub fn open_csv(bytes: &[u8]) -> Result<String, JsError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| JsError::new(&format!("CSV is not valid UTF-8: {e}")))?;
    let wb = lattice_io::read_csv_str(text, "Sheet1").map_err(|e| JsError::new(&e.to_string()))?;
    let info = load_workbook(wb);
    serde_json::to_string(&info).map_err(|e| JsError::new(&e.to_string()))
}

/// Open a TSV workbook from raw bytes, replacing the current workbook.
///
/// Returns the `WorkbookInfo` as a JSON string.
#[wasm_bindgen]
pub fn open_tsv(bytes: &[u8]) -> Result<String, JsError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| JsError::new(&format!("TSV is not valid UTF-8: {e}")))?;
    let wb = lattice_io::read_tsv_str(text, "Sheet1").map_err(|e| JsError::new(&e.to_string()))?;
    let info = load_workbook(wb);
    serde_json::to_string(&info).map_err(|e| JsError::new(&e.to_string()))
}

/// Serialize the current workbook to `.xlsx` bytes for download.
#[wasm_bindgen]
pub fn save_xlsx() -> Result<Vec<u8>, JsError> {
    STATE.with(|state| {
        let state = state.borrow();
        lattice_io::write_xlsx_to_buffer(&state.workbook).map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Serialize a sheet of the current workbook to CSV bytes for download.
///
/// An empty `sheet` string exports the active sheet.
#[wasm_bindgen]
pub fn export_csv_bytes(sheet: &str) -> Result<Vec<u8>, JsError> {
    STATE.with(|state| {
        let state = state.borrow();
        let sheet_name = if sheet.is_empty() { None } else { Some(sheet) };
        lattice_io::write_csv_string(&state.workbook, sheet_name)
            .map(|s| s.into_bytes())
            .map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Serialize a sheet of the current workbook to TSV bytes for download.
///
/// An empty `sheet` string exports the active sheet.
#[wasm_bindgen]
pub fn export_tsv_bytes(sheet: &str) -> Result<Vec<u8>, JsError> {
    STATE.with(|state| {
        let state = state.borrow();
        let sheet_name = if sheet.is_empty() { None } else { Some(sheet) };
        lattice_io::write_tsv_string(&state.workbook, sheet_name)
            .map(|s| s.into_bytes())
            .map_err(|e| JsError::new(&e.to_string()))
    })
}

/// Minimal workbook summary returned by the byte-based open functions.
#[derive(serde::Serialize)]
struct WorkbookInfo {
    sheets: Vec<String>,
    active_sheet: String,
}

/// Replace the global workbook with `wb` and return its `WorkbookInfo`.
fn load_workbook(wb: Workbook) -> WorkbookInfo {
    let info = WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    };
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.replace_workbook(wb);
        state.file_path = None;
    });
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a fresh dispatch against a brand-new state.
    fn run(command: &str, args: Value) -> Result<Value, String> {
        let mut state = AppState::new();
        dispatch(&mut state, command, args)
    }

    #[test]
    fn list_sheets_on_fresh_workbook() {
        let out = run("list_sheets", Value::Null).unwrap();
        let arr = out.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Sheet1");
        assert_eq!(arr[0]["is_active"], true);
    }

    #[test]
    fn set_then_get_cell_round_trips() {
        let mut state = AppState::new();
        dispatch(
            &mut state,
            "set_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 0, "value": "42" }),
        )
        .unwrap();
        let cell = dispatch(
            &mut state,
            "get_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 0 }),
        )
        .unwrap();
        assert_eq!(cell["value"], "42");
    }

    #[test]
    fn set_cell_with_formula_evaluates() {
        let mut state = AppState::new();
        dispatch(
            &mut state,
            "set_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 0, "value": "10" }),
        )
        .unwrap();
        dispatch(
            &mut state,
            "set_cell",
            serde_json::json!({
                "sheet": "Sheet1", "row": 0, "col": 1,
                "value": "", "formula": "A1*2"
            }),
        )
        .unwrap();
        let cell = dispatch(
            &mut state,
            "get_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 1 }),
        )
        .unwrap();
        assert_eq!(cell["value"], "20");
        assert_eq!(cell["formula"], "A1*2");
    }

    #[test]
    fn undo_restores_previous_value() {
        let mut state = AppState::new();
        dispatch(
            &mut state,
            "set_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 0, "value": "first" }),
        )
        .unwrap();
        dispatch(
            &mut state,
            "set_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 0, "value": "second" }),
        )
        .unwrap();
        dispatch(&mut state, "undo", Value::Null).unwrap();
        let cell = dispatch(
            &mut state,
            "get_cell",
            serde_json::json!({ "sheet": "Sheet1", "row": 0, "col": 0 }),
        )
        .unwrap();
        assert_eq!(cell["value"], "first");
    }

    #[test]
    fn add_and_list_sheets() {
        let mut state = AppState::new();
        dispatch(
            &mut state,
            "add_sheet",
            serde_json::json!({ "name": "Data" }),
        )
        .unwrap();
        let out = dispatch(&mut state, "list_sheets", Value::Null).unwrap();
        assert_eq!(out.as_array().unwrap().len(), 2);
    }

    #[test]
    fn unknown_command_errors() {
        let err = run("not_a_command", Value::Null).unwrap_err();
        assert!(err.contains("unknown command"));
    }

    #[test]
    fn move_sheet_reports_unsupported() {
        let err = run(
            "move_sheet",
            serde_json::json!({ "name": "Sheet1", "toIndex": 0 }),
        )
        .unwrap_err();
        assert!(err.contains("not supported"));
    }
}
