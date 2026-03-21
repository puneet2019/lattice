//! Analysis tool handlers: describe_data, correlate, trend_analysis.
//!
//! These tools implement statistical analysis directly using workbook primitives,
//! without depending on lattice-analysis (which may not be fully implemented yet).

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::{CellRef, CellValue, Workbook};

use super::ToolDef;
use crate::schema::{object_schema, string_prop};

/// Return tool definitions for analysis operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "describe_data".to_string(),
            description:
                "Compute descriptive statistics for a data range (mean, median, std_dev, min, max, count, nulls)"
                    .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range", string_prop("Data range in A1:B2 notation")),
                ],
                &["sheet", "range"],
            ),
        },
        ToolDef {
            name: "correlate".to_string(),
            description: "Compute the Pearson correlation coefficient between two data ranges"
                .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range_x", string_prop("First data range (single column)")),
                    ("range_y", string_prop("Second data range (single column)")),
                ],
                &["sheet", "range_x", "range_y"],
            ),
        },
        ToolDef {
            name: "trend_analysis".to_string(),
            description:
                "Perform linear regression on x,y data ranges and return slope, intercept, and r_squared"
                    .to_string(),
            input_schema: object_schema(
                &[
                    ("sheet", string_prop("Sheet name")),
                    ("range_x", string_prop("X data range (independent variable)")),
                    ("range_y", string_prop("Y data range (dependent variable)")),
                ],
                &["sheet", "range_x", "range_y"],
            ),
        },
    ]
}

/// Arguments for describe_data.
#[derive(Debug, Deserialize)]
pub struct DescribeDataArgs {
    pub sheet: String,
    pub range: String,
}

/// Handle the `describe_data` tool call.
pub fn handle_describe_data(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: DescribeDataArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let (start, end) = parse_range(&args.range)?;
    let sheet = workbook.get_sheet(&args.sheet).map_err(|e| e.to_string())?;

    // Collect all numeric values and count non-numeric / empty cells.
    let mut numbers: Vec<f64> = Vec::new();
    let mut null_count = 0u32;
    let mut text_count = 0u32;
    let mut bool_count = 0u32;
    let mut total_cells = 0u32;

    for row in start.row..=end.row {
        for col in start.col..=end.col {
            total_cells += 1;
            match sheet.get_cell(row, col) {
                Some(cell) => match &cell.value {
                    CellValue::Number(n) => numbers.push(*n),
                    CellValue::Empty => null_count += 1,
                    CellValue::Text(_) => text_count += 1,
                    CellValue::Boolean(_) => bool_count += 1,
                    CellValue::Date(_) => text_count += 1,
                    CellValue::Error(_) => null_count += 1,
                },
                None => null_count += 1,
            }
        }
    }

    if numbers.is_empty() {
        return Ok(json!({
            "range": args.range,
            "total_cells": total_cells,
            "numeric_count": 0,
            "null_count": null_count,
            "text_count": text_count,
            "bool_count": bool_count,
            "message": "No numeric data found in range",
        }));
    }

    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let count = numbers.len();
    let sum: f64 = numbers.iter().sum();
    let mean = sum / count as f64;
    let min = numbers[0];
    let max = numbers[count - 1];

    // Median.
    let median = if count.is_multiple_of(2) {
        (numbers[count / 2 - 1] + numbers[count / 2]) / 2.0
    } else {
        numbers[count / 2]
    };

    // Standard deviation (population).
    let variance: f64 = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / count as f64;
    let std_dev = variance.sqrt();

    // Quartiles (Q1, Q3).
    let q1 = percentile(&numbers, 25.0);
    let q3 = percentile(&numbers, 75.0);

    Ok(json!({
        "range": args.range,
        "total_cells": total_cells,
        "numeric_count": count,
        "null_count": null_count,
        "text_count": text_count,
        "bool_count": bool_count,
        "statistics": {
            "mean": mean,
            "median": median,
            "std_dev": std_dev,
            "min": min,
            "max": max,
            "sum": sum,
            "q1": q1,
            "q3": q3,
            "variance": variance,
        },
    }))
}

/// Arguments for correlate.
#[derive(Debug, Deserialize)]
pub struct CorrelateArgs {
    pub sheet: String,
    pub range_x: String,
    pub range_y: String,
}

