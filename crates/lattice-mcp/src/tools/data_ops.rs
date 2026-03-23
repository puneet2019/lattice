//! Data operation tool handlers: clear_range, find_replace, sort_range, deduplicate, transpose.

use std::collections::HashSet;

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{
    CellRef, CellValue, FillDirection, PivotAggregation, PivotConfig, PivotValue, Range, Workbook,
    col_to_letter, fill_range, generate_pivot,
};

use super::ToolDef;
use crate::schema::{bool_prop, object_schema, string_prop};

/// Return tool definitions for data operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "clear_range".to_string(),
            description: "Clear all cells in a range (removes values, formulas, and formatting)"
                .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range to clear in A1:B2 notation")),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "find_replace".to_string(),
            description: "Find and optionally replace text in cells".to_string(),
            input_schema: object_schema(
                &[
                    ("find", string_prop("Text or pattern to search for")),
                    (
                        "replace",
                        string_prop("Replacement text (omit for find-only)"),
                    ),
                    (
                        "sheet",
                        string_prop("Sheet to search (omit for all sheets)"),
                    ),
                    ("regex", bool_prop("Treat find as a regular expression")),
                    (
                        "case_sensitive",
                        bool_prop("Case-sensitive search (default true)"),
                    ),
                ],
                &["find"],
            ),
        },
        ToolDef {
            name: "sort_range".to_string(),
            description: "Sort rows in a range by one or more columns".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Range to sort in A1:B2 notation")),
                    (
                        "sort_by",
                        json!({
                            "type": "array",
                            "description": "Columns to sort by, in priority order",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "column": {"type": "string", "description": "Column letter (e.g. 'A')"},
                                    "ascending": {"type": "boolean", "description": "Sort ascending (default true)"}
                                },
                                "required": ["column"]
                            }
                        }),
                    ),
                ],
                &["sheet", "range", "sort_by"],
            ),
        },
        ToolDef {
            name: "deduplicate".to_string(),
            description: "Remove duplicate rows from a range based on specified columns"
                .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    (
                        "range",
                        string_prop("Range to deduplicate in A1:B2 notation"),
                    ),
                    (
                        "columns",
                        json!({
                            "type": "array",
                            "description": "Column letters to check for duplicates (e.g. ['A', 'B']). If omitted, all columns are checked.",
                            "items": {"type": "string"}
                        }),
                    ),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "transpose".to_string(),
            description:
                "Transpose data from a source range to a target location (rows become columns)"
                    .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    (
                        "source_range",
                        string_prop("Source range in A1:B2 notation"),
                    ),
                    (
                        "target_cell",
                        string_prop("Top-left cell for transposed output in A1 notation"),
                    ),
                ],
                &["sheet", "source_range", "target_cell"],
            ),
        },
        ToolDef {
            name: "auto_fill".to_string(),
            description: "Detect a pattern in a source range and fill a target range with the continuation of that pattern (numeric sequences, text with numbers, repeating cycles)".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("source_range", string_prop("Source range in A1:B2 notation containing the pattern")),
                    ("target_range", string_prop("Target range in A1:B2 notation to fill")),
                    ("direction", string_prop("Fill direction: 'down', 'right', 'up', or 'left'")),
                ],
                &["sheet", "source_range", "target_range", "direction"],
            ),
        },
        ToolDef {
            name: "generate_pivot".to_string(),
            description: "Generate a pivot table from sheet data. Groups rows by specified fields and applies aggregation functions (sum, count, average, min, max, count_distinct) to value fields.".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name containing the source data")),
                    ("source_range", string_prop("Source data range in A1:B2 notation")),
                    (
                        "row_fields",
                        json!({
                            "type": "array",
                            "description": "Column letters to group by (e.g. ['A', 'B'])",
                            "items": {"type": "string"}
                        }),
                    ),
                    (
                        "value_fields",
                        json!({
                            "type": "array",
                            "description": "Value fields with aggregation functions",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "col": {"type": "string", "description": "Column letter (e.g. 'C')"},
                                    "aggregation": {
                                        "type": "string",
                                        "description": "Aggregation function: 'sum', 'count', 'average', 'min', 'max', or 'count_distinct'"
                                    }
                                },
                                "required": ["col", "aggregation"]
                            }
                        }),
                    ),
                ],
                &["sheet", "source_range", "row_fields", "value_fields"],
            ),
        },
        ToolDef {
            name: "remove_duplicates".to_string(),
            description: "Remove duplicate rows from a range based on specified columns (uses the core engine's Sheet::remove_duplicates)".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("start_row", json!({"type": "integer", "description": "Start row (1-based, e.g. 1 for row 1)"})),
                    ("end_row", json!({"type": "integer", "description": "End row (1-based, inclusive)"})),
                    (
                        "columns",
                        json!({
                            "type": "array",
                            "description": "Column letters to check for duplicates (e.g. ['A', 'B'])",
                            "items": {"type": "string"}
                        }),
                    ),
                ],
                &["sheet", "start_row", "end_row", "columns"],
            ),
        },
        ToolDef {
            name: "text_to_columns".to_string(),
            description: "Split text in a column by a delimiter into adjacent columns (uses the core engine's Sheet::text_to_columns)".to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("col", string_prop("Column letter to split (e.g. 'A')")),
                    ("delimiter", string_prop("Delimiter to split by (e.g. ',', '\\t', '|')")),
                    ("start_row", json!({"type": "integer", "description": "Start row (1-based, e.g. 1 for row 1)"})),
                    ("end_row", json!({"type": "integer", "description": "End row (1-based, inclusive)"})),
                ],
                &["sheet", "col", "delimiter", "start_row", "end_row"],
            ),
        },
    ]
}

