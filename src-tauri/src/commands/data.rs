use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_core::{CellRef, Operation, SortDirection, SortKey};

use crate::state::AppState;

/// Find text in a sheet.
///
/// When `case_sensitive` is false, both the query and cell text are
/// compared in lowercase.
#[tauri::command]
pub async fn find_in_sheet(
    state: State<'_, AppState>,
    sheet: String,
    query: String,
    case_sensitive: Option<bool>,
) -> Result<Vec<(u32, u32)>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;

    let case_sensitive = case_sensitive.unwrap_or(true);
    let query_lower = if case_sensitive {
        query.clone()
    } else {
        query.to_lowercase()
    };

    let mut matches = Vec::new();
    for (&(row, col), cell) in s.cells() {
        let text = match &cell.value {
            lattice_core::CellValue::Text(t) => Some(t.as_str()),
            lattice_core::CellValue::Number(n) => {
                // Numbers are compared as their string representation.
                // We need to allocate, so handle below.
                let s = n.to_string();
                if case_sensitive {
                    if s.contains(&query) {
                        matches.push((row, col));
                    }
                } else if s.to_lowercase().contains(&query_lower) {
                    matches.push((row, col));
                }
                None
            }
            _ => None,
        };

        if let Some(t) = text {
            if case_sensitive {
                if t.contains(&query) {
                    matches.push((row, col));
                }
            } else if t.to_lowercase().contains(&query_lower) {
                matches.push((row, col));
            }
        }
    }
    Ok(matches)
}

/// Duplicate a sheet.
#[tauri::command]
pub async fn duplicate_sheet(
    state: State<'_, AppState>,
    source: String,
    new_name: String,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let sheet = wb.get_sheet(&source).map_err(|e| e.to_string())?.clone();
    wb.add_sheet(&new_name).map_err(|e| e.to_string())?;
    let dest = wb.get_sheet_mut(&new_name).map_err(|e| e.to_string())?;
    for (&(row, col), cell) in sheet.cells() {
        dest.set_cell(row, col, cell.clone());
    }
    Ok(())
}

/// Insert rows at the given position.
///
/// Pushes an `InsertRows` operation to the undo stack so the change
/// is reversible.
#[tauri::command]
pub async fn insert_rows(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.insert_rows(row, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::InsertRows { sheet, row, count });

    Ok(())
}

/// Delete rows at the given position.
///
/// Saves the deleted cells so undo can restore them. Pushes a
/// `DeleteRows` operation to the undo stack.
#[tauri::command]
pub async fn delete_rows(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;

    // Save cells in the rows being deleted for undo.
    let end_row = row + count;
    let deleted_cells: Vec<(u32, u32, lattice_core::Cell)> = s
        .cells()
        .iter()
        .filter(|((r, _), _)| *r >= row && *r < end_row)
        .map(|((r, c), cell)| (*r, *c, cell.clone()))
        .collect();

    s.delete_rows(row, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::DeleteRows {
        sheet,
        row,
        count,
        deleted_cells,
    });

    Ok(())
}

/// Insert columns at the given position.
///
/// Pushes an `InsertCols` operation to the undo stack so the change
/// is reversible.
#[tauri::command]
pub async fn insert_cols(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.insert_cols(col, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::InsertCols { sheet, col, count });

    Ok(())
}