/// Handle the `correlate` tool call.
pub fn handle_correlate(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: CorrelateArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let x_values = extract_column_numbers(workbook, &args.sheet, &args.range_x)?;
    let y_values = extract_column_numbers(workbook, &args.sheet, &args.range_y)?;

    if x_values.len() != y_values.len() {
        return Err(format!(
            "Ranges must have the same number of numeric values. X has {}, Y has {}",
            x_values.len(),
            y_values.len()
        ));
    }

    if x_values.len() < 2 {
        return Err("Need at least 2 numeric values to compute correlation".to_string());
    }

    let n = x_values.len() as f64;
    let x_mean: f64 = x_values.iter().sum::<f64>() / n;
    let y_mean: f64 = y_values.iter().sum::<f64>() / n;

    let mut cov_sum = 0.0;
    let mut x_var_sum = 0.0;
    let mut y_var_sum = 0.0;

    for i in 0..x_values.len() {
        let dx = x_values[i] - x_mean;
        let dy = y_values[i] - y_mean;
        cov_sum += dx * dy;
        x_var_sum += dx * dx;
        y_var_sum += dy * dy;
    }

    let denominator = (x_var_sum * y_var_sum).sqrt();
    if denominator == 0.0 {
        return Ok(json!({
            "range_x": args.range_x,
            "range_y": args.range_y,
            "n": x_values.len(),
            "correlation": null,
            "message": "Cannot compute correlation: zero variance in one or both ranges",
        }));
    }

    let r = cov_sum / denominator;

    Ok(json!({
        "range_x": args.range_x,
        "range_y": args.range_y,
        "n": x_values.len(),
        "correlation": r,
        "r_squared": r * r,
        "interpretation": interpret_correlation(r),
    }))
}

/// Arguments for trend_analysis.
#[derive(Debug, Deserialize)]
pub struct TrendAnalysisArgs {
    pub sheet: String,
    pub range_x: String,
    pub range_y: String,
}