/// Arguments for clear_range.
#[derive(Debug, Deserialize)]
pub struct ClearRangeArgs {
    pub sheet: String,
    pub range: String,
}

/// Handle the `clear_range` tool call.
pub fn handle_clear_range(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: ClearRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    let mut cells_cleared = 0u32;
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            if sheet.get_cell(row, col).is_some() {
                sheet.clear_cell(row, col);
                cells_cleared += 1;
            }
        }
    }

    Ok(json!({
        "success": true,
        "range": args.range,
        "cells_cleared": cells_cleared,
    }))
}

/// Arguments for find_replace.
#[derive(Debug, Deserialize)]
pub struct FindReplaceArgs {
    pub find: String,
    pub replace: Option<String>,
    pub sheet: Option<String>,
    pub regex: Option<bool>,
    pub case_sensitive: Option<bool>,
}

/// Handle the `find_replace` tool call.
pub fn handle_find_replace(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: FindReplaceArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let use_regex = args.regex.unwrap_or(false);
    let case_sensitive = args.case_sensitive.unwrap_or(true);

    let regex_pattern = if use_regex {
        let pattern = if case_sensitive {
            args.find.clone()
        } else {
            format!("(?i){}", args.find)
        };
        Some(
            regex::Regex::new(&pattern)
                .map_err(|e| format!("Invalid regex pattern '{}': {}", args.find, e))?,
        )
    } else {
        None
    };

    let sheet_names: Vec<String> = match &args.sheet {
        Some(name) => {
            // Verify the sheet exists.
            workbook.get_sheet(name).map_err(|e| e.to_string())?;
            vec![name.clone()]
        }
        None => workbook.sheet_names(),
    };

    let mut matches = Vec::new();
    let mut replacements_made = 0u32;

    for sheet_name in &sheet_names {
        let sheet = workbook.get_sheet(sheet_name).map_err(|e| e.to_string())?;

        // Collect matching cells.
        let cells_snapshot: Vec<((u32, u32), CellValue)> = sheet
            .cells()
            .iter()
            .map(|(&pos, cell)| (pos, cell.value.clone()))
            .collect();

        for ((row, col), value) in &cells_snapshot {
            let text = cell_value_to_string(value);
            if text.is_empty() {
                continue;
            }

            let is_match = if let Some(ref re) = regex_pattern {
                re.is_match(&text)
            } else if case_sensitive {
                text.contains(&args.find)
            } else {
                text.to_lowercase().contains(&args.find.to_lowercase())
            };

            if is_match {
                let cell_label = format!("{}{}", col_to_letter(*col), row + 1);
                matches.push(json!({
                    "sheet": sheet_name,
                    "cell_ref": cell_label,
                    "value": text,
                }));
            }
        }
    }

    // Perform replacements if a replacement string was given.
    if let Some(ref replacement) = args.replace {
        for m in &matches {
            let sheet_name = m["sheet"].as_str().unwrap();
            let cell_ref_str = m["cell_ref"].as_str().unwrap();

            let cell_ref = CellRef::parse(cell_ref_str).map_err(|e| e.to_string())?;

            let sheet = workbook
                .get_sheet_mut(sheet_name)
                .map_err(|e| e.to_string())?;

            if let Some(cell) = sheet.get_cell(cell_ref.row, cell_ref.col) {
                let mut new_cell = cell.clone();
                let old_text = cell_value_to_string(&cell.value);
                let new_text = if let Some(ref re) = regex_pattern {
                    re.replace_all(&old_text, replacement.as_str()).to_string()
                } else if case_sensitive {
                    old_text.replace(&args.find, replacement)
                } else {
                    case_insensitive_replace(&old_text, &args.find, replacement)
                };

                new_cell.value = CellValue::Text(new_text);
                sheet.set_cell(cell_ref.row, cell_ref.col, new_cell);
                replacements_made += 1;
            }
        }
    }

    Ok(json!({
        "matches_found": matches.len(),
        "replacements_made": replacements_made,
        "matches": matches,
    }))
}

