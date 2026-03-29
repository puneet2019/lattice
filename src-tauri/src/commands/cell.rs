use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::formula::evaluator::SimpleEvaluator;
use lattice_core::named_function::NamedFunction;
use lattice_core::{
    BorderStyle, CellRef, CellValue, FormulaEngine, NumberFormat, Operation, SheetResolver,
    Workbook, format_value,
};

use crate::state::AppState;

/// A [`SheetResolver`] wrapper around a [`Workbook`] that provides a real
/// `import_range` implementation using `lattice_io::read_xlsx`.
///
/// The core engine (`lattice-core`) is I/O-free, so its `Workbook`
/// implementation of `import_range` always returns `None`.  This wrapper
/// is used by the Tauri layer to resolve `IMPORTRANGE` formulas by
/// reading the external file from disk.
struct ImportRangeResolver<'a> {
    workbook: &'a Workbook,
}

impl<'a> SheetResolver for ImportRangeResolver<'a> {
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

    fn import_range(&self, file_path: &str, range_string: &str) -> Option<CellValue> {
        import_range_from_file(file_path, range_string)
    }
}

/// Open an xlsx file and extract the requested range as a `CellValue::Array`.
///
/// The `range_string` should be in `"SheetName!A1:C10"` format.  Returns
/// `None` if the file cannot be read or the range is invalid.
fn import_range_from_file(file_path: &str, range_string: &str) -> Option<CellValue> {
    // Parse "SheetName!A1:C10" -> (sheet_name, start_ref, end_ref)
    let excl = range_string.find('!')?;
    let sheet_name = range_string[..excl].trim();
    if sheet_name.is_empty() {
        return None;
    }
    let cell_range = &range_string[excl + 1..];

    // Split on ':' for start and end refs.  If there is no colon, treat
    // the whole string as a single cell reference (start == end).
    let (start_str, end_str) = if let Some(colon) = cell_range.find(':') {
        (
            cell_range[..colon].trim().to_string(),
            cell_range[colon + 1..].trim().to_string(),
        )
    } else {
        let single = cell_range.trim().to_string();
        (single.clone(), single)
    };

    let start = CellRef::parse(&start_str).ok()?;
    let end = CellRef::parse(&end_str).ok()?;

    // Read the workbook from disk.
    let path = Path::new(file_path);
    let ext_wb = lattice_io::xlsx_reader::read_xlsx(path).ok()?;
    let ext_sheet = ext_wb.get_sheet(sheet_name).ok()?;

    let min_row = start.row.min(end.row);
    let max_row = start.row.max(end.row);
    let min_col = start.col.min(end.col);
    let max_col = start.col.max(end.col);

    let mut rows: Vec<Vec<CellValue>> = Vec::new();
    for r in min_row..=max_row {
        let mut row_vals: Vec<CellValue> = Vec::new();
        for c in min_col..=max_col {
            let val = ext_sheet
                .get_cell(r, c)
                .map(|cell| cell.value.clone())
                .unwrap_or(CellValue::Empty);
            row_vals.push(val);
        }
        rows.push(row_vals);
    }

    Some(CellValue::Array(rows))
}

/// A single border edge serialized for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderEdgeData {
    pub style: String,
    pub color: String,
}

/// Cell borders serialized for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellBordersData {
    pub top: Option<BorderEdgeData>,
    pub bottom: Option<BorderEdgeData>,
    pub left: Option<BorderEdgeData>,
    pub right: Option<BorderEdgeData>,
}

