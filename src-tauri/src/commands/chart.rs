use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use lattice_charts::{Chart, ChartData, ChartType, DataSeries, render_chart};

use crate::state::AppState;

/// Chart information returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartInfo {
    pub id: String,
    pub chart_type: String,
    pub data_range: String,
    pub sheet: String,
    pub title: Option<String>,
    pub width: u32,
    pub height: u32,
}

/// In-app chart store. Charts live here until they are persisted to a file.
///
/// We use a simple `Mutex<HashMap>` since chart operations are infrequent
/// and never long-running. This avoids adding async locking overhead.
pub struct ChartStore {
    charts: Mutex<HashMap<String, Chart>>,
}

impl ChartStore {
    pub fn new() -> Self {
        Self {
            charts: Mutex::new(HashMap::new()),
        }
    }

    /// Insert a chart into the store, returning the chart ID on success.
    pub fn insert(&self, id: String, chart: Chart) -> Result<(), String> {
        self.charts
            .lock()
            .map_err(|e| format!("Chart store lock error: {}", e))?
            .insert(id, chart);
        Ok(())
    }
}

impl Default for ChartStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new chart and return its generated ID.
#[tauri::command]
pub async fn create_chart(
    state: State<'_, AppState>,
    sheet: String,
    chart_type: String,
    data_range: String,
    title: Option<String>,
) -> Result<String, String> {
    let ct = parse_chart_type(&chart_type)?;
    let stacked = is_stacked(&chart_type);
    let chart_id = uuid::Uuid::new_v4().to_string();

    let mut chart = Chart::new(&chart_id, ct, &data_range, &sheet);
    if let Some(t) = title {
        chart = chart.with_title(t);
    }

    state
        .chart_store
        .charts
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?
        .insert(chart_id.clone(), chart);

    // Store stacked flag separately so render_chart_svg can use it.
    if stacked {
        state
            .chart_stacked
            .lock()
            .map_err(|e| format!("Stacked map lock error: {}", e))?
            .insert(chart_id.clone(), true);
    }

    Ok(chart_id)
}

/// Render a chart to an SVG string.
///
/// Extracts data from the workbook for the chart's data range, then
/// delegates to `lattice_charts::render_chart`.
#[tauri::command]
pub async fn render_chart_svg(
    state: State<'_, AppState>,
    chart_id: String,
) -> Result<String, String> {
    let (chart, stacked) = {
        let store = state
            .chart_store
            .charts
            .lock()
            .map_err(|e| format!("Chart store lock error: {}", e))?;
        let c = store
            .get(&chart_id)
            .cloned()
            .ok_or_else(|| format!("Chart not found: {}", chart_id))?;
        // Retrieve the stacked flag from the chart_type_raw metadata.
        let stacked = state
            .chart_stacked
            .lock()
            .map(|m| m.get(&chart_id).copied().unwrap_or(false))
            .unwrap_or(false);
        (c, stacked)
    };

    // Extract data from the workbook for this chart's range.
    let data = extract_chart_data(&state, &chart).await?;
    let mut options = chart.to_options();
    options.stacked = stacked;

    Ok(render_chart(&chart.chart_type, &data, &options))
}

/// List all charts, optionally filtered by sheet.
#[tauri::command]
pub async fn list_charts(
    state: State<'_, AppState>,
    sheet: Option<String>,
) -> Result<Vec<ChartInfo>, String> {
    let store = state
        .chart_store
        .charts
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?;

    let charts: Vec<ChartInfo> = store
        .values()
        .filter(|c| sheet.as_ref().is_none_or(|s| c.sheet == *s))
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

    Ok(charts)
}

/// Get the current configuration of an existing chart.
#[tauri::command]
pub async fn get_chart_config(
    state: State<'_, AppState>,
    chart_id: String,
) -> Result<ChartInfo, String> {
    let store = state
        .chart_store
        .charts
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?;

    let chart = store
        .get(&chart_id)
        .ok_or_else(|| format!("Chart not found: {}", chart_id))?;

    // Reconstruct the chart_type string including stacked prefix.
    let stacked = state
        .chart_stacked
        .lock()
        .map(|m| m.get(&chart_id).copied().unwrap_or(false))
        .unwrap_or(false);

    let chart_type_str = if stacked {
        format!("stacked_{}", chart.chart_type)
    } else {
        chart.chart_type.to_string()
    };

    Ok(ChartInfo {
        id: chart.id.clone(),
        chart_type: chart_type_str,
        data_range: chart.data_range.clone(),
        sheet: chart.sheet.clone(),
        title: chart.title.clone(),
        width: chart.width,
        height: chart.height,
    })
}