/// Arguments for sort_range.
#[derive(Debug, Deserialize)]
pub struct SortRangeArgs {
    pub sheet: String,
    pub range: String,
    pub sort_by: Vec<SortColumn>,
}

/// A single column sort specification.
#[derive(Debug, Deserialize)]
pub struct SortColumn {
    pub column: String,
    pub ascending: Option<bool>,
}

/// Handle the `sort_range` tool call.
pub fn handle_sort_range(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: SortRangeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;

    // Read all rows from the range.
    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    let num_cols = (end.col - start.col + 1) as usize;

    // Collect rows as Vec<Vec<CellValue>>.
    let mut rows: Vec<Vec<CellValue>> = Vec::new();
    for row in start.row..=end.row {
        let mut row_data = Vec::with_capacity(num_cols);
        for col in start.col..=end.col {
            let val = match sheet.get_cell(row, col) {
                Some(c) => c.value.clone(),
                None => CellValue::Empty,
            };
            row_data.push(val);
        }
        rows.push(row_data);
    }

    // Parse sort column references to column offsets within the range.
    let mut sort_keys: Vec<(usize, bool)> = Vec::new();
    for sc in &args.sort_by {
        let col_ref = CellRef::parse(&format!("{}1", sc.column))
            .map_err(|e| format!("Invalid column '{}': {}", sc.column, e))?;
        if col_ref.col < start.col || col_ref.col > end.col {
            return Err(format!(
                "Sort column '{}' is outside the range '{}'",
                sc.column, args.range
            ));
        }
        let offset = (col_ref.col - start.col) as usize;
        sort_keys.push((offset, sc.ascending.unwrap_or(true)));
    }

    // Sort rows.
    rows.sort_by(|a, b| {
        for &(col_idx, ascending) in &sort_keys {
            let av = a.get(col_idx).cloned().unwrap_or(CellValue::Empty);
            let bv = b.get(col_idx).cloned().unwrap_or(CellValue::Empty);
            let cmp = compare_cell_values(&av, &bv);
            let cmp = if ascending { cmp } else { cmp.reverse() };
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });

    // Write sorted rows back.
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    for (row_offset, row_data) in rows.iter().enumerate() {
        let row = start.row + row_offset as u32;
        for (col_offset, val) in row_data.iter().enumerate() {
            let col = start.col + col_offset as u32;
            sheet.set_value(row, col, val.clone());
        }
    }

    Ok(json!({
        "success": true,
        "range": args.range,
        "rows_sorted": rows.len(),
    }))
}

/// Arguments for deduplicate.
#[derive(Debug, Deserialize)]
pub struct DeduplicateArgs {
    pub sheet: String,
    pub range: String,
    pub columns: Option<Vec<String>>,
}

/// Handle the `deduplicate` tool call.
pub fn handle_deduplicate(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: DeduplicateArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;

    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    let num_cols = (end.col - start.col + 1) as usize;

    // Determine which column offsets to use for dedup comparison.
    let dedup_col_offsets: Vec<usize> = match &args.columns {
        Some(cols) => {
            let mut offsets = Vec::new();
            for col_letter in cols {
                let col_ref = CellRef::parse(&format!("{}1", col_letter))
                    .map_err(|e| format!("Invalid column '{}': {}", col_letter, e))?;
                if col_ref.col < start.col || col_ref.col > end.col {
                    return Err(format!(
                        "Column '{}' is outside the range '{}'",
                        col_letter, args.range
                    ));
                }
                offsets.push((col_ref.col - start.col) as usize);
            }
            offsets
        }
        None => (0..num_cols).collect(),
    };

    // Read all rows.
    let mut rows: Vec<Vec<CellValue>> = Vec::new();
    for row in start.row..=end.row {
        let mut row_data = Vec::with_capacity(num_cols);
        for col in start.col..=end.col {
            let val = match sheet.get_cell(row, col) {
                Some(c) => c.value.clone(),
                None => CellValue::Empty,
            };
            row_data.push(val);
        }
        rows.push(row_data);
    }

    let original_count = rows.len();

    // Deduplicate: keep first occurrence.
    let mut seen = HashSet::new();
    let mut unique_rows = Vec::new();
    for row in &rows {
        let key: Vec<String> = dedup_col_offsets
            .iter()
            .map(|&idx| row.get(idx).map(cell_value_to_string).unwrap_or_default())
            .collect();
        let key_str = key.join("\x00");
        if seen.insert(key_str) {
            unique_rows.push(row.clone());
        }
    }

    let duplicates_removed = original_count - unique_rows.len();

    // Write unique rows back, clearing remaining cells.
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    for (row_offset, row_data) in unique_rows.iter().enumerate() {
        let row = start.row + row_offset as u32;
        for (col_offset, val) in row_data.iter().enumerate() {
            let col = start.col + col_offset as u32;
            sheet.set_value(row, col, val.clone());
        }
    }

    // Clear remaining rows that were duplicates.
    for row_offset in unique_rows.len()..original_count {
        let row = start.row + row_offset as u32;
        for col in start.col..=end.col {
            sheet.clear_cell(row, col);
        }
    }

    Ok(json!({
        "success": true,
        "range": args.range,
        "original_rows": original_count,
        "unique_rows": unique_rows.len(),
        "duplicates_removed": duplicates_removed,
    }))
}

/// Arguments for transpose.
#[derive(Debug, Deserialize)]
pub struct TransposeArgs {
    pub sheet: String,
    pub source_range: String,
    pub target_cell: String,
}

/// Handle the `transpose` tool call.
pub fn handle_transpose(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: TransposeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.source_range)?;
    let target =
        CellRef::parse(&args.target_cell).map_err(|e| format!("Invalid target cell: {}", e))?;

    // Read source data.
    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    let num_rows = (end.row - start.row + 1) as usize;
    let num_cols = (end.col - start.col + 1) as usize;

    let mut data: Vec<Vec<CellValue>> = Vec::new();
    for row in start.row..=end.row {
        let mut row_data = Vec::new();
        for col in start.col..=end.col {
            let val = match sheet.get_cell(row, col) {
                Some(c) => c.value.clone(),
                None => CellValue::Empty,
            };
            row_data.push(val);
        }
        data.push(row_data);
    }

    // Write transposed data: original rows become columns and vice versa.
    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    for (src_col, col_idx) in (0..num_cols).enumerate() {
        for (src_row, row_idx) in (0..num_rows).enumerate() {
            let val = data[row_idx][col_idx].clone();
            let target_row = target.row + src_col as u32;
            let target_col = target.col + src_row as u32;
            sheet.set_value(target_row, target_col, val);
        }
    }

    Ok(json!({
        "success": true,
        "source_range": args.source_range,
        "target_cell": args.target_cell,
        "original_dimensions": format!("{}x{}", num_rows, num_cols),
        "transposed_dimensions": format!("{}x{}", num_cols, num_rows),
    }))
}