/// Serializable cell data returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    /// The display value as a string (formatted according to number_format).
    pub value: String,
    /// The raw formula text (without leading `=`), if any.
    pub formula: Option<String>,
    /// Format identifier (style_id from the cell).
    pub format_id: u32,
    /// Whether the cell is bold.
    pub bold: bool,
    /// Whether the cell is italic.
    pub italic: bool,
    /// Whether the cell is underlined.
    pub underline: bool,
    /// Whether the cell has strikethrough.
    pub strikethrough: bool,
    /// The number format pattern string, if any.
    pub number_format: Option<String>,
    /// Font color as CSS hex string, or null for theme default.
    pub font_color: Option<String>,
    /// Background/fill color as CSS hex string, if set.
    pub bg_color: Option<String>,
    /// Font family name.
    pub font_family: String,
    /// Horizontal alignment: "left", "center", or "right".
    pub h_align: String,
    /// Vertical alignment: "top", "middle", or "bottom".
    pub v_align: String,
    /// Font size in points.
    pub font_size: f64,
    /// Text wrapping mode: "Overflow", "Wrap", or "Clip".
    pub text_wrap: String,
    /// Cell border configuration.
    pub borders: Option<CellBordersData>,
    /// Text rotation in degrees (0-360, 0 = normal).
    pub text_rotation: i16,
    /// Number of indent levels (0 = none).
    pub indent: u8,
    /// Optional comment / note text.
    pub comment: Option<String>,
}

/// Get a single cell's data.
#[tauri::command]
pub async fn get_cell(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<Option<CellData>, String> {
    let workbook = state.workbook.read().await;
    let cell = workbook
        .get_cell(&sheet, row, col)
        .map_err(|e| e.to_string())?;

    Ok(cell.map(cell_to_data))
}

/// Set a cell's value (and optionally a formula).
///
/// When a formula is provided (value starting with `=`), the formula is
/// evaluated using [`SimpleEvaluator`] and the computed result is stored
/// as the cell's value. The raw formula text is preserved on the cell so
/// it can be shown in the formula bar.
#[tauri::command]
pub async fn set_cell(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
    value: String,
    formula: Option<String>,
) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;

    // Record the old value for undo.
    let old_value = workbook
        .get_cell(&sheet, row, col)
        .map_err(|e| e.to_string())?
        .map(|c| c.value.clone())
        .unwrap_or(CellValue::Empty);

    let new_value = if let Some(ref formula_text) = formula {
        // Evaluate the formula to get the computed value.
        let evaluator = SimpleEvaluator;
        let resolver = ImportRangeResolver {
            workbook: &workbook,
        };
        let eval_result = {
            let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;
            evaluator.evaluate_with_context(formula_text, s, Some(&resolver))
        };
        match eval_result {
            Ok(v) => v,
            Err(e) => CellValue::Error(map_error_to_cell_error(&e)),
        }
    } else {
        let (val, _) = parse_cell_value(&value);
        val
    };

    // Set the cell value on the sheet.
    workbook
        .set_cell(&sheet, row, col, new_value.clone())
        .map_err(|e| e.to_string())?;

    // If a formula was provided, store it on the cell.
    if let Some(ref formula_text) = formula {
        let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
        if let Some(cell) = s.get_cell(row, col) {
            let mut cell = cell.clone();
            cell.formula = Some(formula_text.clone());
            s.set_cell(row, col, cell);
        }
    }

    // If parsing detected a number format (e.g. percentage), apply it.
    if formula.is_none() {
        let (_, number_format) = parse_cell_value(&value);
        if let Some(fmt) = number_format {
            let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
            if let Some(cell) = s.get_cell(row, col) {
                let mut cell = cell.clone();
                cell.format.number_format = Some(fmt);
                s.set_cell(row, col, cell);
            }
        }
    }

    // Push to undo stack.
    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::SetCell {
        sheet: sheet.clone(),
        row,
        col,
        old_value,
        new_value,
    });

    // Recalculate dependent cells: any cell in this sheet that has a formula
    // might depend on the cell we just changed. Re-evaluate all formula cells.
    recalculate_formulas(&mut workbook, &sheet);

    Ok(())
}

