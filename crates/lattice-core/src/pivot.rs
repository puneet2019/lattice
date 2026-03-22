//! Pivot table engine for generating pivot table results from sheet data.
//!
//! Given a `PivotConfig` specifying source data, row grouping fields, and
//! value aggregations, [`generate_pivot`] produces a [`PivotResult`] with
//! headers and grouped/aggregated rows.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::cell::CellValue;
use crate::error::{LatticeError, Result};
use crate::selection::Range;
use crate::workbook::Workbook;

/// Aggregation function to apply to pivot value fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PivotAggregation {
    /// Sum of numeric values.
    Sum,
    /// Count of non-empty values.
    Count,
    /// Arithmetic mean of numeric values.
    Average,
    /// Minimum numeric value.
    Min,
    /// Maximum numeric value.
    Max,
    /// Count of distinct values.
    CountDistinct,
}

/// A value field in a pivot table — which source column to aggregate and how.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotValue {
    /// 0-based column index within the source range.
    pub source_col: u32,
    /// Aggregation function to apply.
    pub aggregation: PivotAggregation,
    /// Optional label for the result column. If `None`, a default label is
    /// generated from the aggregation type and column index.
    pub label: Option<String>,
}

/// Configuration for generating a pivot table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotConfig {
    /// Name of the sheet containing the source data.
    pub source_sheet: String,
    /// The rectangular range of source data (inclusive).
    pub source_range: Range,
    /// Column indices (within the source range) to use as row grouping fields.
    pub row_fields: Vec<u32>,
    /// Column indices to use as column grouping (reserved for future use).
    pub col_fields: Vec<u32>,
    /// Value fields with aggregation functions.
    pub value_fields: Vec<PivotValue>,
}

/// The result of generating a pivot table.
#[derive(Debug, Clone, PartialEq)]
pub struct PivotResult {
    /// Column headers for the result table.
    pub headers: Vec<String>,
    /// Data rows, each containing one `CellValue` per header column.
    pub rows: Vec<Vec<CellValue>>,
}