// ── auto_fill ─────────────────────────────────────────────────────────────────

/// Arguments for auto_fill.
#[derive(Debug, Deserialize)]
pub struct AutoFillArgs {
    pub sheet: String,
    pub source_range: String,
    pub target_range: String,
    pub direction: String,
}

/// Handle the `auto_fill` tool call.
///
/// Delegates to the core engine's `fill_range` function.
pub fn handle_auto_fill(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: AutoFillArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (src_start, src_end) = parse_range(&args.source_range)?;
    let (tgt_start, tgt_end) = parse_range(&args.target_range)?;

    let direction = match args.direction.to_lowercase().as_str() {
        "down" => FillDirection::Down,
        "right" => FillDirection::Right,
        "up" => FillDirection::Up,
        "left" => FillDirection::Left,
        _ => {
            return Err(format!(
                "Invalid direction '{}': expected 'down', 'right', 'up', or 'left'",
                args.direction
            ));
        }
    };

    let source = Range {
        start: lattice_core::CellRef {
            row: src_start.row,
            col: src_start.col,
        },
        end: lattice_core::CellRef {
            row: src_end.row,
            col: src_end.col,
        },
    };
    let target = Range {
        start: lattice_core::CellRef {
            row: tgt_start.row,
            col: tgt_start.col,
        },
        end: lattice_core::CellRef {
            row: tgt_end.row,
            col: tgt_end.col,
        },
    };

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    fill_range(sheet, &source, &target, direction);

    // Count filled cells.
    let target_rows = (tgt_end.row - tgt_start.row + 1) as u32;
    let target_cols = (tgt_end.col - tgt_start.col + 1) as u32;
    let cells_filled = target_rows * target_cols;

    Ok(json!({
        "success": true,
        "source_range": args.source_range,
        "target_range": args.target_range,
        "direction": args.direction,
        "cells_filled": cells_filled,
    }))
}

// ── generate_pivot ────────────────────────────────────────────────────────────

/// Arguments for generate_pivot.
#[derive(Debug, Deserialize)]
pub struct GeneratePivotArgs {
    pub sheet: String,
    pub source_range: String,
    pub row_fields: Vec<String>,
    pub value_fields: Vec<PivotValueArg>,
}

/// A value field spec from the MCP caller.
#[derive(Debug, Deserialize)]
pub struct PivotValueArg {
    pub col: String,
    pub aggregation: String,
}