/// Re-evaluate all formula cells across ALL sheets in the workbook.
///
/// This is a simple brute-force recalculation. A future optimisation would
/// build a dependency graph and only recalculate affected cells. The
/// `_changed_sheet` parameter is kept for potential future use but currently
/// all sheets are recalculated to ensure cross-sheet references stay correct.
fn recalculate_formulas(workbook: &mut lattice_core::Workbook, _changed_sheet: &str) {
    let all_sheet_names = workbook.sheet_names();
    let evaluator = SimpleEvaluator;

    for sheet_name in &all_sheet_names {
        // Collect all cells with formulas first (to avoid borrow conflicts).
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
            let resolver = ImportRangeResolver { workbook };
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
            // Update the value without clearing the formula.
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

/// Get a rectangular range of cells.
#[tauri::command]
pub async fn get_range(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    start_col: u32,
    end_row: u32,
    end_col: u32,
) -> Result<Vec<Vec<Option<CellData>>>, String> {
    let workbook = state.workbook.read().await;
    let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    for r in start_row..=end_row {
        let mut row_data = Vec::new();
        for c in start_col..=end_col {
            let cell = s.get_cell(r, c);
            row_data.push(cell.map(cell_to_data));
        }
        rows.push(row_data);
    }

    Ok(rows)
}

/// Convert a core `Border` to a frontend-serializable `BorderEdgeData`.
fn border_to_data(border: &lattice_core::Border) -> BorderEdgeData {
    BorderEdgeData {
        style: match border.style {
            BorderStyle::None => "none".to_string(),
            BorderStyle::Thin => "thin".to_string(),
            BorderStyle::Medium => "medium".to_string(),
            BorderStyle::Thick => "thick".to_string(),
            BorderStyle::Dashed => "dashed".to_string(),
            BorderStyle::Dotted => "dotted".to_string(),
            BorderStyle::Double => "double".to_string(),
        },
        color: border.color.clone(),
    }
}

/// Convert a core `Cell` into a frontend `CellData` with all format fields.
fn cell_to_data(c: &lattice_core::Cell) -> CellData {
    let borders = {
        let b = &c.format.borders;
        let has_any =
            b.top.is_some() || b.bottom.is_some() || b.left.is_some() || b.right.is_some();
        if has_any {
            Some(CellBordersData {
                top: b.top.as_ref().map(border_to_data),
                bottom: b.bottom.as_ref().map(border_to_data),
                left: b.left.as_ref().map(border_to_data),
                right: b.right.as_ref().map(border_to_data),
            })
        } else {
            None
        }
    };

    CellData {
        value: format_cell_display(&c.value, &c.format.number_format),
        formula: c.formula.clone(),
        format_id: c.style_id,
        bold: c.format.bold,
        italic: c.format.italic,
        underline: c.format.underline,
        strikethrough: c.format.strikethrough,
        number_format: c.format.number_format.clone(),
        font_color: c.format.font_color.clone(),
        bg_color: c.format.bg_color.clone(),
        font_family: c.format.font_family.clone(),
        h_align: match c.format.h_align {
            lattice_core::HAlign::Left => "left".to_string(),
            lattice_core::HAlign::Center => "center".to_string(),
            lattice_core::HAlign::Right => "right".to_string(),
        },
        v_align: match c.format.v_align {
            lattice_core::VAlign::Top => "top".to_string(),
            lattice_core::VAlign::Middle => "middle".to_string(),
            lattice_core::VAlign::Bottom => "bottom".to_string(),
        },
        font_size: c.format.font_size,
        text_wrap: match c.format.text_wrap {
            lattice_core::TextWrap::Overflow => "Overflow".to_string(),
            lattice_core::TextWrap::Wrap => "Wrap".to_string(),
            lattice_core::TextWrap::Clip => "Clip".to_string(),
        },
        borders,
        text_rotation: c.format.text_rotation,
        indent: c.format.indent,
        comment: c.comment.clone(),
    }
}

/// Format a cell value for display, using the core engine's `format_value`.
///
/// When a number_format pattern is set, uses `NumberFormat::Custom` (which
/// currently falls back to General). When no pattern is set, uses General.
fn format_cell_display(value: &CellValue, number_format: &Option<String>) -> String {
    let fmt = match number_format {
        Some(pattern) => NumberFormat::Custom(pattern.clone()),
        None => NumberFormat::General,
    };
    format_value(value, &fmt)
}

// ---------------------------------------------------------------------------
// Comment commands
// ---------------------------------------------------------------------------

/// Set a comment/note on a cell.
#[tauri::command]
pub async fn set_comment(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
    text: String,
) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;
    let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.set_comment(row, col, text);
    Ok(())
}

/// Get the comment/note on a cell, if any.
#[tauri::command]
pub async fn get_comment(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<Option<String>, String> {
    let workbook = state.workbook.read().await;
    let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;
    Ok(s.get_comment(row, col).map(|s| s.to_string()))
}

/// Remove the comment/note from a cell.
#[tauri::command]
pub async fn remove_comment(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<(), String> {
    let mut workbook = state.workbook.write().await;
    let s = workbook.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.remove_comment(row, col);
    Ok(())
}

// ---------------------------------------------------------------------------
// Protection commands
// ---------------------------------------------------------------------------

/// Serializable sheet protection info returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetProtectionData {
    pub is_protected: bool,
    pub allow_select: bool,
    pub allow_sort: bool,
    pub allow_filter: bool,
}

/// Check whether a specific cell is protected (falls within a protected range).
#[tauri::command]
pub async fn is_cell_protected(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    col: u32,
) -> Result<bool, String> {
    let workbook = state.workbook.read().await;
    let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;
    // A cell is "protected" when the sheet is protected AND the cell is in a protected range
    // (or when the sheet is protected and there are no explicit ranges — all cells are protected).
    if !s.is_protected() {
        return Ok(false);
    }
    let ranges = s.protected_ranges();
    if ranges.is_empty() {
        // Sheet protection with no explicit ranges = all cells are protected.
        return Ok(true);
    }
    Ok(s.is_cell_protected(row, col))
}

/// Get the sheet-level protection settings.
#[tauri::command]
pub async fn get_sheet_protection(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Option<SheetProtectionData>, String> {
    let workbook = state.workbook.read().await;
    let s = workbook.get_sheet(&sheet).map_err(|e| e.to_string())?;
    Ok(s.protection.as_ref().map(|p| SheetProtectionData {
        is_protected: p.is_protected,
        allow_select: p.allow_select,
        allow_sort: p.allow_sort,
        allow_filter: p.allow_filter,
    }))
}

/// Map a [`LatticeError`] to the most appropriate [`CellError`] variant.
///
/// Formula errors are inspected for keywords (`DIV`, `REF`, `NAME`, `N/A`,
/// `NUM`) to return the correct spreadsheet error type. All other errors
/// default to `CellError::Value`.
fn map_error_to_cell_error(err: &lattice_core::LatticeError) -> lattice_core::CellError {
    use lattice_core::{CellError, LatticeError};

    match err {
        LatticeError::FormulaError(msg) => {
            let upper = msg.to_uppercase();
            if upper.contains("DIV") || upper.contains("DIVISION") {
                CellError::DivZero
            } else if upper.contains("REF") {
                CellError::Ref
            } else if upper.contains("NAME") {
                CellError::Name
            } else if upper.contains("N/A") {
                CellError::NA
            } else if upper.contains("NUM") {
                CellError::Num
            } else {
                CellError::Value
            }
        }
        _ => CellError::Value,
    }
}

/// Parse a string into a `CellValue`, inferring the type.
///
/// Returns `(CellValue, Option<String>)` where the second element is an
/// optional number format pattern to apply to the cell (e.g. "0%" for
/// percentage input).
fn parse_cell_value(s: &str) -> (CellValue, Option<String>) {
    if s.is_empty() {
        return (CellValue::Empty, None);
    }

    // Try boolean.
    match s.to_uppercase().as_str() {
        "TRUE" => return (CellValue::Boolean(true), None),
        "FALSE" => return (CellValue::Boolean(false), None),
        _ => {}
    }

    // Try percentage: trailing `%` means divide by 100 and format as percent.
    if let Some(before_pct) = s.strip_suffix('%') {
        let trimmed = before_pct.trim();
        if let Ok(n) = trimmed.parse::<f64>() {
            return (CellValue::Number(n / 100.0), Some("0%".to_string()));
        }
    }

    // Try number.
    if let Ok(n) = s.parse::<f64>() {
        return (CellValue::Number(n), None);
    }

    // Try currency: leading $, EUR, GBP, JPY symbols with optional commas.
    if let Some(result) = try_parse_currency(s) {
        return result;
    }

    // Try date: various common date formats.
    if let Some(result) = try_parse_date(s) {
        return result;
    }

    // Default to text.
    (CellValue::Text(s.to_string()), None)
}

/// Try to parse a currency string like "$1,234.56", "EUR1234", etc.
///
/// Strips leading currency symbols ($, EUR, GBP, JPY) and commas, then
/// parses the remaining digits as a number. Returns the appropriate
/// number format string for display.
fn try_parse_currency(s: &str) -> Option<(CellValue, Option<String>)> {
    let trimmed = s.trim();

    // Detect leading currency symbol and determine format pattern.
    let (rest, fmt) = if let Some(r) = trimmed.strip_prefix('$') {
        (r, "$#,##0.00")
    } else if let Some(r) = trimmed.strip_prefix('\u{20AC}') {
        // Euro sign
        (r, "\u{20AC}#,##0.00")
    } else if let Some(r) = trimmed.strip_prefix('\u{00A3}') {
        // Pound sign
        (r, "\u{00A3}#,##0.00")
    } else if let Some(r) = trimmed.strip_prefix('\u{00A5}') {
        // Yen sign
        (r, "\u{00A5}#,##0.00")
    } else {
        return None;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    // Strip commas (thousands separators) and parse the number.
    let without_commas: String = rest.chars().filter(|&c| c != ',').collect();
    if let Ok(n) = without_commas.parse::<f64>() {
        Some((CellValue::Number(n), Some(fmt.to_string())))
    } else {
        None
    }
}

/// Try to parse a date string into an Excel serial date number.
///
/// Supported formats:
/// - `M/D/YYYY` or `MM/DD/YYYY` (e.g., "1/2/2024", "12/31/2025")
/// - `YYYY-MM-DD` (ISO 8601, e.g., "2024-01-15")
/// - `D-Mon-YYYY` or `DD-MMM-YYYY` (e.g., "15-Jan-2024")
fn try_parse_date(s: &str) -> Option<(CellValue, Option<String>)> {
    let trimmed = s.trim();

    // Try M/D/YYYY or MM/DD/YYYY
    if let Some(result) = try_parse_mdy_slash(trimmed) {
        return Some(result);
    }

    // Try YYYY-MM-DD (ISO 8601)
    if let Some(result) = try_parse_iso_date(trimmed) {
        return Some(result);
    }

    // Try D-Mon-YYYY or DD-MMM-YYYY
    if let Some(result) = try_parse_dmy_month_name(trimmed) {
        return Some(result);
    }

    None
}

/// Parse M/D/YYYY or MM/DD/YYYY format.
fn try_parse_mdy_slash(s: &str) -> Option<(CellValue, Option<String>)> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: i32 = parts[2].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let serial = date_to_serial(year, month, day);
    Some((
        CellValue::Number(serial as f64),
        Some("MM/DD/YYYY".to_string()),
    ))
}

/// Parse YYYY-MM-DD (ISO 8601) format.
fn try_parse_iso_date(s: &str) -> Option<(CellValue, Option<String>)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    // First part must be 4 digits (year) to distinguish from D-Mon-YYYY.
    if parts[0].len() != 4 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let serial = date_to_serial(year, month, day);
    Some((
        CellValue::Number(serial as f64),
        Some("MM/DD/YYYY".to_string()),
    ))
}

/// Parse D-Mon-YYYY or DD-MMM-YYYY format (e.g., "15-Jan-2024").
fn try_parse_dmy_month_name(s: &str) -> Option<(CellValue, Option<String>)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let day: u32 = parts[0].parse().ok()?;
    let month = month_name_to_number(parts[1])?;
    let year: i32 = parts[2].parse().ok()?;

    if !is_valid_date(year, month, day) {
        return None;
    }

    let serial = date_to_serial(year, month, day);
    Some((
        CellValue::Number(serial as f64),
        Some("MM/DD/YYYY".to_string()),
    ))
}

/// Convert a 3-letter month abbreviation to its 1-based month number.
fn month_name_to_number(name: &str) -> Option<u32> {
    match name.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

/// Check whether a date is valid (within reasonable spreadsheet bounds).
fn is_valid_date(year: i32, month: u32, day: u32) -> bool {
    if !(1900..=9999).contains(&year) {
        return false;
    }
    if !(1..=12).contains(&month) {
        return false;
    }
    if !(1..=31).contains(&day) {
        return false;
    }
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap { 29 } else { 28 }
        }
        _ => return false,
    };
    day <= days_in_month
}

