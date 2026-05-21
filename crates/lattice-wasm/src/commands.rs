//! Command handlers for the WASM dispatcher.
//!
//! Every handler here is a faithful, **synchronous** port of a Tauri command
//! from `src-tauri/src/commands/*.rs`. Each takes a `&mut AppState` and the
//! command's parameter struct (deserialized from camelCase JSON) and returns
//! a `serde_json::Value` on success or a `String` error on failure.
//!
//! The dispatcher in `lib.rs` matches command name strings, deserializes the
//! params, calls the handler, and re-serializes the result.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use lattice_charts::{Chart, ChartData, ChartType, DataSeries, render_chart};
use lattice_core::formula::evaluator::SimpleEvaluator;
use lattice_core::validation::validate;
use lattice_core::{
    Border, BorderStyle, CellFormat, CellRef, CellValue, ComparisonOperator, ConditionalRule,
    ConditionalRuleType, ConditionalStyle, FormulaEngine, HAlign, NamedFunction, Operation, Range,
    SheetResolver, SortDirection, SortKey, TextWrap, VAlign, ValidationEnforcement, ValidationRule,
    ValidationType, Workbook, col_to_letter,
};

use crate::cell_parse::{CellData, cell_to_data, map_error_to_cell_error, parse_cell_value};
use crate::state::AppState;

/// Result type for command handlers: a JSON value on success, error string on failure.
type CmdResult = Result<Value, String>;

// ===========================================================================
// SheetResolver — WASM has no filesystem, so `import_range` always returns None.
// ===========================================================================

/// A [`SheetResolver`] wrapper around a [`Workbook`].
///
/// Unlike the desktop `ImportRangeResolver`, the WASM build has no filesystem,
/// so `import_range` always returns `None` — `IMPORTRANGE()` formulas
/// evaluate to an error rather than reading an external file.
struct WasmResolver<'a> {
    workbook: &'a Workbook,
}

impl SheetResolver for WasmResolver<'_> {
    fn resolve_cell(
        &self,
        sheet_name: &str,
        row: u32,
        col: u32,
    ) -> lattice_core::Result<CellValue> {
        self.workbook.resolve_cell(sheet_name, row, col)
    }

    fn resolve_named_function(&self, name: &str) -> Option<&NamedFunction> {
        self.workbook.resolve_named_function(name)
    }

    /// WASM has no filesystem: `IMPORTRANGE` cannot read external files.
    fn import_range(&self, _file_path: &str, _range_string: &str) -> Option<CellValue> {
        None
    }
}

/// Re-evaluate all formula cells across ALL sheets in the workbook.
///
/// Faithful port of `recalculate_formulas` from `src-tauri/src/commands/cell.rs`.
fn recalculate_formulas(workbook: &mut Workbook, _changed_sheet: &str) {
    let all_sheet_names = workbook.sheet_names();
    let evaluator = SimpleEvaluator;

    for sheet_name in &all_sheet_names {
        let formula_cells: Vec<(u32, u32, String)> = {
            let Ok(s) = workbook.get_sheet(sheet_name) else {
                continue;
            };
            s.cells()
                .iter()
                .filter_map(|(&(r, c), cell)| cell.formula.as_ref().map(|f| (r, c, f.clone())))
                .collect()
        };

        for (r, c, formula_text) in formula_cells {
            let resolver = WasmResolver { workbook };
            let result = {
                let Ok(s) = workbook.get_sheet(sheet_name) else {
                    continue;
                };
                evaluator.evaluate_with_context(&formula_text, s, Some(&resolver))
            };
            let new_val = match result {
                Ok(v) => v,
                Err(e) => CellValue::Error(map_error_to_cell_error(&e)),
            };
            if let Ok(s) = workbook.get_sheet_mut(sheet_name)
                && let Some(cell) = s.get_cell(r, c)
            {
                let mut cell = cell.clone();
                cell.value = new_val;
                s.set_cell(r, c, cell);
            }
        }
    }
}

// ===========================================================================
// Cell commands
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetCellParams {
    sheet: String,
    row: u32,
    col: u32,
}