/// Delete columns at the given position.
///
/// Saves the deleted cells so undo can restore them. Pushes a
/// `DeleteCols` operation to the undo stack.
#[tauri::command]
pub async fn delete_cols(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;

    // Save cells in the columns being deleted for undo.
    let end_col = col + count;
    let deleted_cells: Vec<(u32, u32, lattice_core::Cell)> = s
        .cells()
        .iter()
        .filter(|((_, c), _)| *c >= col && *c < end_col)
        .map(|((r, c), cell)| (*r, *c, cell.clone()))
        .collect();

    s.delete_cols(col, count);

    let mut stack = state.undo_stack.write().await;
    stack.push(Operation::DeleteCols {
        sheet,
        col,
        count,
        deleted_cells,
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Hide / Unhide rows and columns
// ---------------------------------------------------------------------------

/// Hide rows starting at `start_row` for `count` rows.
#[tauri::command]
pub async fn hide_rows(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.hide_rows(start_row, count);
    Ok(())
}

/// Unhide rows starting at `start_row` for `count` rows.
#[tauri::command]
pub async fn unhide_rows(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.unhide_rows(start_row, count);
    Ok(())
}

/// Hide columns starting at `start_col` for `count` columns.
#[tauri::command]
pub async fn hide_cols(
    state: State<'_, AppState>,
    sheet: String,
    start_col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.hide_cols(start_col, count);
    Ok(())
}

/// Unhide columns starting at `start_col` for `count` columns.
#[tauri::command]
pub async fn unhide_cols(
    state: State<'_, AppState>,
    sheet: String,
    start_col: u32,
    count: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.unhide_cols(start_col, count);
    Ok(())
}

/// Get the set of hidden column indices for a sheet.
#[tauri::command]
pub async fn get_hidden_cols(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Vec<u32>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    let mut cols: Vec<u32> = s.hidden_cols.iter().copied().collect();
    cols.sort();
    Ok(cols)
}

// ---------------------------------------------------------------------------
// Sort
// ---------------------------------------------------------------------------

/// A sort key for the sort_range command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortKeyInput {
    /// 0-based column index to sort by.
    pub col: u32,
    /// Sort direction: "asc" or "desc".
    pub direction: String,
}

/// Sort a range of rows in a sheet by the given keys.
///
/// If `range` is provided (A1:B10 notation), sort that range.
/// Otherwise, sort the entire used range of the sheet.
#[tauri::command]
pub async fn sort_range(
    state: State<'_, AppState>,
    sheet: String,
    range: Option<String>,
    sort_keys: Vec<SortKeyInput>,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;

    let (start_row, start_col, end_row, end_col) = if let Some(ref range_str) = range {
        let parts: Vec<&str> = range_str.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid range format '{}': expected 'A1:B2'", range_str));
        }
        let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
        let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
        (start.row, start.col, end.row, end.col)
    } else {
        let (max_row, max_col) = s.used_range();
        (0, 0, max_row, max_col)
    };

    let keys: Vec<SortKey> = sort_keys
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

    lattice_core::sort::sort_range(s, start_row, end_row, start_col, end_col, &keys)
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Named Ranges
// ---------------------------------------------------------------------------

/// Information about a named range returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedRangeInfo {
    pub name: String,
    pub sheet: Option<String>,
    pub range: String,
}

/// Add a named range to the workbook.
#[tauri::command]
pub async fn add_named_range(
    state: State<'_, AppState>,
    name: String,
    range: String,
    sheet: Option<String>,
) -> Result<(), String> {
    let parts: Vec<&str> = range.split(':').collect();
    let core_range = if parts.len() == 2 {
        let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
        let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
        lattice_core::Range { start, end }
    } else if parts.len() == 1 {
        let cell = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
        lattice_core::Range {
            start: cell.clone(),
            end: cell,
        }
    } else {
        return Err(format!("Invalid range format '{}'", range));
    };

    let mut wb = state.workbook.write().await;
    wb.named_ranges
        .add(name, sheet, core_range)
        .map_err(|e| e.to_string())
}

/// List all named ranges in the workbook.
#[tauri::command]
pub async fn list_named_ranges(
    state: State<'_, AppState>,
) -> Result<Vec<NamedRangeInfo>, String> {
    let wb = state.workbook.read().await;
    let ranges = wb.named_ranges.list();
    Ok(ranges
        .into_iter()
        .map(|nr| NamedRangeInfo {
            name: nr.name.clone(),
            sheet: nr.sheet.clone(),
            range: format_range(&nr.range),
        })
        .collect())
}

/// Remove a named range by name.
#[tauri::command]
pub async fn remove_named_range(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    wb.named_ranges.remove(&name).map_err(|e| e.to_string())
}

/// Resolve a named range to its sheet and A1-notation range string.
#[tauri::command]
pub async fn resolve_named_range(
    state: State<'_, AppState>,
    name: String,
) -> Result<NamedRangeInfo, String> {
    let wb = state.workbook.read().await;
    let nr = wb
        .named_ranges
        .get(&name)
        .ok_or_else(|| format!("Named range '{}' not found", name))?;
    Ok(NamedRangeInfo {
        name: nr.name.clone(),
        sheet: nr.sheet.clone(),
        range: format_range(&nr.range),
    })
}

// ---------------------------------------------------------------------------
// Row group commands
// ---------------------------------------------------------------------------

/// Serialized row group for listing.
#[derive(Debug, Clone, Serialize)]
pub struct RowGroupOutput {
    pub start: u32,
    pub end: u32,
    pub collapsed: bool,
}

/// Add a row group (collapsible section).
#[tauri::command]
pub async fn add_row_group(
    state: State<'_, AppState>,
    sheet: String,
    start: u32,
    end: u32,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.add_row_group(start, end).map_err(|e| e.to_string())
}

/// Remove a row group by index.
#[tauri::command]
pub async fn remove_row_group(
    state: State<'_, AppState>,
    sheet: String,
    index: usize,
) -> Result<(), String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.remove_row_group(index).map_err(|e| e.to_string())
}

/// Toggle a row group between collapsed and expanded.
#[tauri::command]
pub async fn toggle_row_group(
    state: State<'_, AppState>,
    sheet: String,
    index: usize,
) -> Result<bool, String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    s.toggle_row_group(index).map_err(|e| e.to_string())
}