/// Update an existing chart's type, data range, and/or title.
#[tauri::command]
pub async fn update_chart(
    state: State<'_, AppState>,
    chart_id: String,
    chart_type: Option<String>,
    data_range: Option<String>,
    title: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<ChartInfo, String> {
    let mut store = state
        .chart_store
        .charts
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?;

    let chart = store
        .get_mut(&chart_id)
        .ok_or_else(|| format!("Chart not found: {}", chart_id))?;

    // Update chart type if provided.
    let mut new_stacked = None;
    if let Some(ref ct_str) = chart_type {
        let ct = parse_chart_type(ct_str)?;
        chart.chart_type = ct;
        new_stacked = Some(is_stacked(ct_str));
    }

    if let Some(ref dr) = data_range {
        // Validate range format.
        let _ = parse_range(dr)?;
        chart.data_range = dr.clone();
    }

    // Title: Some("") clears, Some("text") sets, None leaves unchanged.
    if let Some(ref t) = title {
        chart.title = if t.is_empty() { None } else { Some(t.clone()) };
    }

    if let Some(w) = width {
        chart.width = w;
    }

    if let Some(h) = height {
        chart.height = h;
    }

    let info = ChartInfo {
        id: chart.id.clone(),
        chart_type: chart_type.unwrap_or_else(|| chart.chart_type.to_string()),
        data_range: chart.data_range.clone(),
        sheet: chart.sheet.clone(),
        title: chart.title.clone(),
        width: chart.width,
        height: chart.height,
    };

    // Update stacked flag if chart type was changed.
    if let Some(stacked) = new_stacked
        && let Ok(mut map) = state.chart_stacked.lock()
    {
        if stacked {
            map.insert(chart_id, true);
        } else {
            map.remove(&chart_id);
        }
    }

    Ok(info)
}

/// Delete a chart by its ID.
#[tauri::command]
pub async fn delete_chart(state: State<'_, AppState>, chart_id: String) -> Result<(), String> {
    let mut store = state
        .chart_store
        .charts
        .lock()
        .map_err(|e| format!("Chart store lock error: {}", e))?;

    if store.remove(&chart_id).is_some() {
        // Clean up stacked flag.
        if let Ok(mut stacked) = state.chart_stacked.lock() {
            stacked.remove(&chart_id);
        }
        Ok(())
    } else {
        Err(format!("Chart not found: {}", chart_id))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Parse an A1-style range string like "A1:C5" into (start_row, start_col, end_row, end_col).
///
/// Returns 0-based row and 0-based column indices.
fn parse_range(range: &str) -> Result<(u32, u32, u32, u32), String> {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid range format: {}", range));
    }
    let (sr, sc) = parse_cell_ref(parts[0])?;
    let (er, ec) = parse_cell_ref(parts[1])?;
    Ok((sr, sc, er, ec))
}

/// Parse a single cell reference like "B3" into (row, col), 0-based.
fn parse_cell_ref(s: &str) -> Result<(u32, u32), String> {
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
    // Convert to 0-based.
    let col = col.saturating_sub(1);

    let row: u32 = row_str
        .parse::<u32>()
        .map_err(|_| format!("Invalid row number in cell ref: {}", s))?;
    // Convert to 0-based.
    let row = row.saturating_sub(1);

    Ok((row, col))
}

/// Extract chart data from the workbook for the given chart definition.
///
/// Convention: first column = labels, remaining columns = data series.
/// The first row is treated as series names (headers).
async fn extract_chart_data(state: &AppState, chart: &Chart) -> Result<ChartData, String> {
    let (sr, sc, er, ec) = parse_range(&chart.data_range)?;
    let workbook = state.workbook.read().await;
    let sheet = workbook
        .get_sheet(&chart.sheet)
        .map_err(|e| e.to_string())?;

    // Read raw values from cells.
    let mut rows: Vec<Vec<String>> = Vec::new();
    for r in sr..=er {
        let mut row_vals = Vec::new();
        for c in sc..=ec {
            let val = match sheet.get_cell(r, c) {
                Some(cell) => match &cell.value {
                    lattice_core::CellValue::Text(t) => t.clone(),
                    lattice_core::CellValue::Number(n) => n.to_string(),
                    lattice_core::CellValue::Boolean(b) | lattice_core::CellValue::Checkbox(b) => {
                        b.to_string()
                    }
                    lattice_core::CellValue::Date(d) => d.clone(),
                    lattice_core::CellValue::Empty => String::new(),
                    lattice_core::CellValue::Error(e) => e.to_string(),
                    lattice_core::CellValue::Array(_) => "{array}".to_string(),
                    lattice_core::CellValue::Lambda { .. } => "{lambda}".to_string(),
                },
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
        // Single column: no labels, one unnamed series.
        let values: Vec<f64> = rows
            .iter()
            .skip(1) // skip header row
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

    // First row = headers (first cell is label header, rest are series names).
    let header = &rows[0];

    // Data rows (skip header).
    let data_rows: Vec<&Vec<String>> = rows.iter().skip(1).collect();

    // Labels from first column of data rows.
    let labels: Vec<String> = data_rows.iter().map(|r| r[0].clone()).collect();

    // Each remaining column is a data series.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chart_type_valid() {
        assert_eq!(parse_chart_type("bar").unwrap(), ChartType::Bar);
        assert_eq!(parse_chart_type("line").unwrap(), ChartType::Line);
        assert_eq!(parse_chart_type("pie").unwrap(), ChartType::Pie);
        assert_eq!(parse_chart_type("scatter").unwrap(), ChartType::Scatter);
        assert_eq!(parse_chart_type("area").unwrap(), ChartType::Area);
    }

    #[test]
    fn test_parse_chart_type_invalid() {
        assert!(parse_chart_type("invalid").is_err());
    }

    #[test]
    fn test_parse_cell_ref() {
        assert_eq!(parse_cell_ref("A1").unwrap(), (0, 0));
        assert_eq!(parse_cell_ref("B3").unwrap(), (2, 1));
        assert_eq!(parse_cell_ref("C10").unwrap(), (9, 2));
        assert_eq!(parse_cell_ref("AA1").unwrap(), (0, 26));
    }

    #[test]
    fn test_parse_range() {
        let (sr, sc, er, ec) = parse_range("A1:C5").unwrap();
        assert_eq!((sr, sc, er, ec), (0, 0, 4, 2));
    }

    #[test]
    fn test_parse_range_invalid() {
        assert!(parse_range("A1").is_err());
        assert!(parse_range("A1:B2:C3").is_err());
    }
}