/// `get_cell` — return a single cell's data, or `null` if empty.
pub fn get_cell(state: &mut AppState, params: Value) -> CmdResult {
    let p: GetCellParams = de(params)?;
    let cell = state
        .workbook
        .get_cell(&p.sheet, p.row, p.col)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(cell.map(cell_to_data)).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetCellParams {
    sheet: String,
    row: u32,
    col: u32,
    value: String,
    formula: Option<String>,
}

/// `set_cell` — set a cell value (and optionally a formula).
///
/// Faithful port of `set_cell` from `src-tauri/src/commands/cell.rs`,
/// including the formula-evaluation path, validation enforcement, number
/// format inference, undo push, and full recalculation.
pub fn set_cell(state: &mut AppState, params: Value) -> CmdResult {
    let p: SetCellParams = de(params)?;

    // Record the old value for undo.
    let old_value = state
        .workbook
        .get_cell(&p.sheet, p.row, p.col)
        .map_err(|e| e.to_string())?
        .map(|c| c.value.clone())
        .unwrap_or(CellValue::Empty);

    let new_value = if let Some(ref formula_text) = p.formula {
        let evaluator = SimpleEvaluator;
        let resolver = WasmResolver {
            workbook: &state.workbook,
        };
        let eval_result = {
            let s = state
                .workbook
                .get_sheet(&p.sheet)
                .map_err(|e| e.to_string())?;
            evaluator.evaluate_with_context(formula_text, s, Some(&resolver))
        };
        match eval_result {
            Ok(v) => v,
            Err(e) => CellValue::Error(map_error_to_cell_error(&e)),
        }
    } else {
        let (val, _) = parse_cell_value(&p.value);
        val
    };

    // Check validation enforcement before writing.
    if let Some(rule) = state.workbook.validations.get_rule(&p.sheet, p.row, p.col)
        && rule.enforcement == ValidationEnforcement::Reject
        && !validate(&new_value, rule)
    {
        let msg = rule
            .error_message
            .clone()
            .unwrap_or_else(|| "Value does not pass validation".to_string());
        return Err(msg);
    }

    state
        .workbook
        .set_cell(&p.sheet, p.row, p.col, new_value.clone())
        .map_err(|e| e.to_string())?;

    // Store the formula on the cell, if any.
    if let Some(ref formula_text) = p.formula {
        let s = state
            .workbook
            .get_sheet_mut(&p.sheet)
            .map_err(|e| e.to_string())?;
        if let Some(cell) = s.get_cell(p.row, p.col) {
            let mut cell = cell.clone();
            cell.formula = Some(formula_text.clone());
            s.set_cell(p.row, p.col, cell);
        }
    }

    // Apply an inferred number format (e.g. percentage / currency / date).
    if p.formula.is_none() {
        let (_, number_format) = parse_cell_value(&p.value);
        if let Some(fmt) = number_format {
            let s = state
                .workbook
                .get_sheet_mut(&p.sheet)
                .map_err(|e| e.to_string())?;
            if let Some(cell) = s.get_cell(p.row, p.col) {
                let mut cell = cell.clone();
                cell.format.number_format = Some(fmt);
                s.set_cell(p.row, p.col, cell);
            }
        }
    }

    state.undo_stack.push(Operation::SetCell {
        sheet: p.sheet.clone(),
        row: p.row,
        col: p.col,
        old_value,
        new_value,
    });

    recalculate_formulas(&mut state.workbook, &p.sheet);

    Ok(Value::Null)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetRangeParams {
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
}

/// `get_range` — return a rectangular range of cells.
pub fn get_range(state: &mut AppState, params: Value) -> CmdResult {
    let p: GetRangeParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let mut rows: Vec<Vec<Option<CellData>>> = Vec::new();
    for r in p.start_row..=p.end_row {
        let mut row_data = Vec::new();
        for c in p.start_col..=p.end_col {
            row_data.push(s.get_cell(r, c).map(cell_to_data));
        }
        rows.push(row_data);
    }
    serde_json::to_value(rows).map_err(|e| e.to_string())
}

// ===========================================================================
// Comment commands
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentSetParams {
    sheet: String,
    row: u32,
    col: u32,
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CellPosParams {
    sheet: String,
    row: u32,
    col: u32,
}

/// `set_comment` — set a comment / note on a cell.
pub fn set_comment(state: &mut AppState, params: Value) -> CmdResult {
    let p: CommentSetParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.set_comment(p.row, p.col, p.text);
    Ok(Value::Null)
}

/// `get_comment` — return a cell's comment, or `null`.
pub fn get_comment(state: &mut AppState, params: Value) -> CmdResult {
    let p: CellPosParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    Ok(json!(s.get_comment(p.row, p.col).map(|s| s.to_string())))
}

/// `remove_comment` — remove a cell's comment.
pub fn remove_comment(state: &mut AppState, params: Value) -> CmdResult {
    let p: CellPosParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.remove_comment(p.row, p.col);
    Ok(Value::Null)
}

// ===========================================================================
// Data cleanup
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveDuplicatesParams {
    sheet: String,
    start_row: u32,
    end_row: u32,
    columns: Vec<u32>,
}

/// `remove_duplicates` — remove duplicate rows, return number removed.
pub fn remove_duplicates(state: &mut AppState, params: Value) -> CmdResult {
    let p: RemoveDuplicatesParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    Ok(json!(s.remove_duplicates(
        p.start_row,
        p.end_row,
        &p.columns
    )))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextToColumnsParams {
    sheet: String,
    col: u32,
    delimiter: String,
    start_row: u32,
    end_row: u32,
}

/// `text_to_columns` — split a column by delimiter, return max columns produced.
pub fn text_to_columns(state: &mut AppState, params: Value) -> CmdResult {
    let p: TextToColumnsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    Ok(json!(s.text_to_columns(
        p.col,
        &p.delimiter,
        p.start_row,
        p.end_row
    )))
}

// ===========================================================================
// Format commands
// ===========================================================================

#[derive(Deserialize)]
struct BorderEdgeUpdate {
    style: Option<String>,
    color: Option<String>,
}

#[derive(Deserialize)]
struct BordersUpdate {
    top: Option<BorderEdgeUpdate>,
    bottom: Option<BorderEdgeUpdate>,
    left: Option<BorderEdgeUpdate>,
    right: Option<BorderEdgeUpdate>,
}

#[derive(Deserialize)]
struct FormatUpdate {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strikethrough: Option<bool>,
    font_size: Option<f64>,
    font_family: Option<String>,
    font_color: Option<String>,
    bg_color: Option<String>,
    h_align: Option<String>,
    v_align: Option<String>,
    number_format: Option<String>,
    text_wrap: Option<String>,
    borders: Option<BordersUpdate>,
    text_rotation: Option<i16>,
    indent: Option<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormatCellsParams {
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    format: FormatUpdate,
}

/// Parse a border edge update into a core `Border`, `None` for "none".
fn parse_border_edge(edge: &BorderEdgeUpdate) -> Option<Border> {
    let style_str = edge.style.as_deref().unwrap_or("thin");
    let style = match style_str {
        "none" => return None,
        "thin" => BorderStyle::Thin,
        "medium" => BorderStyle::Medium,
        "thick" => BorderStyle::Thick,
        "dashed" => BorderStyle::Dashed,
        "dotted" => BorderStyle::Dotted,
        "double" => BorderStyle::Double,
        _ => BorderStyle::Thin,
    };
    let color = edge.color.as_deref().unwrap_or("#000000").to_string();
    Some(Border { style, color })
}

/// `format_cells` — apply formatting to a range, with undo support.
pub fn format_cells(state: &mut AppState, params: Value) -> CmdResult {
    let p: FormatCellsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    let format = &p.format;

    let mut changed: Vec<(u32, u32, CellFormat, CellFormat)> = Vec::new();

    for row in p.start_row..=p.end_row {
        for col in p.start_col..=p.end_col {
            let mut cell = s.get_cell(row, col).cloned().unwrap_or_default();
            let old_format = cell.format.clone();

            if let Some(bold) = format.bold {
                cell.format.bold = bold;
            }
            if let Some(italic) = format.italic {
                cell.format.italic = italic;
            }
            if let Some(size) = format.font_size {
                cell.format.font_size = size;
            }
            if let Some(ref family) = format.font_family {
                cell.format.font_family = family.clone();
            }
            if let Some(ref color) = format.font_color {
                cell.format.font_color = Some(color.clone());
            }
            if let Some(underline) = format.underline {
                cell.format.underline = underline;
            }
            if let Some(strikethrough) = format.strikethrough {
                cell.format.strikethrough = strikethrough;
            }
            if let Some(ref bg) = format.bg_color {
                if bg.is_empty() {
                    cell.format.bg_color = None;
                } else {
                    cell.format.bg_color = Some(bg.clone());
                }
            }
            if let Some(ref align) = format.h_align {
                cell.format.h_align = match align.as_str() {
                    "center" => HAlign::Center,
                    "right" => HAlign::Right,
                    _ => HAlign::Left,
                };
            }
            if let Some(ref align) = format.v_align {
                cell.format.v_align = match align.as_str() {
                    "top" => VAlign::Top,
                    "middle" => VAlign::Middle,
                    _ => VAlign::Bottom,
                };
            }
            if let Some(ref nf) = format.number_format {
                cell.format.number_format = Some(nf.clone());
            }
            if let Some(ref tw) = format.text_wrap {
                cell.format.text_wrap = match tw.as_str() {
                    "Wrap" => TextWrap::Wrap,
                    "Clip" => TextWrap::Clip,
                    _ => TextWrap::Overflow,
                };
            }
            if let Some(rotation) = format.text_rotation {
                cell.format.text_rotation = rotation;
            }
            if let Some(indent) = format.indent {
                cell.format.indent = indent;
            }
            if let Some(ref borders) = format.borders {
                if let Some(ref edge) = borders.top {
                    cell.format.borders.top = parse_border_edge(edge);
                }
                if let Some(ref edge) = borders.bottom {
                    cell.format.borders.bottom = parse_border_edge(edge);
                }
                if let Some(ref edge) = borders.left {
                    cell.format.borders.left = parse_border_edge(edge);
                }
                if let Some(ref edge) = borders.right {
                    cell.format.borders.right = parse_border_edge(edge);
                }
            }

            let new_format = cell.format.clone();
            if old_format != new_format {
                changed.push((row, col, old_format, new_format));
            }
            s.set_cell(row, col, cell);
        }
    }

    if !changed.is_empty() {
        state.undo_stack.push(Operation::FormatCells {
            sheet: p.sheet,
            cells: changed,
        });
    }
    Ok(Value::Null)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MergeParams {
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
}

/// `merge_cells` — merge a rectangular region.
pub fn merge_cells(state: &mut AppState, params: Value) -> CmdResult {
    let p: MergeParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.merge_cells(p.start_row, p.start_col, p.end_row, p.end_col)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `unmerge_cells` — unmerge any region containing the given cell.
pub fn unmerge_cells(state: &mut AppState, params: Value) -> CmdResult {
    let p: CellPosParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    Ok(json!(
        s.unmerge_cell(p.row, p.col).map_err(|e| e.to_string())?
    ))
}

#[derive(Deserialize)]
struct SheetOnlyParams {
    sheet: String,
}

#[derive(Serialize)]
struct MergedRegionData {
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
}

/// `get_merged_regions` — list all merged regions for a sheet.
pub fn get_merged_regions(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let out: Vec<MergedRegionData> = s
        .merged_regions()
        .iter()
        .map(|r| MergedRegionData {
            start_row: r.start_row,
            start_col: r.start_col,
            end_row: r.end_row,
            end_col: r.end_col,
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetBandedRowsParams {
    sheet: String,
    enabled: bool,
    even_color: String,
    odd_color: String,
    header_color: Option<String>,
    footer_color: Option<String>,
}

/// `set_banded_rows` — set alternating row colours on a sheet.
pub fn set_banded_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: SetBandedRowsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    if p.enabled {
        s.banded_rows = Some(lattice_core::BandedRows {
            enabled: true,
            even_color: p.even_color,
            odd_color: p.odd_color,
            header_color: p.header_color,
            footer_color: p.footer_color,
        });
    } else {
        s.banded_rows = None;
    }
    Ok(Value::Null)
}

#[derive(Serialize)]
struct BandedRowsData {
    enabled: bool,
    even_color: String,
    odd_color: String,
    header_color: Option<String>,
    footer_color: Option<String>,
}

/// `get_banded_rows` — return the banded-rows config for a sheet.
pub fn get_banded_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let out = s.banded_rows.as_ref().map(|b| BandedRowsData {
        enabled: b.enabled,
        even_color: b.even_color.clone(),
        odd_color: b.odd_color.clone(),
        header_color: b.header_color.clone(),
        footer_color: b.footer_color.clone(),
    });
    serde_json::to_value(out).map_err(|e| e.to_string())
}

// ===========================================================================
// Row / column manipulation
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RowCountParams {
    sheet: String,
    row: u32,
    count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColCountParams {
    sheet: String,
    col: u32,
    count: u32,
}

/// `insert_rows` — insert rows, with undo support.
pub fn insert_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: RowCountParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.insert_rows(p.row, p.count);
    state.undo_stack.push(Operation::InsertRows {
        sheet: p.sheet,
        row: p.row,
        count: p.count,
    });
    Ok(Value::Null)
}

/// `delete_rows` — delete rows, saving cells for undo.
pub fn delete_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: RowCountParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    let end_row = p.row + p.count;
    let deleted_cells: Vec<(u32, u32, lattice_core::Cell)> = s
        .cells()
        .iter()
        .filter(|((r, _), _)| *r >= p.row && *r < end_row)
        .map(|((r, c), cell)| (*r, *c, cell.clone()))
        .collect();
    s.delete_rows(p.row, p.count);
    state.undo_stack.push(Operation::DeleteRows {
        sheet: p.sheet,
        row: p.row,
        count: p.count,
        deleted_cells,
    });
    Ok(Value::Null)
}

/// `insert_cols` — insert columns, with undo support.
pub fn insert_cols(state: &mut AppState, params: Value) -> CmdResult {
    let p: ColCountParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.insert_cols(p.col, p.count);
    state.undo_stack.push(Operation::InsertCols {
        sheet: p.sheet,
        col: p.col,
        count: p.count,
    });
    Ok(Value::Null)
}

/// `delete_cols` — delete columns, saving cells for undo.
pub fn delete_cols(state: &mut AppState, params: Value) -> CmdResult {
    let p: ColCountParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    let end_col = p.col + p.count;
    let deleted_cells: Vec<(u32, u32, lattice_core::Cell)> = s
        .cells()
        .iter()
        .filter(|((_, c), _)| *c >= p.col && *c < end_col)
        .map(|((r, c), cell)| (*r, *c, cell.clone()))
        .collect();
    s.delete_cols(p.col, p.count);
    state.undo_stack.push(Operation::DeleteCols {
        sheet: p.sheet,
        col: p.col,
        count: p.count,
        deleted_cells,
    });
    Ok(Value::Null)
}

// ===========================================================================
// Column / row sizing
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetColWidthParams {
    sheet: String,
    col: u32,
    width: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetRowHeightParams {
    sheet: String,
    row: u32,
    height: f64,
}

/// `set_col_width` — set a column's width on a sheet.
pub fn set_col_width(state: &mut AppState, params: Value) -> CmdResult {
    let p: SetColWidthParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.col_widths.insert(p.col, p.width);
    Ok(Value::Null)
}

/// `set_row_height` — set a row's height on a sheet.
pub fn set_row_height(state: &mut AppState, params: Value) -> CmdResult {
    let p: SetRowHeightParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.row_heights.insert(p.row, p.height);
    Ok(Value::Null)
}

/// `get_col_widths` — return the column-width map for a sheet.
pub fn get_col_widths(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let map: HashMap<String, f64> = s
        .col_widths
        .iter()
        .map(|(&k, &v)| (k.to_string(), v))
        .collect();
    serde_json::to_value(map).map_err(|e| e.to_string())
}

/// `get_row_heights` — return the row-height map for a sheet.
pub fn get_row_heights(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let map: HashMap<String, f64> = s
        .row_heights
        .iter()
        .map(|(&k, &v)| (k.to_string(), v))
        .collect();
    serde_json::to_value(map).map_err(|e| e.to_string())
}

// ===========================================================================
// Sheet tab color / reorder
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetTabColorParams {
    name: String,
    color: Option<String>,
}

/// `set_sheet_tab_color` — set (or clear) a sheet's tab colour.
pub fn set_sheet_tab_color(state: &mut AppState, params: Value) -> CmdResult {
    let p: SetTabColorParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.name)
        .map_err(|e| e.to_string())?;
    s.set_tab_color(p.color);
    Ok(Value::Null)
}

// ===========================================================================
// Search
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FindParams {
    sheet: String,
    query: String,
    case_sensitive: Option<bool>,
}

#[derive(Serialize)]
struct FindResult {
    row: u32,
    col: u32,
    value: String,
}

/// `find_in_sheet` — find text in a sheet.
///
/// The frontend `FindResult` interface expects `{ row, col, value }`, so
/// each match carries its displayed value (unlike the desktop command which
/// returns bare `(row, col)` tuples).
pub fn find_in_sheet(state: &mut AppState, params: Value) -> CmdResult {
    let p: FindParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;

    let case_sensitive = p.case_sensitive.unwrap_or(false);
    let query_cmp = if case_sensitive {
        p.query.clone()
    } else {
        p.query.to_lowercase()
    };

    let mut matches: Vec<FindResult> = Vec::new();
    for (&(row, col), cell) in s.cells() {
        let display = cell_value_to_display(&cell.value);
        if display.is_empty() {
            continue;
        }
        let hay = if case_sensitive {
            display.clone()
        } else {
            display.to_lowercase()
        };
        if hay.contains(&query_cmp) {
            matches.push(FindResult {
                row,
                col,
                value: display,
            });
        }
    }
    matches.sort_by(|a, b| (a.row, a.col).cmp(&(b.row, b.col)));
    serde_json::to_value(matches).map_err(|e| e.to_string())
}

// ===========================================================================
// Sheet commands
// ===========================================================================

#[derive(Serialize)]
struct SheetInfo {
    name: String,
    is_active: bool,
}

/// `list_sheets` — list all sheets with their active state.
pub fn list_sheets(state: &mut AppState, _params: Value) -> CmdResult {
    let active = state.workbook.active_sheet.clone();
    let out: Vec<SheetInfo> = state
        .workbook
        .sheet_names()
        .into_iter()
        .map(|name| SheetInfo {
            is_active: name == active,
            name,
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct AddSheetParams {
    name: String,
}

/// `add_sheet` — add a new sheet.
pub fn add_sheet(state: &mut AppState, params: Value) -> CmdResult {
    let p: AddSheetParams = de(params)?;
    state
        .workbook
        .add_sheet(&p.name)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameSheetParams {
    old: String,
    new_name: String,
}

/// `rename_sheet` — rename an existing sheet.
pub fn rename_sheet(state: &mut AppState, params: Value) -> CmdResult {
    let p: RenameSheetParams = de(params)?;
    state
        .workbook
        .rename_sheet(&p.old, &p.new_name)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

#[derive(Deserialize)]
struct NameOnlyParams {
    name: String,
}

/// `delete_sheet` — delete a sheet by name.
pub fn delete_sheet(state: &mut AppState, params: Value) -> CmdResult {
    let p: NameOnlyParams = de(params)?;
    state
        .workbook
        .remove_sheet(&p.name)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `set_active_sheet` — set the active sheet.
pub fn set_active_sheet(state: &mut AppState, params: Value) -> CmdResult {
    let p: NameOnlyParams = de(params)?;
    let _ = state
        .workbook
        .get_sheet(&p.name)
        .map_err(|e| e.to_string())?;
    state.workbook.active_sheet = p.name;
    Ok(Value::Null)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateSheetParams {
    source: String,
    new_name: String,
}

/// `duplicate_sheet` — duplicate a sheet's cells into a new sheet.
pub fn duplicate_sheet(state: &mut AppState, params: Value) -> CmdResult {
    let p: DuplicateSheetParams = de(params)?;
    let sheet = state
        .workbook
        .get_sheet(&p.source)
        .map_err(|e| e.to_string())?
        .clone();
    state
        .workbook
        .add_sheet(&p.new_name)
        .map_err(|e| e.to_string())?;
    let dest = state
        .workbook
        .get_sheet_mut(&p.new_name)
        .map_err(|e| e.to_string())?;
    for (&(row, col), cell) in sheet.cells() {
        dest.set_cell(row, col, cell.clone());
    }
    Ok(Value::Null)
}

// ===========================================================================
// Edit commands (undo / redo)
// ===========================================================================

/// `undo` — undo the last operation.
pub fn undo(state: &mut AppState, _params: Value) -> CmdResult {
    let op = state
        .undo_stack
        .undo()
        .ok_or_else(|| "Nothing to undo".to_string())?;
    apply_undo(&mut state.workbook, op)?;
    Ok(Value::Null)
}

/// `redo` — redo the last undone operation.
pub fn redo(state: &mut AppState, _params: Value) -> CmdResult {
    let op = state
        .undo_stack
        .redo()
        .ok_or_else(|| "Nothing to redo".to_string())?;
    apply_redo(&mut state.workbook, op)?;
    Ok(Value::Null)
}

/// Apply an operation in the *undo* direction (port of `edit::undo`).
fn apply_undo(workbook: &mut Workbook, op: Operation) -> Result<(), String> {
    match op {
        Operation::SetCell {
            sheet,
            row,
            col,
            old_value,
            ..
        } => {
            workbook
                .set_cell(&sheet, row, col, old_value)
                .map_err(|e| e.to_string())?;
        }
        Operation::AddSheet { name } => {
            workbook.remove_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RemoveSheet { name } => {
            workbook.add_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RenameSheet { old_name, new_name } => {
            workbook
                .rename_sheet(&new_name, &old_name)
                .map_err(|e| e.to_string())?;
        }
        Operation::FormatCells { sheet, cells } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            for (row, col, old_format, _new_format) in cells {
                if let Some(cell) = s.get_cell(row, col) {
                    let mut cell = cell.clone();
                    cell.format = old_format;
                    s.set_cell(row, col, cell);
                }
            }
        }
        Operation::InsertRows { sheet, row, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_rows(row, count);
        }
        Operation::DeleteRows {
            sheet,
            row,
            count,
            deleted_cells,
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.insert_rows(row, count);
            for (r, c, cell) in deleted_cells {
                s.set_cell(r, c, cell);
            }
        }
        Operation::InsertCols { sheet, col, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_cols(col, count);
        }
        Operation::DeleteCols {
            sheet,
            col,
            count,
            deleted_cells,
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.insert_cols(col, count);
            for (r, c, cell) in deleted_cells {
                s.set_cell(r, c, cell);
            }
        }
    }
    Ok(())
}

/// Apply an operation in the *redo* direction (port of `edit::redo`).
fn apply_redo(workbook: &mut Workbook, op: Operation) -> Result<(), String> {
    match op {
        Operation::SetCell {
            sheet,
            row,
            col,
            new_value,
            ..
        } => {
            workbook
                .set_cell(&sheet, row, col, new_value)
                .map_err(|e| e.to_string())?;
        }
        Operation::AddSheet { name } => {
            workbook.add_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RemoveSheet { name } => {
            workbook.remove_sheet(&name).map_err(|e| e.to_string())?;
        }
        Operation::RenameSheet { old_name, new_name } => {
            workbook
                .rename_sheet(&old_name, &new_name)
                .map_err(|e| e.to_string())?;
        }
        Operation::FormatCells { sheet, cells } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            for (row, col, _old_format, new_format) in cells {
                if let Some(cell) = s.get_cell(row, col) {
                    let mut cell = cell.clone();
                    cell.format = new_format;
                    s.set_cell(row, col, cell);
                }
            }
        }
        Operation::InsertRows { sheet, row, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.insert_rows(row, count);
        }
        Operation::DeleteRows {
            sheet, row, count, ..
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_rows(row, count);
        }
        Operation::InsertCols { sheet, col, count } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.insert_cols(col, count);
        }
        Operation::DeleteCols {
            sheet, col, count, ..
        } => {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            s.delete_cols(col, count);
        }
    }
    Ok(())
}

// ===========================================================================
// File commands
// ===========================================================================

#[derive(Serialize)]
struct WorkbookInfo {
    sheets: Vec<String>,
    active_sheet: String,
}

fn workbook_info(wb: &Workbook) -> WorkbookInfo {
    WorkbookInfo {
        sheets: wb.sheet_names(),
        active_sheet: wb.active_sheet.clone(),
    }
}

/// `new_workbook` — create a new empty workbook.
pub fn new_workbook(state: &mut AppState, _params: Value) -> CmdResult {
    let wb = Workbook::new();
    let info = workbook_info(&wb);
    state.replace_workbook(wb);
    state.file_path = None;
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// `get_recent_files` — recent files are not tracked in the browser build.
pub fn get_recent_files(_state: &mut AppState, _params: Value) -> CmdResult {
    // The browser build has no persistent recent-files store.
    Ok(json!([]))
}

/// `add_recent_file` — no-op in the browser build.
pub fn add_recent_file(_state: &mut AppState, _params: Value) -> CmdResult {
    Ok(Value::Null)
}

// ===========================================================================
// Chart commands
// ===========================================================================

#[derive(Serialize)]
struct ChartInfo {
    id: String,
    chart_type: String,
    data_range: String,
    sheet: String,
    title: Option<String>,
    width: u32,
    height: u32,
}

/// Parse a chart type string into a `ChartType` enum value.
fn parse_chart_type(s: &str) -> Result<ChartType, String> {
    match s {
        "bar" | "stacked_bar" => Ok(ChartType::Bar),
        "line" => Ok(ChartType::Line),
        "pie" => Ok(ChartType::Pie),
        "scatter" => Ok(ChartType::Scatter),
        "area" | "stacked_area" => Ok(ChartType::Area),
        "combo" => Ok(ChartType::Combo),
        "histogram" => Ok(ChartType::Histogram),
        "candlestick" => Ok(ChartType::Candlestick),
        "treemap" => Ok(ChartType::Treemap),
        "waterfall" => Ok(ChartType::Waterfall),
        "radar" => Ok(ChartType::Radar),
        "bubble" => Ok(ChartType::Bubble),
        "gauge" => Ok(ChartType::Gauge),
        _ => Err(format!(
            "Invalid chart type '{}'. Valid: bar, line, pie, scatter, area, combo, histogram, candlestick, treemap, waterfall, radar, bubble, gauge, stacked_bar, stacked_area",
            s
        )),
    }
}

/// Returns true when the chart type string requests a stacked rendering.
fn is_stacked(chart_type_str: &str) -> bool {
    chart_type_str.starts_with("stacked_")
}

/// Parse an A1-style range "A1:C5" into (start_row, start_col, end_row, end_col).
fn parse_chart_range(range: &str) -> Result<(u32, u32, u32, u32), String> {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid range format: {}", range));
    }
    let (sr, sc) = parse_chart_cell_ref(parts[0])?;
    let (er, ec) = parse_chart_cell_ref(parts[1])?;
    Ok((sr, sc, er, ec))
}

/// Parse a single cell reference like "B3" into (row, col), 0-based.
fn parse_chart_cell_ref(s: &str) -> Result<(u32, u32), String> {
    let s = s.trim();
    let col_end = s
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| format!("Invalid cell ref: {}", s))?;
    let col_str = &s[..col_end];
    let row_str = &s[col_end..];
    let mut col: u32 = 0;
    for ch in col_str.chars() {
        let c = ch.to_ascii_uppercase();
        if !c.is_ascii_uppercase() {
            return Err(format!("Invalid column letter in cell ref: {}", s));
        }
        col = col * 26 + (c as u32 - b'A' as u32 + 1);
    }
    let col = col.saturating_sub(1);
    let row: u32 = row_str
        .parse::<u32>()
        .map_err(|_| format!("Invalid row number in cell ref: {}", s))?;
    let row = row.saturating_sub(1);
    Ok((row, col))
}

/// Extract chart data from the workbook for the given chart definition.
fn extract_chart_data(workbook: &Workbook, chart: &Chart) -> Result<ChartData, String> {
    let (sr, sc, er, ec) = parse_chart_range(&chart.data_range)?;
    let sheet = workbook
        .get_sheet(&chart.sheet)
        .map_err(|e| e.to_string())?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for r in sr..=er {
        let mut row_vals = Vec::new();
        for c in sc..=ec {
            let val = match sheet.get_cell(r, c) {
                Some(cell) => cell_value_to_display(&cell.value),
                None => String::new(),
            };
            row_vals.push(val);
        }
        rows.push(row_vals);
    }

    if rows.is_empty() || rows[0].is_empty() {
        return Ok(ChartData {
            labels: vec![],
            series: vec![],
        });
    }

    let num_cols = rows[0].len();

    if num_cols == 1 {
        let values: Vec<f64> = rows
            .iter()
            .skip(1)
            .map(|r| r[0].parse::<f64>().unwrap_or(0.0))
            .collect();
        let labels: Vec<String> = (1..=values.len()).map(|i| i.to_string()).collect();
        let name = if rows[0][0].is_empty() {
            "Series 1".to_string()
        } else {
            rows[0][0].clone()
        };
        return Ok(ChartData {
            labels,
            series: vec![DataSeries {
                name,
                values,
                color: None,
            }],
        });
    }

    let header = &rows[0];
    let data_rows: Vec<&Vec<String>> = rows.iter().skip(1).collect();
    let labels: Vec<String> = data_rows.iter().map(|r| r[0].clone()).collect();

    let mut series = Vec::new();
    for col_idx in 1..num_cols {
        let name = if col_idx < header.len() && !header[col_idx].is_empty() {
            header[col_idx].clone()
        } else {
            format!("Series {}", col_idx)
        };
        let values: Vec<f64> = data_rows
            .iter()
            .map(|r| {
                if col_idx < r.len() {
                    r[col_idx].parse::<f64>().unwrap_or(0.0)
                } else {
                    0.0
                }
            })
            .collect();
        series.push(DataSeries {
            name,
            values,
            color: None,
        });
    }

    Ok(ChartData { labels, series })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateChartParams {
    sheet: String,
    chart_type: String,
    data_range: String,
    title: Option<String>,
}

/// `create_chart` — create a chart, return its generated ID.
pub fn create_chart(state: &mut AppState, params: Value) -> CmdResult {
    let p: CreateChartParams = de(params)?;
    let ct = parse_chart_type(&p.chart_type)?;
    let stacked = is_stacked(&p.chart_type);
    let chart_id = uuid::Uuid::new_v4().to_string();

    let mut chart = Chart::new(&chart_id, ct, &p.data_range, &p.sheet);
    if let Some(t) = p.title {
        chart = chart.with_title(t);
    }
    state.chart_store.insert(chart_id.clone(), chart);
    if stacked {
        state.chart_stacked.insert(chart_id.clone(), true);
    }
    Ok(json!(chart_id))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChartIdParams {
    chart_id: String,
}

/// `render_chart_svg` — render a chart to an SVG string.
pub fn render_chart_svg(state: &mut AppState, params: Value) -> CmdResult {
    let p: ChartIdParams = de(params)?;
    let chart = state
        .chart_store
        .charts
        .get(&p.chart_id)
        .cloned()
        .ok_or_else(|| format!("Chart not found: {}", p.chart_id))?;
    let stacked = state
        .chart_stacked
        .get(&p.chart_id)
        .copied()
        .unwrap_or(false);

    let data = extract_chart_data(&state.workbook, &chart)?;
    let mut options = chart.to_options();
    options.stacked = stacked;
    Ok(json!(render_chart(&chart.chart_type, &data, &options)))
}

#[derive(Deserialize)]
struct ListChartsParams {
    sheet: Option<String>,
}

/// `list_charts` — list all charts, optionally filtered by sheet.
pub fn list_charts(state: &mut AppState, params: Value) -> CmdResult {
    let p: ListChartsParams = de(params)?;
    let out: Vec<ChartInfo> = state
        .chart_store
        .charts
        .values()
        .filter(|c| p.sheet.as_ref().is_none_or(|s| c.sheet == *s))
        .map(|c| ChartInfo {
            id: c.id.clone(),
            chart_type: c.chart_type.to_string(),
            data_range: c.data_range.clone(),
            sheet: c.sheet.clone(),
            title: c.title.clone(),
            width: c.width,
            height: c.height,
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

/// `delete_chart` — delete a chart by ID.
pub fn delete_chart(state: &mut AppState, params: Value) -> CmdResult {
    let p: ChartIdParams = de(params)?;
    if state.chart_store.charts.remove(&p.chart_id).is_some() {
        state.chart_stacked.remove(&p.chart_id);
        Ok(Value::Null)
    } else {
        Err(format!("Chart not found: {}", p.chart_id))
    }
}

/// `get_chart_config` — return the current configuration of a chart.
pub fn get_chart_config(state: &mut AppState, params: Value) -> CmdResult {
    let p: ChartIdParams = de(params)?;
    let chart = state
        .chart_store
        .charts
        .get(&p.chart_id)
        .ok_or_else(|| format!("Chart not found: {}", p.chart_id))?;
    let stacked = state
        .chart_stacked
        .get(&p.chart_id)
        .copied()
        .unwrap_or(false);
    let chart_type_str = if stacked {
        format!("stacked_{}", chart.chart_type)
    } else {
        chart.chart_type.to_string()
    };
    let info = ChartInfo {
        id: chart.id.clone(),
        chart_type: chart_type_str,
        data_range: chart.data_range.clone(),
        sheet: chart.sheet.clone(),
        title: chart.title.clone(),
        width: chart.width,
        height: chart.height,
    };
    serde_json::to_value(info).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateChartParams {
    chart_id: String,
    chart_type: Option<String>,
    data_range: Option<String>,
    title: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

/// `update_chart` — update an existing chart's properties.
pub fn update_chart(state: &mut AppState, params: Value) -> CmdResult {
    let p: UpdateChartParams = de(params)?;

    // Validate before mutating.
    if let Some(ref ct_str) = p.chart_type {
        parse_chart_type(ct_str)?;
    }
    if let Some(ref dr) = p.data_range {
        let _ = parse_chart_range(dr)?;
    }

    let chart = state
        .chart_store
        .charts
        .get_mut(&p.chart_id)
        .ok_or_else(|| format!("Chart not found: {}", p.chart_id))?;

    let mut new_stacked = None;
    if let Some(ref ct_str) = p.chart_type {
        chart.chart_type = parse_chart_type(ct_str)?;
        new_stacked = Some(is_stacked(ct_str));
    }
    if let Some(ref dr) = p.data_range {
        chart.data_range = dr.clone();
    }
    if let Some(ref t) = p.title {
        chart.title = if t.is_empty() { None } else { Some(t.clone()) };
    }
    if let Some(w) = p.width {
        chart.width = w;
    }
    if let Some(h) = p.height {
        chart.height = h;
    }

    let info = ChartInfo {
        id: chart.id.clone(),
        chart_type: p
            .chart_type
            .clone()
            .unwrap_or_else(|| chart.chart_type.to_string()),
        data_range: chart.data_range.clone(),
        sheet: chart.sheet.clone(),
        title: chart.title.clone(),
        width: chart.width,
        height: chart.height,
    };

    if let Some(stacked) = new_stacked {
        if stacked {
            state.chart_stacked.insert(p.chart_id, true);
        } else {
            state.chart_stacked.remove(&p.chart_id);
        }
    }
    serde_json::to_value(info).map_err(|e| e.to_string())
}

// ===========================================================================
// Validation commands
// ===========================================================================

#[derive(Serialize)]
struct ValidationData {
    rule_type: String,
    list_items: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    min_date: Option<String>,
    max_date: Option<String>,
    formula: Option<String>,
    allow_blank: bool,
    error_message: Option<String>,
    enforcement: String,
}

fn rule_to_data(rule: &ValidationRule) -> ValidationData {
    let enforcement_str = match rule.enforcement {
        ValidationEnforcement::Warn => "warn".to_string(),
        ValidationEnforcement::Reject => "reject".to_string(),
    };
    match &rule.validation_type {
        ValidationType::List(items) => ValidationData {
            rule_type: "list".to_string(),
            list_items: Some(items.join(", ")),
            min: None,
            max: None,
            min_date: None,
            max_date: None,
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
            enforcement: enforcement_str,
        },
        ValidationType::NumberRange { min, max } => ValidationData {
            rule_type: "number".to_string(),
            list_items: None,
            min: *min,
            max: *max,
            min_date: None,
            max_date: None,
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
            enforcement: enforcement_str,
        },
        ValidationType::DateRange { min, max } => ValidationData {
            rule_type: "date".to_string(),
            list_items: None,
            min: None,
            max: None,
            min_date: min.clone(),
            max_date: max.clone(),
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
            enforcement: enforcement_str,
        },
        ValidationType::TextLength { min, max } => ValidationData {
            rule_type: "text_length".to_string(),
            list_items: None,
            min: min.map(|v| v as f64),
            max: max.map(|v| v as f64),
            min_date: None,
            max_date: None,
            formula: None,
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
            enforcement: enforcement_str,
        },
        ValidationType::ListRange(range_ref) => ValidationData {
            rule_type: "list_range".to_string(),
            list_items: None,
            min: None,
            max: None,
            min_date: None,
            max_date: None,
            formula: Some(range_ref.clone()),
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
            enforcement: enforcement_str,
        },
        ValidationType::Custom(f) => ValidationData {
            rule_type: "custom".to_string(),
            list_items: None,
            min: None,
            max: None,
            min_date: None,
            max_date: None,
            formula: Some(f.clone()),
            allow_blank: rule.allow_blank,
            error_message: rule.error_message.clone(),
            enforcement: enforcement_str,
        },
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetValidationParams {
    sheet: String,
    row: u32,
    col: u32,
    rule_type: String,
    list_items: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    min_date: Option<String>,
    max_date: Option<String>,
    formula: Option<String>,
    allow_blank: Option<bool>,
    error_message: Option<String>,
    enforcement: Option<String>,
}

/// `set_validation` — set a validation rule on a cell.
pub fn set_validation(state: &mut AppState, params: Value) -> CmdResult {
    let p: SetValidationParams = de(params)?;
    let validation_type = match p.rule_type.as_str() {
        "list" => {
            let items = p
                .list_items
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            ValidationType::List(items)
        }
        "number" => ValidationType::NumberRange {
            min: p.min,
            max: p.max,
        },
        "date" => ValidationType::DateRange {
            min: p.min_date,
            max: p.max_date,
        },
        "text_length" => ValidationType::TextLength {
            min: p.min.map(|v| v as usize),
            max: p.max.map(|v| v as usize),
        },
        "list_range" => ValidationType::ListRange(p.formula.unwrap_or_default()),
        "custom" => ValidationType::Custom(p.formula.unwrap_or_default()),
        _ => return Err(format!("Unknown validation type: {}", p.rule_type)),
    };

    let enforcement_mode = match p.enforcement.as_deref() {
        Some("reject") => ValidationEnforcement::Reject,
        _ => ValidationEnforcement::Warn,
    };

    let rule = ValidationRule {
        validation_type,
        allow_blank: p.allow_blank.unwrap_or(true),
        error_message: p.error_message,
        enforcement: enforcement_mode,
    };
    state
        .workbook
        .validations
        .set_rule(&p.sheet, p.row, p.col, rule);
    Ok(Value::Null)
}

/// `get_validation` — return a cell's validation rule, or `null`.
pub fn get_validation(state: &mut AppState, params: Value) -> CmdResult {
    let p: CellPosParams = de(params)?;
    let rule = state.workbook.validations.get_rule(&p.sheet, p.row, p.col);
    serde_json::to_value(rule.map(rule_to_data)).map_err(|e| e.to_string())
}

/// `remove_validation` — remove a cell's validation rule.
pub fn remove_validation(state: &mut AppState, params: Value) -> CmdResult {
    let p: CellPosParams = de(params)?;
    state
        .workbook
        .validations
        .remove_rule(&p.sheet, p.row, p.col);
    Ok(Value::Null)
}

/// `list_validations` — list all validation rules for a sheet.
pub fn list_validations(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let rules = state.workbook.validations.list_rules(&p.sheet);
    let out: Vec<(u32, u32, ValidationData)> = rules
        .into_iter()
        .map(|((r, c), rule)| (r, c, rule_to_data(rule)))
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

// ===========================================================================
// Filter commands
// ===========================================================================

#[derive(Serialize)]
struct FilterInfo {
    active: bool,
    start_col: u32,
    end_col: u32,
    header_row: u32,
    filtered_cols: Vec<u32>,
    total_rows: u32,
    visible_rows: u32,
}

/// `set_auto_filter` — set auto-filter on a sheet.
pub fn set_auto_filter(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let (max_row, max_col) = s.used_range();
    let total_rows = if max_row > 0 { max_row } else { 0 };
    let visible = (1..=max_row).filter(|r| !s.is_row_hidden(*r)).count() as u32;
    let info = FilterInfo {
        active: true,
        start_col: 0,
        end_col: max_col,
        header_row: 0,
        filtered_cols: Vec::new(),
        total_rows,
        visible_rows: visible,
    };
    serde_json::to_value(info).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColParams {
    sheet: String,
    col: u32,
}

/// `get_column_values` — return sorted unique values in a column.
pub fn get_column_values(state: &mut AppState, params: Value) -> CmdResult {
    let p: ColParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let (max_row, _) = s.used_range();

    let mut values: BTreeSet<String> = BTreeSet::new();
    for row in 1..=max_row {
        if let Some(cell) = s.get_cell(row, p.col) {
            let text = cell_value_to_display(&cell.value);
            if !text.is_empty() {
                values.insert(text);
            }
        }
    }
    for row in 1..=max_row {
        if s.get_cell(row, p.col).is_none()
            || matches!(
                s.get_cell(row, p.col).map(|c| &c.value),
                Some(CellValue::Empty)
            )
        {
            values.insert("(Blanks)".to_string());
            break;
        }
    }
    serde_json::to_value(values.into_iter().collect::<Vec<_>>()).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyColumnFilterParams {
    sheet: String,
    col: u32,
    values: Vec<String>,
}

/// `apply_column_filter` — hide rows that fail the combined column filters.
pub fn apply_column_filter(state: &mut AppState, params: Value) -> CmdResult {
    let p: ApplyColumnFilterParams = de(params)?;

    // Merge the new filter into the active_filters map for this sheet.
    {
        let sheet_filters = state.active_filters.entry(p.sheet.clone()).or_default();
        sheet_filters.insert(p.col, p.values.clone());
    }
    let all_filters: HashMap<u32, Vec<String>> = state
        .active_filters
        .get(&p.sheet)
        .cloned()
        .unwrap_or_default();

    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    let (max_row, max_col) = s.used_range();
    s.unhide_rows(1, max_row);

    let filter_sets: Vec<(u32, bool, HashSet<String>)> = all_filters
        .iter()
        .map(|(&c, vals)| {
            let allow_blanks = vals.iter().any(|v| v == "(Blanks)");
            let allowed: HashSet<String> = vals
                .iter()
                .filter(|v| *v != "(Blanks)")
                .map(|v| v.to_lowercase())
                .collect();
            (c, allow_blanks, allowed)
        })
        .collect();

    for row in 1..=max_row {
        let passes_all = filter_sets.iter().all(|(c, allow_blanks, allowed)| {
            let cell_val = s.get_cell(row, *c).map(|cell| &cell.value);
            let is_blank = cell_val.is_none() || matches!(cell_val, Some(CellValue::Empty));
            if is_blank {
                *allow_blanks
            } else {
                let text = cell_value_to_display(cell_val.unwrap()).to_lowercase();
                allowed.contains(&text)
            }
        });
        if !passes_all {
            s.hide_rows(row, 1);
        }
    }

    let visible = (1..=max_row).filter(|r| !s.is_row_hidden(*r)).count() as u32;
    let filtered_cols: Vec<u32> = all_filters.keys().copied().collect();
    let info = FilterInfo {
        active: true,
        start_col: 0,
        end_col: max_col,
        header_row: 0,
        filtered_cols,
        total_rows: max_row,
        visible_rows: visible,
    };
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// `clear_filter` — clear all filters and unhide all rows.
pub fn clear_filter(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    state.active_filters.remove(&p.sheet);
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    let (max_row, _) = s.used_range();
    if max_row > 0 {
        s.unhide_rows(0, max_row + 1);
    }
    Ok(Value::Null)
}

/// `get_filter_info` — return the current filter state for a sheet.
pub fn get_filter_info(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let filtered_cols: Vec<u32> = state
        .active_filters
        .get(&p.sheet)
        .map(|m| m.keys().copied().collect())
        .unwrap_or_default();
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let (max_row, max_col) = s.used_range();
    let has_hidden = (1..=max_row).any(|r| s.is_row_hidden(r));
    let visible = (1..=max_row).filter(|r| !s.is_row_hidden(*r)).count() as u32;
    let info = FilterInfo {
        active: has_hidden || !filtered_cols.is_empty(),
        start_col: 0,
        end_col: max_col,
        header_row: 0,
        filtered_cols,
        total_rows: max_row,
        visible_rows: visible,
    };
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// `get_hidden_rows` — return the sorted set of hidden rows.
pub fn get_hidden_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let mut rows: Vec<u32> = s.hidden_rows.iter().copied().collect();
    rows.sort();
    serde_json::to_value(rows).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HideRowsParams {
    sheet: String,
    start_row: u32,
    count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HideColsParams {
    sheet: String,
    start_col: u32,
    count: u32,
}

/// `hide_rows` — hide a span of rows.
pub fn hide_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: HideRowsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.hide_rows(p.start_row, p.count);
    Ok(Value::Null)
}

/// `unhide_rows` — unhide a span of rows.
pub fn unhide_rows(state: &mut AppState, params: Value) -> CmdResult {
    let p: HideRowsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.unhide_rows(p.start_row, p.count);
    Ok(Value::Null)
}

/// `hide_cols` — hide a span of columns.
pub fn hide_cols(state: &mut AppState, params: Value) -> CmdResult {
    let p: HideColsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.hide_cols(p.start_col, p.count);
    Ok(Value::Null)
}

/// `unhide_cols` — unhide a span of columns.
pub fn unhide_cols(state: &mut AppState, params: Value) -> CmdResult {
    let p: HideColsParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.unhide_cols(p.start_col, p.count);
    Ok(Value::Null)
}

/// `get_hidden_cols` — return the sorted set of hidden columns.
pub fn get_hidden_cols(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let mut cols: Vec<u32> = s.hidden_cols.iter().copied().collect();
    cols.sort();
    serde_json::to_value(cols).map_err(|e| e.to_string())
}

// ===========================================================================
// Sort
// ===========================================================================

#[derive(Deserialize)]
struct SortKeyInput {
    col: u32,
    direction: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SortRangeParams {
    sheet: String,
    range: Option<String>,
    sort_keys: Vec<SortKeyInput>,
    has_headers: Option<bool>,
}

/// `sort_range` — sort a range of rows by the given keys.
pub fn sort_range(state: &mut AppState, params: Value) -> CmdResult {
    let p: SortRangeParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;

    let (start_row, start_col, end_row, end_col) = if let Some(ref range_str) = p.range {
        let parts: Vec<&str> = range_str.split(':').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid range format '{}': expected 'A1:B2'",
                range_str
            ));
        }
        let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
        let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
        (start.row, start.col, end.row, end.col)
    } else {
        let (max_row, max_col) = s.used_range();
        (0, 0, max_row, max_col)
    };

    let effective_start_row = if p.has_headers.unwrap_or(false) {
        start_row + 1
    } else {
        start_row
    };

    let keys: Vec<SortKey> = p
        .sort_keys
        .iter()
        .map(|k| SortKey {
            col: k.col,
            direction: if k.direction == "desc" {
                SortDirection::Descending
            } else {
                SortDirection::Ascending
            },
        })
        .collect();

    lattice_core::sort::sort_range(s, effective_start_row, end_row, start_col, end_col, &keys)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

// ===========================================================================
// Row group commands
// ===========================================================================

#[derive(Serialize)]
struct RowGroupData {
    start: u32,
    end: u32,
    collapsed: bool,
    level: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddRowGroupParams {
    sheet: String,
    start: u32,
    end: u32,
}

/// `add_row_group` — add a collapsible row group.
pub fn add_row_group(state: &mut AppState, params: Value) -> CmdResult {
    let p: AddRowGroupParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.add_row_group(p.start, p.end).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RowGroupIndexParams {
    sheet: String,
    index: usize,
}

/// `remove_row_group` — remove a row group by index.
pub fn remove_row_group(state: &mut AppState, params: Value) -> CmdResult {
    let p: RowGroupIndexParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    s.remove_row_group(p.index).map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `toggle_row_group` — toggle a row group's collapsed state.
pub fn toggle_row_group(state: &mut AppState, params: Value) -> CmdResult {
    let p: RowGroupIndexParams = de(params)?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    Ok(json!(
        s.toggle_row_group(p.index).map_err(|e| e.to_string())?
    ))
}

/// `get_row_groups` — list all row groups for a sheet.
pub fn get_row_groups(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let out: Vec<RowGroupData> = s
        .row_groups()
        .iter()
        .map(|g| RowGroupData {
            start: g.start,
            end: g.end,
            collapsed: g.collapsed,
            level: 0,
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

// ===========================================================================
// Named range commands
// ===========================================================================

#[derive(Serialize)]
struct NamedRangeInfo {
    name: String,
    sheet: Option<String>,
    range: String,
}

/// Format a `Range` as an "A1:B2" string.
fn format_range(range: &Range) -> String {
    let start = format_cell_ref(&range.start);
    let end = format_cell_ref(&range.end);
    if start == end {
        start
    } else {
        format!("{}:{}", start, end)
    }
}

/// Format a `CellRef` as an "A1" string.
fn format_cell_ref(cell: &CellRef) -> String {
    format!("{}{}", col_to_letter(cell.col), cell.row + 1)
}

#[derive(Deserialize)]
struct AddNamedRangeParams {
    name: String,
    range: String,
    sheet: Option<String>,
}

/// `add_named_range` — add a named range to the workbook.
pub fn add_named_range(state: &mut AppState, params: Value) -> CmdResult {
    let p: AddNamedRangeParams = de(params)?;
    let parts: Vec<&str> = p.range.split(':').collect();
    let core_range = if parts.len() == 2 {
        let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
        let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
        Range { start, end }
    } else if parts.len() == 1 {
        let cell = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
        Range {
            start: cell.clone(),
            end: cell,
        }
    } else {
        return Err(format!("Invalid range format '{}'", p.range));
    };
    state
        .workbook
        .named_ranges
        .add(p.name, p.sheet, core_range)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `list_named_ranges` — list all named ranges.
pub fn list_named_ranges(state: &mut AppState, _params: Value) -> CmdResult {
    let out: Vec<NamedRangeInfo> = state
        .workbook
        .named_ranges
        .list()
        .into_iter()
        .map(|nr| NamedRangeInfo {
            name: nr.name.clone(),
            sheet: nr.sheet.clone(),
            range: format_range(&nr.range),
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

/// `remove_named_range` — remove a named range by name.
pub fn remove_named_range(state: &mut AppState, params: Value) -> CmdResult {
    let p: NameOnlyParams = de(params)?;
    state
        .workbook
        .named_ranges
        .remove(&p.name)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `resolve_named_range` — resolve a named range to its sheet and A1 string.
pub fn resolve_named_range(state: &mut AppState, params: Value) -> CmdResult {
    let p: NameOnlyParams = de(params)?;
    let nr = state
        .workbook
        .named_ranges
        .get(&p.name)
        .ok_or_else(|| format!("Named range '{}' not found", p.name))?;
    let info = NamedRangeInfo {
        name: nr.name.clone(),
        sheet: nr.sheet.clone(),
        range: format_range(&nr.range),
    };
    serde_json::to_value(info).map_err(|e| e.to_string())
}

// ===========================================================================
// Conditional format commands
// ===========================================================================

#[derive(Deserialize)]
struct RuleTypeInput {
    kind: String,
    operator: Option<String>,
    value1: Option<f64>,
    value2: Option<f64>,
    text: Option<String>,
    min_color: Option<String>,
    max_color: Option<String>,
    mid_color: Option<String>,
    bar_color: Option<String>,
    icons: Option<Vec<String>>,
    thresholds: Option<Vec<f64>>,
}

#[derive(Deserialize)]
struct StyleInput {
    bold: Option<bool>,
    italic: Option<bool>,
    font_color: Option<String>,
    bg_color: Option<String>,
}

#[derive(Serialize)]
struct RuleOutput {
    kind: String,
    description: String,
    bold: Option<bool>,
    italic: Option<bool>,
    font_color: Option<String>,
    bg_color: Option<String>,
    operator: Option<String>,
    value1: Option<f64>,
    value2: Option<f64>,
    text: Option<String>,
    min_color: Option<String>,
    max_color: Option<String>,
    mid_color: Option<String>,
    bar_color: Option<String>,
    icons: Option<Vec<String>>,
    thresholds: Option<Vec<f64>>,
}

#[derive(Serialize)]
struct ConditionalFormatOutput {
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    rules: Vec<RuleOutput>,
}

fn parse_cf_rule(input: RuleTypeInput, style_input: StyleInput) -> Result<ConditionalRule, String> {
    let rule_type = match input.kind.as_str() {
        "cell_value" => {
            let op_str = input.operator.as_deref().unwrap_or(">");
            let v1 = input.value1.unwrap_or(0.0);
            let v2 = input.value2;
            let operator = match op_str {
                ">" => ComparisonOperator::GreaterThan,
                "<" => ComparisonOperator::LessThan,
                ">=" => ComparisonOperator::GreaterThanOrEqual,
                "<=" => ComparisonOperator::LessThanOrEqual,
                "=" => ComparisonOperator::Equal,
                "!=" => ComparisonOperator::NotEqual,
                "between" => ComparisonOperator::Between,
                "not_between" => ComparisonOperator::NotBetween,
                _ => return Err(format!("Unknown operator: {}", op_str)),
            };
            ConditionalRuleType::CellValue {
                operator,
                value1: v1,
                value2: v2,
            }
        }
        "text_contains" => ConditionalRuleType::TextContains(input.text.unwrap_or_default()),
        "is_blank" => ConditionalRuleType::IsBlank,
        "is_not_blank" => ConditionalRuleType::IsNotBlank,
        "is_error" => ConditionalRuleType::IsError,
        "color_scale" => ConditionalRuleType::ColorScale {
            min_color: input.min_color.unwrap_or_else(|| "#ffffff".to_string()),
            max_color: input.max_color.unwrap_or_else(|| "#ff0000".to_string()),
            mid_color: input.mid_color,
        },
        "data_bar" => ConditionalRuleType::DataBar {
            color: input.bar_color.unwrap_or_else(|| "#4285f4".to_string()),
            max_length_percent: 100,
        },
        "icon_set" => ConditionalRuleType::IconSet {
            icons: input.icons.unwrap_or_else(|| {
                vec![
                    "\u{2191}".to_string(),
                    "\u{2192}".to_string(),
                    "\u{2193}".to_string(),
                ]
            }),
            thresholds: input.thresholds.unwrap_or_else(|| vec![67.0, 33.0]),
        },
        _ => return Err(format!("Unknown rule kind: {}", input.kind)),
    };

    let style = ConditionalStyle {
        bold: style_input.bold,
        italic: style_input.italic,
        font_color: style_input.font_color,
        bg_color: style_input.bg_color,
    };

    Ok(ConditionalRule {
        rule_type,
        style,
        priority: 0,
        stop_if_true: false,
    })
}

fn cf_rule_to_output(rule: &ConditionalRule) -> RuleOutput {
    let (kind, description, operator, value1, value2, text) = match &rule.rule_type {
        ConditionalRuleType::CellValue {
            operator,
            value1,
            value2,
        } => {
            let op_str = match operator {
                ComparisonOperator::GreaterThan => ">",
                ComparisonOperator::LessThan => "<",
                ComparisonOperator::GreaterThanOrEqual => ">=",
                ComparisonOperator::LessThanOrEqual => "<=",
                ComparisonOperator::Equal => "=",
                ComparisonOperator::NotEqual => "!=",
                ComparisonOperator::Between => "between",
                ComparisonOperator::NotBetween => "not between",
            };
            let desc = if let Some(v2) = value2 {
                format!("Cell value {} {} and {}", op_str, value1, v2)
            } else {
                format!("Cell value {} {}", op_str, value1)
            };
            (
                "cell_value".to_string(),
                desc,
                Some(op_str.to_string()),
                Some(*value1),
                *value2,
                None,
            )
        }
        ConditionalRuleType::TextContains(t) => (
            "text_contains".to_string(),
            format!("Text contains \"{}\"", t),
            None,
            None,
            None,
            Some(t.clone()),
        ),
        ConditionalRuleType::IsBlank => (
            "is_blank".to_string(),
            "Cell is blank".to_string(),
            None,
            None,
            None,
            None,
        ),
        ConditionalRuleType::IsNotBlank => (
            "is_not_blank".to_string(),
            "Cell is not blank".to_string(),
            None,
            None,
            None,
            None,
        ),
        ConditionalRuleType::IsError => (
            "is_error".to_string(),
            "Cell is error".to_string(),
            None,
            None,
            None,
            None,
        ),
        ConditionalRuleType::ColorScale {
            min_color,
            max_color,
            mid_color,
        } => {
            return RuleOutput {
                kind: "color_scale".to_string(),
                description: format!("Color scale: {} to {}", min_color, max_color),
                bold: rule.style.bold,
                italic: rule.style.italic,
                font_color: rule.style.font_color.clone(),
                bg_color: rule.style.bg_color.clone(),
                operator: None,
                value1: None,
                value2: None,
                text: None,
                min_color: Some(min_color.clone()),
                max_color: Some(max_color.clone()),
                mid_color: mid_color.clone(),
                bar_color: None,
                icons: None,
                thresholds: None,
            };
        }
        ConditionalRuleType::DataBar { color, .. } => {
            return RuleOutput {
                kind: "data_bar".to_string(),
                description: format!("Data bar: {}", color),
                bold: rule.style.bold,
                italic: rule.style.italic,
                font_color: rule.style.font_color.clone(),
                bg_color: rule.style.bg_color.clone(),
                operator: None,
                value1: None,
                value2: None,
                text: None,
                min_color: None,
                max_color: None,
                mid_color: None,
                bar_color: Some(color.clone()),
                icons: None,
                thresholds: None,
            };
        }
        ConditionalRuleType::IconSet { icons, thresholds } => {
            return RuleOutput {
                kind: "icon_set".to_string(),
                description: format!("Icon set: {}", icons.join(" ")),
                bold: rule.style.bold,
                italic: rule.style.italic,
                font_color: rule.style.font_color.clone(),
                bg_color: rule.style.bg_color.clone(),
                operator: None,
                value1: None,
                value2: None,
                text: None,
                min_color: None,
                max_color: None,
                mid_color: None,
                bar_color: None,
                icons: Some(icons.clone()),
                thresholds: Some(thresholds.clone()),
            };
        }
        _ => (
            "other".to_string(),
            "Custom rule".to_string(),
            None,
            None,
            None,
            None,
        ),
    };

    RuleOutput {
        kind,
        description,
        bold: rule.style.bold,
        italic: rule.style.italic,
        font_color: rule.style.font_color.clone(),
        bg_color: rule.style.bg_color.clone(),
        operator,
        value1,
        value2,
        text,
        min_color: None,
        max_color: None,
        mid_color: None,
        bar_color: None,
        icons: None,
        thresholds: None,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddConditionalFormatParams {
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    rule_type: RuleTypeInput,
    style: StyleInput,
}

/// `add_conditional_format` — add a conditional formatting rule to a range.
pub fn add_conditional_format(state: &mut AppState, params: Value) -> CmdResult {
    let p: AddConditionalFormatParams = de(params)?;
    let rule = parse_cf_rule(p.rule_type, p.style)?;
    state.conditional_formats.add_rule(
        &p.sheet,
        p.start_row,
        p.start_col,
        p.end_row,
        p.end_col,
        rule,
    );
    Ok(Value::Null)
}

/// `list_conditional_formats` — list all conditional format ranges for a sheet.
pub fn list_conditional_formats(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let ranges = state.conditional_formats.list_ranges(&p.sheet);
    let out: Vec<ConditionalFormatOutput> = ranges
        .iter()
        .map(|r| ConditionalFormatOutput {
            start_row: r.start_row,
            start_col: r.start_col,
            end_row: r.end_row,
            end_col: r.end_col,
            rules: r.rules.iter().map(cf_rule_to_output).collect(),
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveConditionalFormatParams {
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
    rule_index: usize,
}

/// `remove_conditional_format` — remove a conditional formatting rule.
pub fn remove_conditional_format(state: &mut AppState, params: Value) -> CmdResult {
    let p: RemoveConditionalFormatParams = de(params)?;
    if state.conditional_formats.remove_rule(
        &p.sheet,
        p.start_row,
        p.start_col,
        p.end_row,
        p.end_col,
        p.rule_index,
    ) {
        Ok(Value::Null)
    } else {
        Err("Rule not found".to_string())
    }
}

// ===========================================================================
// Named function commands
// ===========================================================================

#[derive(Serialize)]
struct NamedFunctionInfo {
    name: String,
    params: Vec<String>,
    body: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct AddNamedFunctionParams {
    name: String,
    params: Vec<String>,
    body: String,
    description: Option<String>,
}

/// `add_named_function` — add a named function to the workbook.
pub fn add_named_function(state: &mut AppState, params: Value) -> CmdResult {
    let p: AddNamedFunctionParams = de(params)?;
    state
        .workbook
        .add_named_function(p.name, p.params, p.body, p.description)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `remove_named_function` — remove a named function by name.
pub fn remove_named_function(state: &mut AppState, params: Value) -> CmdResult {
    let p: NameOnlyParams = de(params)?;
    state
        .workbook
        .remove_named_function(&p.name)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `list_named_functions` — list all named functions in the workbook.
pub fn list_named_functions(state: &mut AppState, _params: Value) -> CmdResult {
    let out: Vec<NamedFunctionInfo> = state
        .workbook
        .list_named_functions()
        .iter()
        .map(|nf| NamedFunctionInfo {
            name: nf.name.clone(),
            params: nf.params.clone(),
            body: nf.body.clone(),
            description: nf.description.clone(),
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

// ===========================================================================
// Filter view commands
// ===========================================================================

#[derive(Serialize)]
struct FilterViewInfo {
    name: String,
    column_filters: HashMap<u32, Vec<String>>,
}

#[derive(Deserialize)]
struct SaveFilterViewParams {
    name: String,
    column_filters: HashMap<u32, Vec<String>>,
}

/// `save_filter_view` — save a named filter view to the workbook.
pub fn save_filter_view(state: &mut AppState, params: Value) -> CmdResult {
    let p: SaveFilterViewParams = de(params)?;
    state
        .workbook
        .filter_views
        .add(p.name, p.column_filters)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

/// `list_filter_views` — list all saved filter views.
pub fn list_filter_views(state: &mut AppState, _params: Value) -> CmdResult {
    let out: Vec<FilterViewInfo> = state
        .workbook
        .filter_views
        .list()
        .iter()
        .map(|v| FilterViewInfo {
            name: v.name.clone(),
            column_filters: v.column_filters.clone(),
        })
        .collect();
    serde_json::to_value(out).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct ApplyFilterViewParams {
    sheet: String,
    name: String,
}

/// `apply_filter_view` — apply a saved filter view, return rows hidden.
pub fn apply_filter_view(state: &mut AppState, params: Value) -> CmdResult {
    let p: ApplyFilterViewParams = de(params)?;
    let view = state
        .workbook
        .filter_views
        .get(&p.name)
        .cloned()
        .ok_or_else(|| format!("filter view '{}' not found", p.name))?;
    let s = state
        .workbook
        .get_sheet_mut(&p.sheet)
        .map_err(|e| e.to_string())?;
    let hidden =
        lattice_core::filter_view::apply_filter_view(s, &view).map_err(|e| e.to_string())?;
    Ok(json!(hidden))
}

/// `delete_filter_view` — delete a saved filter view by name.
pub fn delete_filter_view(state: &mut AppState, params: Value) -> CmdResult {
    let p: NameOnlyParams = de(params)?;
    state
        .workbook
        .filter_views
        .remove(&p.name)
        .map_err(|e| e.to_string())?;
    Ok(Value::Null)
}

// ===========================================================================
// Pivot table commands
// ===========================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetSheetHeadersParams {
    sheet: String,
    header_row: u32,
}

/// `get_sheet_headers` — return the values in a given row as strings.
pub fn get_sheet_headers(state: &mut AppState, params: Value) -> CmdResult {
    let p: GetSheetHeadersParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;
    let (_, max_col) = s.used_range();
    let mut headers = Vec::new();
    for col in 0..=max_col {
        let value = s
            .get_cell(p.header_row, col)
            .map(|c| cell_value_to_display(&c.value))
            .unwrap_or_default();
        headers.push(value);
    }
    serde_json::to_value(headers).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct PivotValueInput {
    col: u32,
    aggregation: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePivotTableParams {
    sheet: String,
    source_range: String,
    row_fields: Vec<u32>,
    value_fields: Vec<PivotValueInput>,
    target_sheet: String,
}

/// `create_pivot_table` — build a pivot table and write it to a target sheet.
pub fn create_pivot_table(state: &mut AppState, params: Value) -> CmdResult {
    use lattice_core::{PivotAggregation, PivotConfig, PivotValue, generate_pivot};

    let p: CreatePivotTableParams = de(params)?;

    let parts: Vec<&str> = p.source_range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid source range '{}': expected 'A1:B2' format",
            p.source_range
        ));
    }
    let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
    let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
    let range = Range { start, end };

    let pivot_values: Vec<PivotValue> = p
        .value_fields
        .iter()
        .map(|vf| {
            let agg = match vf.aggregation.to_lowercase().as_str() {
                "sum" => PivotAggregation::Sum,
                "count" => PivotAggregation::Count,
                "average" => PivotAggregation::Average,
                "min" => PivotAggregation::Min,
                "max" => PivotAggregation::Max,
                "countdistinct" => PivotAggregation::CountDistinct,
                _ => PivotAggregation::Sum,
            };
            PivotValue {
                source_col: vf.col,
                aggregation: agg,
                label: None,
            }
        })
        .collect();

    let config = PivotConfig {
        source_sheet: p.sheet.clone(),
        source_range: range,
        row_fields: p.row_fields,
        col_fields: vec![],
        value_fields: pivot_values,
    };

    let result = generate_pivot(&state.workbook, &config).map_err(|e| e.to_string())?;

    if state.workbook.get_sheet(&p.target_sheet).is_err() {
        state
            .workbook
            .add_sheet(&p.target_sheet)
            .map_err(|e| e.to_string())?;
    }
    let target = state
        .workbook
        .get_sheet_mut(&p.target_sheet)
        .map_err(|e| e.to_string())?;

    for (col, header) in result.headers.iter().enumerate() {
        target.set_value(0, col as u32, CellValue::Text(header.clone()));
    }
    for (row_idx, row) in result.rows.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            target.set_value((row_idx + 1) as u32, col_idx as u32, value.clone());
        }
    }
    Ok(Value::Null)
}

// ===========================================================================
// Column statistics
// ===========================================================================

#[derive(Serialize)]
struct ColumnStats {
    count: u32,
    unique: u32,
    sum: Option<f64>,
    average: Option<f64>,
    median: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    std_dev: Option<f64>,
    histogram: Vec<u32>,
}

/// `get_column_stats` — compute descriptive statistics for a column.
pub fn get_column_stats(state: &mut AppState, params: Value) -> CmdResult {
    let p: ColParams = de(params)?;
    let s = state
        .workbook
        .get_sheet(&p.sheet)
        .map_err(|e| e.to_string())?;

    let mut numbers: Vec<f64> = Vec::new();
    let mut values_set: HashSet<String> = HashSet::new();
    let mut count = 0u32;

    for (&(_, c), cell) in s.cells() {
        if c != p.col {
            continue;
        }
        match &cell.value {
            CellValue::Empty => {}
            CellValue::Number(n) => {
                count += 1;
                numbers.push(*n);
                values_set.insert(n.to_bits().to_string());
            }
            CellValue::Text(t) => {
                count += 1;
                values_set.insert(t.clone());
            }
            CellValue::Boolean(b) | CellValue::Checkbox(b) => {
                count += 1;
                values_set.insert(b.to_string());
            }
            CellValue::Date(d) => {
                count += 1;
                values_set.insert(d.clone());
            }
            _ => {
                count += 1;
            }
        }
    }

    if numbers.is_empty() {
        let stats = ColumnStats {
            count,
            unique: values_set.len() as u32,
            sum: None,
            average: None,
            median: None,
            min: None,
            max: None,
            std_dev: None,
            histogram: Vec::new(),
        };
        return serde_json::to_value(stats).map_err(|e| e.to_string());
    }

    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = numbers.len();
    let sum: f64 = numbers.iter().sum();
    let mean = sum / n as f64;
    let min = numbers[0];
    let max = numbers[n - 1];
    let median = if n.is_multiple_of(2) {
        (numbers[n / 2 - 1] + numbers[n / 2]) / 2.0
    } else {
        numbers[n / 2]
    };
    let variance: f64 = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    let histogram = if (max - min).abs() < f64::EPSILON {
        vec![n as u32]
    } else {
        let num_buckets = 10usize;
        let mut buckets = vec![0u32; num_buckets];
        let range = max - min;
        for &v in &numbers {
            let idx = ((v - min) / range * (num_buckets as f64 - 1.0)).round() as usize;
            let idx = idx.min(num_buckets - 1);
            buckets[idx] += 1;
        }
        buckets
    };

    let stats = ColumnStats {
        count,
        unique: values_set.len() as u32,
        sum: Some(sum),
        average: Some(mean),
        median: Some(median),
        min: Some(min),
        max: Some(max),
        std_dev: Some(std_dev),
        histogram,
    };
    serde_json::to_value(stats).map_err(|e| e.to_string())
}

// ===========================================================================
// Export commands
// ===========================================================================

/// `export_html` — export a sheet as print-ready HTML.
pub fn export_html(state: &mut AppState, params: Value) -> CmdResult {
    let p: SheetOnlyParams = de(params)?;
    let sheet_name = if p.sheet.is_empty() {
        None
    } else {
        Some(p.sheet.as_str())
    };
    let html = lattice_io::export_print_html(&state.workbook, sheet_name, None)
        .map_err(|e| e.to_string())?;
    Ok(json!(html))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrintSettingsParams {
    paper_size: String,
    orientation: String,
    show_gridlines: bool,
    show_headers: bool,
    scale: f64,
    margins: String,
    custom_margins: Option<[f64; 4]>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportPrintHtmlParams {
    sheet: String,
    settings: PrintSettingsParams,
}

/// `export_print_html` — export a sheet as print HTML with custom settings.
pub fn export_print_html(state: &mut AppState, params: Value) -> CmdResult {
    let p: ExportPrintHtmlParams = de(params)?;
    let sheet_name = if p.sheet.is_empty() {
        None
    } else {
        Some(p.sheet.as_str())
    };
    let settings = lattice_io::PrintSettings {
        paper_size: p.settings.paper_size,
        orientation: p.settings.orientation,
        show_gridlines: p.settings.show_gridlines,
        show_headers: p.settings.show_headers,
        scale: p.settings.scale,
        margins: p.settings.margins,
        custom_margins: p.settings.custom_margins,
    };
    let html = lattice_io::export_print_html(&state.workbook, sheet_name, Some(&settings))
        .map_err(|e| e.to_string())?;
    Ok(json!(html))
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Convert a `CellValue` to a display string (port of `cell_value_to_display`).
fn cell_value_to_display(val: &CellValue) -> String {
    match val {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) => b.to_string().to_uppercase(),
        CellValue::Checkbox(b) => b.to_string().to_uppercase(),
        CellValue::Empty => String::new(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".to_string(),
        CellValue::Lambda { .. } => "{lambda}".to_string(),
    }
}

/// Deserialize command params, converting any error to a readable string.
fn de<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("invalid arguments: {}", e))
}