/// Handle the `trend_analysis` tool call.
pub fn handle_trend_analysis(workbook: &Workbook, args: Value) -> Result<Value, String> {
    let args: TrendAnalysisArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let x_values = extract_column_numbers(workbook, &args.sheet, &args.range_x)?;
    let y_values = extract_column_numbers(workbook, &args.sheet, &args.range_y)?;

    if x_values.len() != y_values.len() {
        return Err(format!(
            "Ranges must have the same number of numeric values. X has {}, Y has {}",
            x_values.len(),
            y_values.len()
        ));
    }

    if x_values.len() < 2 {
        return Err("Need at least 2 data points for trend analysis".to_string());
    }

    let n = x_values.len() as f64;
    let x_mean: f64 = x_values.iter().sum::<f64>() / n;
    let y_mean: f64 = y_values.iter().sum::<f64>() / n;

    // Linear regression: y = slope * x + intercept.
    let mut xy_sum = 0.0;
    let mut x_sq_sum = 0.0;
    for i in 0..x_values.len() {
        let dx = x_values[i] - x_mean;
        xy_sum += dx * (y_values[i] - y_mean);
        x_sq_sum += dx * dx;
    }

    if x_sq_sum == 0.0 {
        return Err("Cannot compute trend: X values have zero variance".to_string());
    }

    let slope = xy_sum / x_sq_sum;
    let intercept = y_mean - slope * x_mean;

    // R-squared.
    let ss_res: f64 = x_values
        .iter()
        .zip(y_values.iter())
        .map(|(x, y)| {
            let predicted = slope * x + intercept;
            (y - predicted).powi(2)
        })
        .sum();
    let ss_tot: f64 = y_values.iter().map(|y| (y - y_mean).powi(2)).sum();
    let r_squared = if ss_tot == 0.0 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Ok(json!({
        "range_x": args.range_x,
        "range_y": args.range_y,
        "n": x_values.len(),
        "linear_regression": {
            "slope": slope,
            "intercept": intercept,
            "r_squared": r_squared,
            "equation": format!("y = {:.4}x + {:.4}", slope, intercept),
        },
        "fit_quality": if r_squared >= 0.9 {
            "excellent"
        } else if r_squared >= 0.7 {
            "good"
        } else if r_squared >= 0.5 {
            "moderate"
        } else {
            "poor"
        },
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Parse a range string like "A1:A10" into two CellRefs.
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

/// Extract numeric values from a range in column-major order.
fn extract_column_numbers(
    workbook: &Workbook,
    sheet_name: &str,
    range: &str,
) -> Result<Vec<f64>, String> {
    let (start, end) = parse_range(range)?;
    let sheet = workbook.get_sheet(sheet_name).map_err(|e| e.to_string())?;

    let mut values = Vec::new();
    for row in start.row..=end.row {
        for col in start.col..=end.col {
            if let Some(cell) = sheet.get_cell(row, col)
                && let CellValue::Number(n) = &cell.value
            {
                values.push(*n);
            }
        }
    }
    Ok(values)
}

/// Compute the p-th percentile of a sorted slice (linear interpolation).
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = k.ceil() as usize;

    if f == c {
        sorted[f]
    } else {
        let d = k - f as f64;
        sorted[f] * (1.0 - d) + sorted[c] * d
    }
}

/// Provide a human-readable interpretation of a correlation coefficient.
fn interpret_correlation(r: f64) -> &'static str {
    let abs_r = r.abs();
    if abs_r >= 0.9 {
        "very strong"
    } else if abs_r >= 0.7 {
        "strong"
    } else if abs_r >= 0.5 {
        "moderate"
    } else if abs_r >= 0.3 {
        "weak"
    } else {
        "very weak or none"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_describe_data_basic() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Number(3.0)).unwrap();
        wb.set_cell("Sheet1", 3, 0, CellValue::Number(4.0)).unwrap();
        wb.set_cell("Sheet1", 4, 0, CellValue::Number(5.0)).unwrap();

        let result =
            handle_describe_data(&wb, json!({"sheet": "Sheet1", "range": "A1:A5"})).unwrap();

        assert_eq!(result["numeric_count"], 5);
        assert_eq!(result["statistics"]["mean"], 3.0);
        assert_eq!(result["statistics"]["median"], 3.0);
        assert_eq!(result["statistics"]["min"], 1.0);
        assert_eq!(result["statistics"]["max"], 5.0);
        assert_eq!(result["statistics"]["sum"], 15.0);
    }

    #[test]
    fn test_describe_data_with_nulls() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(10.0))
            .unwrap();
        wb.set_cell("Sheet1", 2, 0, CellValue::Number(20.0))
            .unwrap();

        let result =
            handle_describe_data(&wb, json!({"sheet": "Sheet1", "range": "A1:A3"})).unwrap();

        assert_eq!(result["numeric_count"], 2);
        assert_eq!(result["null_count"], 1);
        assert_eq!(result["statistics"]["mean"], 15.0);
    }

    #[test]
    fn test_describe_data_no_numbers() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Text("hello".into()))
            .unwrap();

        let result =
            handle_describe_data(&wb, json!({"sheet": "Sheet1", "range": "A1:A1"})).unwrap();

        assert_eq!(result["numeric_count"], 0);
        assert_eq!(result["text_count"], 1);
    }

    #[test]
    fn test_correlate_perfect_positive() {
        let mut wb = Workbook::new();
        for i in 0..5 {
            wb.set_cell("Sheet1", i, 0, CellValue::Number(i as f64 + 1.0))
                .unwrap();
            wb.set_cell("Sheet1", i, 1, CellValue::Number((i as f64 + 1.0) * 2.0))
                .unwrap();
        }

        let result = handle_correlate(
            &wb,
            json!({"sheet": "Sheet1", "range_x": "A1:A5", "range_y": "B1:B5"}),
        )
        .unwrap();

        let r: f64 = result["correlation"].as_f64().unwrap();
        assert!(
            (r - 1.0).abs() < 1e-10,
            "Expected correlation ~1.0, got {}",
            r
        );
    }

    #[test]
    fn test_correlate_perfect_negative() {
        let mut wb = Workbook::new();
        for i in 0..5 {
            wb.set_cell("Sheet1", i, 0, CellValue::Number(i as f64 + 1.0))
                .unwrap();
            wb.set_cell("Sheet1", i, 1, CellValue::Number(-(i as f64 + 1.0)))
                .unwrap();
        }

        let result = handle_correlate(
            &wb,
            json!({"sheet": "Sheet1", "range_x": "A1:A5", "range_y": "B1:B5"}),
        )
        .unwrap();

        let r: f64 = result["correlation"].as_f64().unwrap();
        assert!(
            (r - (-1.0)).abs() < 1e-10,
            "Expected correlation ~-1.0, got {}",
            r
        );
    }

    #[test]
    fn test_trend_analysis() {
        let mut wb = Workbook::new();
        // y = 2x + 1
        for i in 0..5 {
            let x = i as f64 + 1.0;
            let y = 2.0 * x + 1.0;
            wb.set_cell("Sheet1", i, 0, CellValue::Number(x)).unwrap();
            wb.set_cell("Sheet1", i, 1, CellValue::Number(y)).unwrap();
        }

        let result = handle_trend_analysis(
            &wb,
            json!({"sheet": "Sheet1", "range_x": "A1:A5", "range_y": "B1:B5"}),
        )
        .unwrap();

        let slope: f64 = result["linear_regression"]["slope"].as_f64().unwrap();
        let intercept: f64 = result["linear_regression"]["intercept"].as_f64().unwrap();
        let r_sq: f64 = result["linear_regression"]["r_squared"].as_f64().unwrap();

        assert!(
            (slope - 2.0).abs() < 1e-10,
            "Expected slope 2.0, got {}",
            slope
        );
        assert!(
            (intercept - 1.0).abs() < 1e-10,
            "Expected intercept 1.0, got {}",
            intercept
        );
        assert!(
            (r_sq - 1.0).abs() < 1e-10,
            "Expected r_squared 1.0, got {}",
            r_sq
        );
        assert_eq!(result["fit_quality"], "excellent");
    }

    #[test]
    fn test_trend_analysis_mismatched_lengths() {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", 0, 0, CellValue::Number(1.0)).unwrap();
        wb.set_cell("Sheet1", 1, 0, CellValue::Number(2.0)).unwrap();
        wb.set_cell("Sheet1", 0, 1, CellValue::Number(3.0)).unwrap();

        let result = handle_trend_analysis(
            &wb,
            json!({"sheet": "Sheet1", "range_x": "A1:A2", "range_y": "B1:B1"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_percentile() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&sorted, 50.0), 3.0);
        assert_eq!(percentile(&sorted, 0.0), 1.0);
        assert_eq!(percentile(&sorted, 100.0), 5.0);
    }
}