/// Handle the `generate_pivot` tool call.
///
/// Delegates to the core engine's `generate_pivot` function.
pub fn handle_generate_pivot(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: GeneratePivotArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.source_range)?;

    // Convert column letters to 0-based indices relative to the source range start.
    let row_fields: Vec<u32> = args
        .row_fields
        .iter()
        .map(|letter| {
            let cr = CellRef::parse(&format!("{}1", letter))
                .map_err(|e| format!("Invalid row_field column '{}': {}", letter, e))?;
            if cr.col < start.col || cr.col > end.col {
                return Err(format!(
                    "Row field column '{}' is outside the source range",
                    letter
                ));
            }
            Ok(cr.col - start.col)
        })
        .collect::<Result<Vec<u32>, String>>()?;

    let value_fields: Vec<PivotValue> = args
        .value_fields
        .iter()
        .map(|vf| {
            let cr = CellRef::parse(&format!("{}1", vf.col))
                .map_err(|e| format!("Invalid value_field column '{}': {}", vf.col, e))?;
            if cr.col < start.col || cr.col > end.col {
                return Err(format!(
                    "Value field column '{}' is outside the source range",
                    vf.col
                ));
            }
            let aggregation = match vf.aggregation.to_lowercase().as_str() {
                "sum" => PivotAggregation::Sum,
                "count" => PivotAggregation::Count,
                "average" | "avg" => PivotAggregation::Average,
                "min" => PivotAggregation::Min,
                "max" => PivotAggregation::Max,
                "count_distinct" | "countdistinct" => PivotAggregation::CountDistinct,
                _ => {
                    return Err(format!(
                        "Invalid aggregation '{}': expected sum, count, average, min, max, or count_distinct",
                        vf.aggregation
                    ));
                }
            };
            Ok(PivotValue {
                source_col: cr.col - start.col,
                aggregation,
                label: None,
            })
        })
        .collect::<Result<Vec<PivotValue>, String>>()?;

    let config = PivotConfig {
        source_sheet: args.sheet.clone(),
        source_range: Range {
            start: lattice_core::CellRef {
                row: start.row,
                col: start.col,
            },
            end: lattice_core::CellRef {
                row: end.row,
                col: end.col,
            },
        },
        row_fields,
        col_fields: vec![],
        value_fields,
    };

    let result = generate_pivot(workbook, &config).map_err(|e| e.to_string())?;

    // Convert PivotResult to JSON.
    let rows_json: Vec<Vec<Value>> = result
        .rows
        .iter()
        .map(|row| row.iter().map(cell_value_to_json).collect())
        .collect();

    Ok(json!({
        "headers": result.headers,
        "rows": rows_json,
        "row_count": result.rows.len(),
    }))
}

/// Convert a CellValue to a JSON Value for output.
fn cell_value_to_json(cv: &CellValue) -> Value {
    match cv {
        CellValue::Empty => Value::Null,
        CellValue::Text(s) => json!(s),
        CellValue::Number(n) => json!(n),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => json!(b),
        CellValue::Error(e) => json!(e.to_string()),
        CellValue::Date(s) => json!(s),
        CellValue::Array(arr) => {
            let rows: Vec<Vec<Value>> = arr
                .iter()
                .map(|row| row.iter().map(cell_value_to_json).collect())
                .collect();
            json!(rows)
        }
    }
}

// ── remove_duplicates ─────────────────────────────────────────────────────────

/// Arguments for remove_duplicates.
#[derive(Debug, Deserialize)]
pub struct RemoveDuplicatesArgs {
    pub sheet: String,
    pub start_row: u32,
    pub end_row: u32,
    pub columns: Vec<String>,
}

/// Handle the `remove_duplicates` tool call.
///
/// Delegates to the core engine's `Sheet::remove_duplicates` method.
pub fn handle_remove_duplicates(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: RemoveDuplicatesArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    if args.start_row == 0 || args.end_row == 0 {
        return Err("start_row and end_row must be 1-based (minimum 1)".to_string());
    }

    // Convert 1-based user input to 0-based internal representation.
    let start_row = args.start_row - 1;
    let end_row = args.end_row - 1;

    // Convert column letters to 0-based indices.
    let col_indices: Vec<u32> = args
        .columns
        .iter()
        .map(|letter| {
            CellRef::parse(&format!("{}1", letter))
                .map(|cr| cr.col)
                .map_err(|e| format!("Invalid column '{}': {}", letter, e))
        })
        .collect::<Result<Vec<u32>, String>>()?;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    let removed = sheet.remove_duplicates(start_row, end_row, &col_indices);

    Ok(json!({
        "success": true,
        "rows_removed": removed,
        "rows_remaining": (end_row - start_row + 1) - removed,
    }))
}

// ── text_to_columns ──────────────────────────────────────────────────────────

/// Arguments for text_to_columns.
#[derive(Debug, Deserialize)]
pub struct TextToColumnsArgs {
    pub sheet: String,
    pub col: String,
    pub delimiter: String,
    pub start_row: u32,
    pub end_row: u32,
}