/// Generate a pivot table from workbook data according to the given config.
///
/// Groups source data by the `row_fields` columns, then computes the
/// requested aggregations on `value_fields` for each group. Returns a
/// structured [`PivotResult`] whose rows are sorted by group key.
///
/// # Errors
///
/// Returns `LatticeError::SheetNotFound` if the source sheet does not exist.
/// Returns `LatticeError::InvalidRange` if any field index is out of range.
pub fn generate_pivot(workbook: &Workbook, config: &PivotConfig) -> Result<PivotResult> {
    let sheet = workbook.get_sheet(&config.source_sheet)?;

    let range = &config.source_range;
    let range_start_col = range.start.col;
    let range_end_col = range.end.col;
    let range_start_row = range.start.row;
    let range_end_row = range.end.row;
    let num_cols = range_end_col - range_start_col + 1;

    // Validate field indices are within the source range column count.
    for &field in &config.row_fields {
        if field >= num_cols {
            return Err(LatticeError::InvalidRange(format!(
                "row_field index {field} is out of range (source has {num_cols} columns)"
            )));
        }
    }
    for pv in &config.value_fields {
        if pv.source_col >= num_cols {
            return Err(LatticeError::InvalidRange(format!(
                "value_field source_col {} is out of range (source has {num_cols} columns)",
                pv.source_col
            )));
        }
    }

    // Build headers: row field labels + value field labels.
    let mut headers: Vec<String> = Vec::new();
    for &field in &config.row_fields {
        // Use the value in the header row (first row of range) as label, or
        // fall back to "Field_N".
        let abs_col = range_start_col + field;
        let label = sheet
            .get_cell(range_start_row, abs_col)
            .and_then(|c| match &c.value {
                CellValue::Text(s) => Some(s.clone()),
                CellValue::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| format!("Field_{field}"));
        headers.push(label);
    }
    for pv in &config.value_fields {
        let abs_col = range_start_col + pv.source_col;
        let col_name = sheet
            .get_cell(range_start_row, abs_col)
            .and_then(|c| match &c.value {
                CellValue::Text(s) => Some(s.clone()),
                CellValue::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| format!("Col_{}", pv.source_col));
        let agg_name = match pv.aggregation {
            PivotAggregation::Sum => "Sum",
            PivotAggregation::Count => "Count",
            PivotAggregation::Average => "Average",
            PivotAggregation::Min => "Min",
            PivotAggregation::Max => "Max",
            PivotAggregation::CountDistinct => "CountDistinct",
        };
        let label = pv
            .label
            .clone()
            .unwrap_or_else(|| format!("{agg_name} of {col_name}"));
        headers.push(label);
    }

    // Data rows start at range_start_row + 1 (skip header row).
    let data_start = range_start_row + 1;
    if data_start > range_end_row {
        // No data rows — return empty result with headers.
        return Ok(PivotResult {
            headers,
            rows: Vec::new(),
        });
    }

    // Group rows by the row_fields key.
    // Use BTreeMap for deterministic (sorted) output order.
    let mut groups: BTreeMap<Vec<String>, Vec<u32>> = BTreeMap::new();

    for row in data_start..=range_end_row {
        let key: Vec<String> = config
            .row_fields
            .iter()
            .map(|&field| {
                let abs_col = range_start_col + field;
                sheet
                    .get_cell(row, abs_col)
                    .map(|c| cell_value_sort_key(&c.value))
                    .unwrap_or_default()
            })
            .collect();
        groups.entry(key).or_default().push(row);
    }

    // For each group, compute aggregations.
    let mut result_rows: Vec<Vec<CellValue>> = Vec::new();

    for (key, row_indices) in &groups {
        let mut result_row: Vec<CellValue> = Vec::new();

        // Add row field values (use the value from the first row in the group).
        for (i, &field) in config.row_fields.iter().enumerate() {
            let abs_col = range_start_col + field;
            let first_row = row_indices[0];
            let value = sheet
                .get_cell(first_row, abs_col)
                .map(|c| c.value.clone())
                .unwrap_or(CellValue::Empty);
            // If the key part is empty, still push the actual value.
            let _ = &key[i]; // used for grouping, actual value from sheet
            result_row.push(value);
        }

        // Compute each value field aggregation.
        for pv in &config.value_fields {
            let abs_col = range_start_col + pv.source_col;
            let aggregated = compute_aggregation(
                &pv.aggregation,
                row_indices,
                abs_col,
                sheet,
            );
            result_row.push(aggregated);
        }

        result_rows.push(result_row);
    }

    Ok(PivotResult {
        headers,
        rows: result_rows,
    })
}

/// Compute an aggregation over the specified rows and column.
fn compute_aggregation(
    agg: &PivotAggregation,
    rows: &[u32],
    abs_col: u32,
    sheet: &crate::sheet::Sheet,
) -> CellValue {
    match agg {
        PivotAggregation::Sum => {
            let mut sum = 0.0;
            for &row in rows {
                if let Some(n) = extract_number(sheet, row, abs_col) {
                    sum += n;
                }
            }
            CellValue::Number(sum)
        }
        PivotAggregation::Count => {
            let count = rows
                .iter()
                .filter(|&&row| {
                    sheet
                        .get_cell(row, abs_col)
                        .map_or(false, |c| c.value != CellValue::Empty)
                })
                .count();
            CellValue::Number(count as f64)
        }
        PivotAggregation::Average => {
            let mut sum = 0.0;
            let mut count = 0u32;
            for &row in rows {
                if let Some(n) = extract_number(sheet, row, abs_col) {
                    sum += n;
                    count += 1;
                }
            }
            if count == 0 {
                CellValue::Number(0.0)
            } else {
                CellValue::Number(sum / count as f64)
            }
        }
        PivotAggregation::Min => {
            let mut min: Option<f64> = None;
            for &row in rows {
                if let Some(n) = extract_number(sheet, row, abs_col) {
                    min = Some(min.map_or(n, |m: f64| m.min(n)));
                }
            }
            CellValue::Number(min.unwrap_or(0.0))
        }
        PivotAggregation::Max => {
            let mut max: Option<f64> = None;
            for &row in rows {
                if let Some(n) = extract_number(sheet, row, abs_col) {
                    max = Some(max.map_or(n, |m: f64| m.max(n)));
                }
            }
            CellValue::Number(max.unwrap_or(0.0))
        }
        PivotAggregation::CountDistinct => {
            let mut seen = HashSet::new();
            for &row in rows {
                let key = sheet
                    .get_cell(row, abs_col)
                    .map(|c| cell_value_sort_key(&c.value))
                    .unwrap_or_default();
                if !key.is_empty() {
                    seen.insert(key);
                }
            }
            CellValue::Number(seen.len() as f64)
        }
    }
}

/// Extract a numeric value from a cell, returning `None` for non-numeric cells.
fn extract_number(sheet: &crate::sheet::Sheet, row: u32, col: u32) -> Option<f64> {
    sheet.get_cell(row, col).and_then(|c| match &c.value {
        CellValue::Number(n) => Some(*n),
        CellValue::Boolean(true) => Some(1.0),
        CellValue::Boolean(false) => Some(0.0),
        _ => None,
    })
}

/// Create a sortable/comparable string key from a CellValue.
fn cell_value_sort_key(value: &CellValue) -> String {
    match value {
        CellValue::Empty => String::new(),
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => n.to_string(),
        CellValue::Boolean(b) => b.to_string(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(d) => d.clone(),
    }
}