/// Get all row groups for a sheet.
#[tauri::command]
pub async fn get_row_groups(
    state: State<'_, AppState>,
    sheet: String,
) -> Result<Vec<RowGroupOutput>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    Ok(s.row_groups()
        .iter()
        .map(|g| RowGroupOutput {
            start: g.start,
            end: g.end,
            collapsed: g.collapsed,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Remove duplicates
// ---------------------------------------------------------------------------

/// Remove duplicate rows based on specified columns.
///
/// Returns the number of duplicate rows removed.
#[tauri::command]
pub async fn remove_duplicates(
    state: State<'_, AppState>,
    sheet: String,
    start_row: u32,
    end_row: u32,
    columns: Vec<u32>,
) -> Result<u32, String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    Ok(s.remove_duplicates(start_row, end_row, &columns))
}

// ---------------------------------------------------------------------------
// Text to columns
// ---------------------------------------------------------------------------

/// Split text in a column into multiple columns by delimiter.
///
/// Returns the maximum number of columns produced.
#[tauri::command]
pub async fn text_to_columns(
    state: State<'_, AppState>,
    sheet: String,
    col: u32,
    delimiter: String,
    start_row: u32,
    end_row: u32,
) -> Result<u32, String> {
    let mut wb = state.workbook.write().await;
    let s = wb.get_sheet_mut(&sheet).map_err(|e| e.to_string())?;
    Ok(s.text_to_columns(col, &delimiter, start_row, end_row))
}

// ---------------------------------------------------------------------------
// Sheet headers (for pivot dialog column detection)
// ---------------------------------------------------------------------------

/// Return the values in a given row of a sheet as strings.
///
/// Used by the pivot dialog to populate column header dropdowns.
/// If a cell is empty or does not exist, its entry will be an empty string.
#[tauri::command]
pub async fn get_sheet_headers(
    state: State<'_, AppState>,
    sheet: String,
    row: u32,
) -> Result<Vec<String>, String> {
    let wb = state.workbook.read().await;
    let s = wb.get_sheet(&sheet).map_err(|e| e.to_string())?;
    let (_, max_col) = s.used_range();

    let mut headers = Vec::new();
    for col in 0..=max_col {
        let value = s
            .get_cell(row, col)
            .map(|c| match &c.value {
                lattice_core::CellValue::Text(t) => t.clone(),
                lattice_core::CellValue::Number(n) => n.to_string(),
                lattice_core::CellValue::Boolean(b) => b.to_string(),
                lattice_core::CellValue::Date(d) => d.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        headers.push(value);
    }

    Ok(headers)
}

// ---------------------------------------------------------------------------
// Pivot table
// ---------------------------------------------------------------------------

/// Value field specification from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotValueInput {
    /// 0-based column index within the source range.
    pub col: u32,
    /// Aggregation type: "Sum", "Count", "Average", "Min", "Max".
    pub aggregation: String,
}

/// Create a pivot table from source data and write results to a target sheet.
///
/// Parses the `source_range` (A1:C10 format), builds a `PivotConfig`, calls
/// `generate_pivot`, creates the target sheet if needed, and writes the
/// result rows.
#[tauri::command]
pub async fn create_pivot_table(
    state: State<'_, AppState>,
    source_sheet: String,
    source_range: String,
    row_fields: Vec<u32>,
    value_fields: Vec<PivotValueInput>,
    target_sheet: String,
) -> Result<(), String> {
    use lattice_core::{PivotAggregation, PivotConfig, PivotValue, generate_pivot};

    // Parse the source range (e.g. "A1:C10")
    let parts: Vec<&str> = source_range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid source range '{}': expected 'A1:B2' format",
            source_range
        ));
    }
    let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
    let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
    let range = lattice_core::Range { start, end };

    // Convert value field inputs to PivotValue structs.
    let pivot_values: Vec<PivotValue> = value_fields
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
        source_sheet: source_sheet.clone(),
        source_range: range,
        row_fields,
        col_fields: vec![],
        value_fields: pivot_values,
    };

    let mut wb = state.workbook.write().await;

    // Generate the pivot result.
    let result = generate_pivot(&wb, &config).map_err(|e| e.to_string())?;

    // Create the target sheet if it doesn't exist.
    if wb.get_sheet(&target_sheet).is_err() {
        wb.add_sheet(&target_sheet).map_err(|e| e.to_string())?;
    }

    let target = wb
        .get_sheet_mut(&target_sheet)
        .map_err(|e| e.to_string())?;

    // Write headers to row 0.
    for (col, header) in result.headers.iter().enumerate() {
        target.set_value(0, col as u32, lattice_core::CellValue::Text(header.clone()));
    }

    // Write data rows starting at row 1.
    for (row_idx, row) in result.rows.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            target.set_value((row_idx + 1) as u32, col_idx as u32, value.clone());
        }
    }

    Ok(())
}

/// Format a Range as "A1:B2" string.
fn format_range(range: &lattice_core::Range) -> String {
    let start = format_cell_ref(&range.start);
    let end = format_cell_ref(&range.end);
    if start == end {
        start
    } else {
        format!("{}:{}", start, end)
    }
}

/// Format a CellRef as "A1" string.
fn format_cell_ref(cell: &CellRef) -> String {
    let col_str = lattice_core::col_to_letter(cell.col);
    format!("{}{}", col_str, cell.row + 1)
}