/// Handle the `text_to_columns` tool call.
///
/// Delegates to the core engine's `Sheet::text_to_columns` method.
pub fn handle_text_to_columns(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: TextToColumnsArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    if args.start_row == 0 || args.end_row == 0 {
        return Err("start_row and end_row must be 1-based (minimum 1)".to_string());
    }

    let col_ref = CellRef::parse(&format!("{}1", args.col))
        .map_err(|e| format!("Invalid column '{}': {}", args.col, e))?;

    // Convert 1-based user input to 0-based internal representation.
    let start_row = args.start_row - 1;
    let end_row = args.end_row - 1;

    let sheet = workbook
        .get_sheet_mut(&args.sheet)
        .map_err(|e| e.to_string())?;

    let max_columns = sheet.text_to_columns(col_ref.col, &args.delimiter, start_row, end_row);

    Ok(json!({
        "success": true,
        "max_columns_created": max_columns,
        "column": args.col,
        "delimiter": args.delimiter,
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Parse a range string like "A1:C3" into two CellRefs.
fn parse_range(range: &str) -> Result<(CellRef, CellRef), String> {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid range format '{}': expected 'A1:B2'",
            range
        ));
    }
    let start = CellRef::parse(parts[0]).map_err(|e| e.to_string())?;
    let end = CellRef::parse(parts[1]).map_err(|e| e.to_string())?;
    Ok((start, end))
}

/// Convert a CellValue to its string representation for text matching.
fn cell_value_to_string(cv: &CellValue) -> String {
    match cv {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) | CellValue::Checkbox(b) => b.to_string(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(s) => s.clone(),
        CellValue::Array(_) => "{array}".to_string(),
    }
}

/// Compare two CellValues for sorting (Empty < Number < Text < Boolean < Error).
fn compare_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    fn type_rank(v: &CellValue) -> u8 {
        match v {
            CellValue::Empty => 0,
            CellValue::Number(_) => 1,
            CellValue::Text(_) => 2,
            CellValue::Boolean(_) | CellValue::Checkbox(_) => 3,
            CellValue::Date(_) => 4,
            CellValue::Error(_) => 5,
            CellValue::Array(_) => 6,
        }
    }

    match (a, b) {
        (CellValue::Empty, CellValue::Empty) => Ordering::Equal,
        (CellValue::Number(na), CellValue::Number(nb)) => {
            na.partial_cmp(nb).unwrap_or(Ordering::Equal)
        }
        (CellValue::Text(sa), CellValue::Text(sb)) => sa.cmp(sb),
        (CellValue::Boolean(ba), CellValue::Boolean(bb))
        | (CellValue::Checkbox(ba), CellValue::Checkbox(bb))
        | (CellValue::Boolean(ba), CellValue::Checkbox(bb))
        | (CellValue::Checkbox(ba), CellValue::Boolean(bb)) => ba.cmp(bb),
        (CellValue::Date(da), CellValue::Date(db)) => da.cmp(db),
        (CellValue::Error(_), CellValue::Error(_)) => Ordering::Equal,
        (CellValue::Array(_), CellValue::Array(_)) => Ordering::Equal,
        _ => type_rank(a).cmp(&type_rank(b)),
    }
}