/// Convert a date to an Excel serial date number.
///
/// Excel serial dates count days from 1900-01-01 as day 1, with the
/// intentional Lotus 1-2-3 bug that treats 1900 as a leap year (day 60 =
/// Feb 29, 1900 which doesn't exist).
fn date_to_serial(year: i32, month: u32, day: u32) -> i32 {
    let y = year as i64;
    let m = month as i64;
    let days_to_year = |yr: i64| -> i64 {
        let yr = yr - 1;
        yr * 365 + yr / 4 - yr / 100 + yr / 400
    };
    let month_days: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mut day_of_year: i64 = 0;
    for (i, &md) in month_days.iter().enumerate().take((m - 1) as usize) {
        day_of_year += md;
        if i == 1 && is_leap {
            day_of_year += 1;
        }
    }
    day_of_year += day as i64;
    let abs_days = days_to_year(y) + day_of_year;
    let base = days_to_year(1900) + 1;
    let mut serial = (abs_days - base) + 1;
    // Lotus 1-2-3 bug: Excel thinks 1900-02-29 exists (serial 60).
    if serial >= 60 {
        serial += 1;
    }
    serial as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_cell_value basics
    // -----------------------------------------------------------------------

    #[test]
    fn parse_empty() {
        let (val, fmt) = parse_cell_value("");
        assert!(matches!(val, CellValue::Empty));
        assert!(fmt.is_none());
    }

    #[test]
    fn parse_boolean() {
        assert!(matches!(
            parse_cell_value("TRUE").0,
            CellValue::Boolean(true)
        ));
        assert!(matches!(
            parse_cell_value("false").0,
            CellValue::Boolean(false)
        ));
    }

    #[test]
    fn parse_percentage() {
        let (val, fmt) = parse_cell_value("50%");
        match val {
            CellValue::Number(n) => assert!((n - 0.5).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("0%".to_string()));
    }

    #[test]
    fn parse_plain_number() {
        let (val, fmt) = parse_cell_value("42.5");
        match val {
            CellValue::Number(n) => assert!((n - 42.5).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
        assert!(fmt.is_none());
    }

    // -----------------------------------------------------------------------
    // Currency parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_currency_dollar() {
        let (val, fmt) = parse_cell_value("$1,234.56");
        match val {
            CellValue::Number(n) => assert!((n - 1234.56).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("$#,##0.00".to_string()));
    }

    #[test]
    fn parse_currency_dollar_no_commas() {
        let (val, _fmt) = parse_cell_value("$99.99");
        match val {
            CellValue::Number(n) => assert!((n - 99.99).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn parse_currency_euro() {
        let (val, fmt) = parse_cell_value("\u{20AC}500");
        match val {
            CellValue::Number(n) => assert!((n - 500.0).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("\u{20AC}#,##0.00".to_string()));
    }

    #[test]
    fn parse_currency_pound() {
        let (val, _fmt) = parse_cell_value("\u{00A3}1,000");
        match val {
            CellValue::Number(n) => assert!((n - 1000.0).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn parse_currency_yen() {
        let (val, _fmt) = parse_cell_value("\u{00A5}2500");
        match val {
            CellValue::Number(n) => assert!((n - 2500.0).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn parse_currency_invalid() {
        // Dollar sign with no digits
        let (val, _) = parse_cell_value("$");
        assert!(matches!(val, CellValue::Text(_)));
    }

    #[test]
    fn parse_currency_with_spaces() {
        let (val, _) = parse_cell_value("$ 1,234.56");
        match val {
            CellValue::Number(n) => assert!((n - 1234.56).abs() < 1e-10),
            _ => panic!("expected Number"),
        }
    }

    // -----------------------------------------------------------------------
    // Date parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_date_mdy_slash() {
        let (val, fmt) = parse_cell_value("1/2/2024");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2024, 1, 2)),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("MM/DD/YYYY".to_string()));
    }

    #[test]
    fn parse_date_mdy_slash_padded() {
        let (val, _) = parse_cell_value("12/31/2025");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2025, 12, 31)),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn parse_date_iso() {
        let (val, fmt) = parse_cell_value("2024-01-15");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2024, 1, 15)),
            _ => panic!("expected Number"),
        }
        assert_eq!(fmt, Some("MM/DD/YYYY".to_string()));
    }

    #[test]
    fn parse_date_dmy_month_name() {
        let (val, _) = parse_cell_value("15-Jan-2024");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2024, 1, 15)),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn parse_date_dmy_month_name_lowercase() {
        let (val, _) = parse_cell_value("1-feb-2024");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2024, 2, 1)),
            _ => panic!("expected Number"),
        }
    }

    #[test]
    fn parse_date_invalid_month() {
        let (val, _) = parse_cell_value("13/1/2024");
        assert!(matches!(val, CellValue::Text(_)));
    }

    #[test]
    fn parse_date_invalid_day() {
        let (val, _) = parse_cell_value("2/30/2024");
        assert!(matches!(val, CellValue::Text(_)));
    }

    #[test]
    fn parse_date_feb_29_leap() {
        // 2024 is a leap year
        let (val, _) = parse_cell_value("2/29/2024");
        match val {
            CellValue::Number(n) => assert_eq!(n as i32, date_to_serial(2024, 2, 29)),
            _ => panic!("expected Number for leap day"),
        }
    }

    #[test]
    fn parse_date_feb_29_non_leap() {
        // 2023 is not a leap year
        let (val, _) = parse_cell_value("2/29/2023");
        assert!(matches!(val, CellValue::Text(_)));
    }

    #[test]
    fn parse_date_year_before_1900() {
        let (val, _) = parse_cell_value("1/1/1899");
        assert!(matches!(val, CellValue::Text(_)));
    }

    // -----------------------------------------------------------------------
    // date_to_serial known values (cross-check with evaluator)
    // -----------------------------------------------------------------------

    #[test]
    fn date_serial_known_dates() {
        assert_eq!(date_to_serial(1900, 1, 1), 1);
        assert_eq!(date_to_serial(1900, 1, 31), 31);
        assert_eq!(date_to_serial(1900, 2, 28), 59);
        assert_eq!(date_to_serial(1900, 3, 1), 61);
        assert_eq!(date_to_serial(2000, 1, 1), 36526);
        assert_eq!(date_to_serial(2024, 1, 1), 45292);
    }

    // -----------------------------------------------------------------------
    // Strings that should remain text
    // -----------------------------------------------------------------------

    #[test]
    fn parse_plain_text() {
        let (val, _) = parse_cell_value("hello world");
        match val {
            CellValue::Text(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn parse_not_a_date() {
        // "100/200/300" has month > 12 — should be text
        let (val, _) = parse_cell_value("100/200/300");
        assert!(matches!(val, CellValue::Text(_)));
    }
}
