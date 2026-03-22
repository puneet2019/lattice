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
            let aggregated = compute_aggregation(&pv.aggregation, row_indices, abs_col, sheet);
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
                        .is_some_and(|c| c.value != CellValue::Empty)
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
        CellValue::Boolean(b) | CellValue::Checkbox(b) => b.to_string(),
        CellValue::Error(e) => e.to_string(),
        CellValue::Date(d) => d.clone(),
        CellValue::Array(_) => "{array}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::selection::CellRef;
    use crate::workbook::Workbook;

    /// Helper: create a workbook with a "Data" sheet populated with sales data.
    ///
    /// Layout (row 0 is header):
    /// | Region | Product | Amount |
    /// | East   | Widget  | 100    |
    /// | West   | Gadget  | 200    |
    /// | East   | Widget  | 150    |
    /// | West   | Widget  | 300    |
    /// | East   | Gadget  | 50     |
    fn sales_workbook() -> Workbook {
        let mut wb = Workbook::new();
        wb.add_sheet("Data").unwrap();

        let s = wb.get_sheet_mut("Data").unwrap();
        // Header row
        s.set_value(0, 0, CellValue::Text("Region".into()));
        s.set_value(0, 1, CellValue::Text("Product".into()));
        s.set_value(0, 2, CellValue::Text("Amount".into()));
        // Data rows
        s.set_value(1, 0, CellValue::Text("East".into()));
        s.set_value(1, 1, CellValue::Text("Widget".into()));
        s.set_value(1, 2, CellValue::Number(100.0));

        s.set_value(2, 0, CellValue::Text("West".into()));
        s.set_value(2, 1, CellValue::Text("Gadget".into()));
        s.set_value(2, 2, CellValue::Number(200.0));

        s.set_value(3, 0, CellValue::Text("East".into()));
        s.set_value(3, 1, CellValue::Text("Widget".into()));
        s.set_value(3, 2, CellValue::Number(150.0));

        s.set_value(4, 0, CellValue::Text("West".into()));
        s.set_value(4, 1, CellValue::Text("Widget".into()));
        s.set_value(4, 2, CellValue::Number(300.0));

        s.set_value(5, 0, CellValue::Text("East".into()));
        s.set_value(5, 1, CellValue::Text("Gadget".into()));
        s.set_value(5, 2, CellValue::Number(50.0));

        wb
    }

    fn full_range() -> Range {
        Range {
            start: CellRef { row: 0, col: 0 },
            end: CellRef { row: 5, col: 2 },
        }
    }

    #[test]
    fn test_pivot_group_by_region_sum_amount() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0], // Region
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 2,
                aggregation: PivotAggregation::Sum,
                label: None,
            }],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(result.headers, vec!["Region", "Sum of Amount"]);
        assert_eq!(result.rows.len(), 2); // East, West

        // BTreeMap sorts keys: East < West
        assert_eq!(result.rows[0][0], CellValue::Text("East".into()));
        assert_eq!(result.rows[0][1], CellValue::Number(300.0)); // 100+150+50

        assert_eq!(result.rows[1][0], CellValue::Text("West".into()));
        assert_eq!(result.rows[1][1], CellValue::Number(500.0)); // 200+300
    }

    #[test]
    fn test_pivot_multi_field_group_average() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0, 1], // Region + Product
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 2,
                aggregation: PivotAggregation::Average,
                label: None,
            }],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(
            result.headers,
            vec!["Region", "Product", "Average of Amount"]
        );
        // Groups: (East, Gadget), (East, Widget), (West, Gadget), (West, Widget)
        assert_eq!(result.rows.len(), 4);

        // East Gadget: avg(50) = 50
        assert_eq!(result.rows[0][0], CellValue::Text("East".into()));
        assert_eq!(result.rows[0][1], CellValue::Text("Gadget".into()));
        assert_eq!(result.rows[0][2], CellValue::Number(50.0));

        // East Widget: avg(100, 150) = 125
        assert_eq!(result.rows[1][0], CellValue::Text("East".into()));
        assert_eq!(result.rows[1][1], CellValue::Text("Widget".into()));
        assert_eq!(result.rows[1][2], CellValue::Number(125.0));

        // West Gadget: avg(200) = 200
        assert_eq!(result.rows[2][2], CellValue::Number(200.0));

        // West Widget: avg(300) = 300
        assert_eq!(result.rows[3][2], CellValue::Number(300.0));
    }

    #[test]
    fn test_pivot_count_aggregation() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0], // Region
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 2,
                aggregation: PivotAggregation::Count,
                label: Some("Row Count".into()),
            }],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(result.headers, vec!["Region", "Row Count"]);
        // East: 3 rows, West: 2 rows
        assert_eq!(result.rows[0][1], CellValue::Number(3.0));
        assert_eq!(result.rows[1][1], CellValue::Number(2.0));
    }

    #[test]
    fn test_pivot_count_distinct() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0], // Region
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 1, // Product
                aggregation: PivotAggregation::CountDistinct,
                label: None,
            }],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        // East has Widget + Gadget = 2 distinct products
        assert_eq!(result.rows[0][1], CellValue::Number(2.0));
        // West has Gadget + Widget = 2 distinct products
        assert_eq!(result.rows[1][1], CellValue::Number(2.0));
    }

    #[test]
    fn test_pivot_min_max() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0], // Region
            col_fields: vec![],
            value_fields: vec![
                PivotValue {
                    source_col: 2,
                    aggregation: PivotAggregation::Min,
                    label: None,
                },
                PivotValue {
                    source_col: 2,
                    aggregation: PivotAggregation::Max,
                    label: None,
                },
            ],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(result.headers.len(), 3); // Region, Min, Max
        // East: min=50, max=150
        assert_eq!(result.rows[0][1], CellValue::Number(50.0));
        assert_eq!(result.rows[0][2], CellValue::Number(150.0));
        // West: min=200, max=300
        assert_eq!(result.rows[1][1], CellValue::Number(200.0));
        assert_eq!(result.rows[1][2], CellValue::Number(300.0));
    }

    #[test]
    fn test_pivot_empty_data() {
        let mut wb = Workbook::new();
        wb.add_sheet("Empty").unwrap();
        let s = wb.get_sheet_mut("Empty").unwrap();
        s.set_value(0, 0, CellValue::Text("Name".into()));
        s.set_value(0, 1, CellValue::Text("Value".into()));
        // No data rows

        let config = PivotConfig {
            source_sheet: "Empty".into(),
            source_range: Range {
                start: CellRef { row: 0, col: 0 },
                end: CellRef { row: 0, col: 1 },
            },
            row_fields: vec![0],
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 1,
                aggregation: PivotAggregation::Sum,
                label: None,
            }],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(result.headers, vec!["Name", "Sum of Value"]);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn test_pivot_single_row() {
        let mut wb = Workbook::new();
        wb.add_sheet("One").unwrap();
        let s = wb.get_sheet_mut("One").unwrap();
        s.set_value(0, 0, CellValue::Text("Category".into()));
        s.set_value(0, 1, CellValue::Text("Value".into()));
        s.set_value(1, 0, CellValue::Text("A".into()));
        s.set_value(1, 1, CellValue::Number(42.0));

        let config = PivotConfig {
            source_sheet: "One".into(),
            source_range: Range {
                start: CellRef { row: 0, col: 0 },
                end: CellRef { row: 1, col: 1 },
            },
            row_fields: vec![0],
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 1,
                aggregation: PivotAggregation::Sum,
                label: None,
            }],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0], CellValue::Text("A".into()));
        assert_eq!(result.rows[0][1], CellValue::Number(42.0));
    }

    #[test]
    fn test_pivot_sheet_not_found() {
        let wb = Workbook::new();
        let config = PivotConfig {
            source_sheet: "NonExistent".into(),
            source_range: full_range(),
            row_fields: vec![0],
            col_fields: vec![],
            value_fields: vec![],
        };

        let result = generate_pivot(&wb, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_pivot_field_out_of_range() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![10], // out of range
            col_fields: vec![],
            value_fields: vec![],
        };

        let result = generate_pivot(&wb, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_pivot_value_field_out_of_range() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0],
            col_fields: vec![],
            value_fields: vec![PivotValue {
                source_col: 99,
                aggregation: PivotAggregation::Sum,
                label: None,
            }],
        };

        let result = generate_pivot(&wb, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_pivot_multiple_value_fields() {
        let wb = sales_workbook();
        let config = PivotConfig {
            source_sheet: "Data".into(),
            source_range: full_range(),
            row_fields: vec![0],
            col_fields: vec![],
            value_fields: vec![
                PivotValue {
                    source_col: 2,
                    aggregation: PivotAggregation::Sum,
                    label: Some("Total".into()),
                },
                PivotValue {
                    source_col: 2,
                    aggregation: PivotAggregation::Count,
                    label: Some("Rows".into()),
                },
                PivotValue {
                    source_col: 2,
                    aggregation: PivotAggregation::Average,
                    label: Some("Avg".into()),
                },
            ],
        };

        let result = generate_pivot(&wb, &config).unwrap();

        assert_eq!(result.headers, vec!["Region", "Total", "Rows", "Avg"]);
        // East: sum=300, count=3, avg=100
        assert_eq!(result.rows[0][1], CellValue::Number(300.0));
        assert_eq!(result.rows[0][2], CellValue::Number(3.0));
        assert_eq!(result.rows[0][3], CellValue::Number(100.0));
    }
}