/// Case-insensitive string replacement.
fn case_insensitive_replace(haystack: &str, needle: &str, replacement: &str) -> String {
    let lower_haystack = haystack.to_lowercase();
    let lower_needle = needle.to_lowercase();
    let mut result = String::new();
    let mut last_end = 0;

    for (start, _) in lower_haystack.match_indices(&lower_needle) {
        result.push_str(&haystack[last_end..start]);
        result.push_str(replacement);
        last_end = start + needle.len();
    }
    result.push_str(&haystack[last_end..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_range() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(3.0)).unwrap();

        let result =
            handle_clear_range(&mut wb, json!({"sheet": "Sheet1", "range": "A1:B2"})).unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["cells_cleared"], 3);
        assert!(wb.get_cell("Sheet1", 0, 0).unwrap().is_none());
    }

    #[test]
    fn test_find_replace_find_only() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello world".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("goodbye".into()))
            .unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("hello again".into()))
            .unwrap();

        let result = handle_find_replace(&mut wb, json!({"find": "hello"})).unwrap();

        assert_eq!(result["matches_found"], 2);
        assert_eq!(result["replacements_made"], 0);
    }

    #[test]
    fn test_find_replace_with_replacement() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello world".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("hello again".into()))
            .unwrap();

        let result = handle_find_replace(
            &mut wb,
            json!({"find": "hello", "replace": "hi", "sheet": "Sheet1"}),
        )
        .unwrap();

        assert_eq!(result["matches_found"], 2);
        assert_eq!(result["replacements_made"], 2);

        let cell = wb.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        assert_eq!(cell.value, CellValue::Text("hi world".into()));
    }

    #[test]
    fn test_find_replace_case_insensitive() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Hello World".into()))
            .unwrap();

        let result =
            handle_find_replace(&mut wb, json!({"find": "hello", "case_sensitive": false}))
                .unwrap();

        assert_eq!(result["matches_found"], 1);
    }

    #[test]
    fn test_find_replace_regex() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("abc123".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("xyz".into()))
            .unwrap();

        let result = handle_find_replace(
            &mut wb,
            json!({"find": "\\d+", "regex": true, "replace": "NUM"}),
        )
        .unwrap();

        assert_eq!(result["matches_found"], 1);
        let cell = wb.get_cell("Sheet1", 0, 0).unwrap().unwrap();
        assert_eq!(cell.value, CellValue::Text("abcNUM".into()));
    }

    #[test]
    fn test_sort_range_ascending() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Number(2.0)).unwrap();

        let result = handle_sort_range(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "range": "A1:A3",
                "sort_by": [{"column": "A", "ascending": true}]
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["rows_sorted"], 3);

        assert_eq!(
            wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 1, 0).unwrap().unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 2, 0).unwrap().unwrap().value,
            CellValue::Number(3.0)
        );
    }

    #[test]
    fn test_sort_range_descending() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Number(2.0)).unwrap();

        handle_sort_range(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "range": "A1:A3",
                "sort_by": [{"column": "A", "ascending": false}]
            }),
        )
        .unwrap();

        assert_eq!(
            wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Number(3.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 1, 0).unwrap().unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 2, 0).unwrap().unwrap().value,
            CellValue::Number(1.0)
        );
    }

    #[test]
    fn test_sort_range_multi_column() {
        let mut wb = Workbook::new();
        // Row 0: name="B", score=2
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("B".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(2.0)).unwrap();
        // Row 1: name="A", score=1
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("A".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(1.0)).unwrap();
        // Row 2: name="A", score=3
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("A".into()))
            .unwrap();
        wb.set_cell("Sheet1", 2, 1, CellValue::Number(3.0)).unwrap();

        handle_sort_range(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "range": "A1:B3",
                "sort_by": [
                    {"column": "A", "ascending": true},
                    {"column": "B", "ascending": true}
                ]
            }),
        )
        .unwrap();

        // Should be: A,1 then A,3 then B,2
        assert_eq!(
            wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("A".into())
        );
        assert_eq!(
            wb.get_cell("Sheet1", 0, 1).unwrap().unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 1, 0).unwrap().unwrap().value,
            CellValue::Text("A".into())
        );
        assert_eq!(
            wb.get_cell("Sheet1", 1, 1).unwrap().unwrap().value,
            CellValue::Number(3.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 2, 0).unwrap().unwrap().value,
            CellValue::Text("B".into())
        );
    }

    #[test]
    fn test_deduplicate() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("apple".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("banana".into()))
            .unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("apple".into()))
            .unwrap();
        wb.set_cell("Sheet1", 3, 0, CellValue::Text("cherry".into()))
            .unwrap();

        let result = handle_deduplicate(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "range": "A1:A4"
            }),
        )
        .unwrap();

        assert_eq!(result["original_rows"], 4);
        assert_eq!(result["unique_rows"], 3);
        assert_eq!(result["duplicates_removed"], 1);

        // Check that row 3 (was duplicate apple) is now cherry.
        assert_eq!(
            wb.get_cell("Sheet1", 2, 0).unwrap().unwrap().value,
            CellValue::Text("cherry".into())
        );
        // Row 4 should be cleared.
        assert!(wb.get_cell("Sheet1", 3, 0).unwrap().is_none());
    }

    #[test]
    fn test_deduplicate_with_columns() {
        let mut wb = Workbook::new();
        // Only check column A for duplicates.
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("a".into()))
            .unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("a".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(2.0)).unwrap();

        let result = handle_deduplicate(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "range": "A1:B2",
                "columns": ["A"]
            }),
        )
        .unwrap();

        assert_eq!(result["duplicates_removed"], 1);
    }

    #[test]
    fn test_transpose() {
        let mut wb = Workbook::new();
        // 2x3 source data.
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 0, 2, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(4.0)).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(5.0)).unwrap();
        wb.set_cell("Sheet1", 1, 2, CellValue::Number(6.0)).unwrap();

        let result = handle_transpose(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "source_range": "A1:C2",
                "target_cell": "E1"
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["original_dimensions"], "2x3");
        assert_eq!(result["transposed_dimensions"], "3x2");

        // E1 should be 1, F1 should be 4
        assert_eq!(
            wb.get_cell("Sheet1", 0, 4).unwrap().unwrap().value,
            CellValue::Number(1.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 0, 5).unwrap().unwrap().value,
            CellValue::Number(4.0)
        );
        // E2 should be 2, F2 should be 5
        assert_eq!(
            wb.get_cell("Sheet1", 1, 4).unwrap().unwrap().value,
            CellValue::Number(2.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 1, 5).unwrap().unwrap().value,
            CellValue::Number(5.0)
        );
        // E3 should be 3, F3 should be 6
        assert_eq!(
            wb.get_cell("Sheet1", 2, 4).unwrap().unwrap().value,
            CellValue::Number(3.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 2, 5).unwrap().unwrap().value,
            CellValue::Number(6.0)
        );
    }

    #[test]
    fn test_auto_fill_numeric_sequence() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Number(3.0)).unwrap();

        let result = handle_auto_fill(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "source_range": "A1:A3",
                "target_range": "A4:A6",
                "direction": "down"
            }),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["cells_filled"], 3);

        // Should continue: 4, 5, 6
        assert_eq!(
            wb.get_cell("Sheet1", 3, 0).unwrap().unwrap().value,
            CellValue::Number(4.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 4, 0).unwrap().unwrap().value,
            CellValue::Number(5.0)
        );
        assert_eq!(
            wb.get_cell("Sheet1", 5, 0).unwrap().unwrap().value,
            CellValue::Number(6.0)
        );
    }

    #[test]
    fn test_auto_fill_invalid_direction() {
        let mut wb = Workbook::new();
        let result = handle_auto_fill(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "source_range": "A1:A1",
                "target_range": "A2:A2",
                "direction": "diagonal"
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_pivot_basic() {
        let mut wb = Workbook::new();
        // Category | Amount
        // A        | 10
        // B        | 20
        // A        | 30
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Category".into())).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Text("Amount".into())).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("A".into())).unwrap();
        wb.set_cell("Sheet1", 1, 1, CellValue::Number(10.0)).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("B".into())).unwrap();
        wb.set_cell("Sheet1", 2, 1, CellValue::Number(20.0)).unwrap();
        wb.set_cell("Sheet1", 3, 0, CellValue::Text("A".into())).unwrap();
        wb.set_cell("Sheet1", 3, 1, CellValue::Number(30.0)).unwrap();

        let result = handle_generate_pivot(
            &wb,
            json!({
                "sheet": "Sheet1",
                "source_range": "A1:B4",
                "row_fields": ["A"],
                "value_fields": [{"col": "B", "aggregation": "sum"}]
            }),
        )
        .unwrap();

        assert_eq!(result["row_count"], 2);
        assert!(result["headers"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn test_generate_pivot_invalid_aggregation() {
        let wb = Workbook::new();
        let result = handle_generate_pivot(
            &wb,
            json!({
                "sheet": "Sheet1",
                "source_range": "A1:B2",
                "row_fields": ["A"],
                "value_fields": [{"col": "B", "aggregation": "invalid"}]
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_duplicates_basic() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("Alice".into()))
            .unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Text("Bob".into()))
            .unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Text("Alice".into()))
            .unwrap();

        let result = handle_remove_duplicates(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "start_row": 1,
                "end_row": 3,
                "columns": ["A"]
            }),
        )
        .unwrap();

        assert_eq!(result["rows_removed"], 1);
        assert_eq!(result["rows_remaining"], 2);
    }

    #[test]
    fn test_remove_duplicates_invalid_row() {
        let mut wb = Workbook::new();
        let result = handle_remove_duplicates(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "start_row": 0,
                "end_row": 3,
                "columns": ["A"]
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_text_to_columns_basic() {
        let mut wb = Workbook::new();
        wb.set_cell(
            "Sheet1",
            0,
            0,
            CellValue::Text("a,b,c".into()),
        )
        .unwrap();
        wb.set_cell(
            "Sheet1",
            1,
            0,
            CellValue::Text("x,y".into()),
        )
        .unwrap();

        let result = handle_text_to_columns(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "col": "A",
                "delimiter": ",",
                "start_row": 1,
                "end_row": 2
            }),
        )
        .unwrap();

        assert_eq!(result["max_columns_created"], 3);
        assert_eq!(result["success"], true);

        // Row 0: A="a", B="b", C="c"
        assert_eq!(
            wb.get_cell("Sheet1", 0, 0).unwrap().unwrap().value,
            CellValue::Text("a".into())
        );
        assert_eq!(
            wb.get_cell("Sheet1", 0, 1).unwrap().unwrap().value,
            CellValue::Text("b".into())
        );
        assert_eq!(
            wb.get_cell("Sheet1", 0, 2).unwrap().unwrap().value,
            CellValue::Text("c".into())
        );
    }

    #[test]
    fn test_text_to_columns_invalid_row() {
        let mut wb = Workbook::new();
        let result = handle_text_to_columns(
            &mut wb,
            json!({
                "sheet": "Sheet1",
                "col": "A",
                "delimiter": ",",
                "start_row": 0,
                "end_row": 1
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_compare_cell_values() {
        use std::cmp::Ordering;
        assert_eq!(
            compare_cell_values(&CellValue::Empty, &CellValue::Number(1.0)),
            Ordering::Less
        );
        assert_eq!(
            compare_cell_values(&CellValue::Number(1.0), &CellValue::Number(2.0)),
            Ordering::Less
        );
        assert_eq!(
            compare_cell_values(&CellValue::Text("a".into()), &CellValue::Text("b".into())),
            Ordering::Less
        );
    }
}
