//! Core chart types and definitions.
//!
//! This module provides the chart data model including chart type definitions,
//! data containers (`ChartData`, `DataSeries`), rendering options (`ChartOptions`),
//! and the persistent `Chart` definition used in workbooks.

use serde::{Deserialize, Serialize};

/// The type of chart to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    /// Vertical or horizontal bar chart.
    Bar,
    /// Line chart (single or multi-series).
    Line,
    /// Pie or donut chart.
    Pie,
    /// Scatter plot (XY).
    Scatter,
    /// Area chart (stacked or unstacked).
    Area,
    /// Combination chart (e.g. bar + line).
    Combo,
    /// Histogram (distribution).
    Histogram,
    /// Candlestick chart (financial: open/high/low/close).
    Candlestick,
    /// Treemap chart (hierarchical area chart).
    Treemap,
    /// Waterfall chart (cumulative positive/negative values).
    Waterfall,
    /// Radar / spider chart (multi-axis polygon).
    Radar,
    /// Bubble chart (scatter with size dimension).
    Bubble,
    /// Gauge / speedometer chart (single value on arc).
    Gauge,
}

impl std::fmt::Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChartType::Bar => write!(f, "bar"),
            ChartType::Line => write!(f, "line"),
            ChartType::Pie => write!(f, "pie"),
            ChartType::Scatter => write!(f, "scatter"),
            ChartType::Area => write!(f, "area"),
            ChartType::Combo => write!(f, "combo"),
            ChartType::Histogram => write!(f, "histogram"),
            ChartType::Candlestick => write!(f, "candlestick"),
            ChartType::Treemap => write!(f, "treemap"),
            ChartType::Waterfall => write!(f, "waterfall"),
            ChartType::Radar => write!(f, "radar"),
            ChartType::Bubble => write!(f, "bubble"),
            ChartType::Gauge => write!(f, "gauge"),
        }
    }
}

/// Data to be rendered in a chart.
///
/// Contains category labels for the x-axis and one or more data series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    /// Category labels for the x-axis.
    pub labels: Vec<String>,
    /// One or more data series to plot.
    pub series: Vec<DataSeries>,
}

/// A single data series within a chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    /// Display name for this series (used in legend).
    pub name: String,
    /// Numeric values for each category.
    pub values: Vec<f64>,
    /// Optional color override (CSS hex, e.g. "#ff0000").
    pub color: Option<String>,
}

/// Rendering options for chart output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOptions {
    /// Chart title displayed at the top.
    pub title: Option<String>,
    /// Chart subtitle displayed below the title.
    pub subtitle: Option<String>,
    /// SVG width in pixels (default 600).
    pub width: u32,
    /// SVG height in pixels (default 400).
    pub height: u32,
    /// Whether to show a legend.
    pub show_legend: bool,
    /// Whether to show grid lines.
    pub show_grid: bool,
    /// Whether to show data labels on data points/bars.
    pub show_data_labels: bool,
    /// Label for the x-axis.
    pub x_axis_label: Option<String>,
    /// Label for the y-axis.
    pub y_axis_label: Option<String>,
    /// Custom color palette (CSS hex strings, e.g. "#ff0000").
    pub color_palette: Option<Vec<String>>,
    /// Background color for the chart (CSS hex, default "#ffffff").
    pub background_color: Option<String>,
    /// Whether to render stacked bars/areas (default false).
    pub stacked: bool,
}

impl Default for ChartOptions {
    fn default() -> Self {
        Self {
            title: None,
            subtitle: None,
            width: 600,
            height: 400,
            show_legend: true,
            show_grid: true,
            show_data_labels: false,
            x_axis_label: None,
            y_axis_label: None,
            color_palette: None,
            background_color: None,
            stacked: false,
        }
    }
}

/// A chart definition within a workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    /// Unique chart identifier.
    pub id: String,
    /// The type of chart.
    pub chart_type: ChartType,
    /// Chart title (optional).
    pub title: Option<String>,
    /// Data range in A1:B2 notation.
    pub data_range: String,
    /// The sheet this chart belongs to.
    pub sheet: String,
    /// X-axis label.
    pub x_axis_label: Option<String>,
    /// Y-axis label.
    pub y_axis_label: Option<String>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Chart {
    /// Create a new chart with default dimensions.
    pub fn new(
        id: impl Into<String>,
        chart_type: ChartType,
        data_range: impl Into<String>,
        sheet: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            chart_type,
            title: None,
            data_range: data_range.into(),
            sheet: sheet.into(),
            x_axis_label: None,
            y_axis_label: None,
            width: 600,
            height: 400,
        }
    }

    /// Set the chart title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Convert this chart definition into `ChartOptions` for rendering.
    pub fn to_options(&self) -> ChartOptions {
        ChartOptions {
            title: self.title.clone(),
            subtitle: None,
            width: self.width,
            height: self.height,
            show_legend: true,
            show_grid: true,
            show_data_labels: false,
            x_axis_label: self.x_axis_label.clone(),
            y_axis_label: self.y_axis_label.clone(),
            color_palette: None,
            background_color: None,
            stacked: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_creation() {
        let chart =
            Chart::new("chart-1", ChartType::Bar, "A1:B5", "Sheet1").with_title("Sales Data");
        assert_eq!(chart.id, "chart-1");
        assert_eq!(chart.chart_type, ChartType::Bar);
        assert_eq!(chart.title, Some("Sales Data".to_string()));
        assert_eq!(chart.data_range, "A1:B5");
        assert_eq!(chart.sheet, "Sheet1");
        assert_eq!(chart.width, 600);
        assert_eq!(chart.height, 400);
    }

    #[test]
    fn test_chart_type_display() {
        assert_eq!(format!("{}", ChartType::Bar), "bar");
        assert_eq!(format!("{}", ChartType::Line), "line");
        assert_eq!(format!("{}", ChartType::Candlestick), "candlestick");
    }

    #[test]
    fn test_chart_type_serde() {
        let json = serde_json::to_string(&ChartType::Scatter).unwrap();
        assert_eq!(json, "\"scatter\"");
        let parsed: ChartType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ChartType::Scatter);
    }

    #[test]
    fn test_chart_data_creation() {
        let data = ChartData {
            labels: vec!["Q1".into(), "Q2".into(), "Q3".into()],
            series: vec![DataSeries {
                name: "Revenue".into(),
                values: vec![100.0, 200.0, 150.0],
                color: Some("#ff0000".into()),
            }],
        };
        assert_eq!(data.labels.len(), 3);
        assert_eq!(data.series[0].name, "Revenue");
        assert_eq!(data.series[0].values, vec![100.0, 200.0, 150.0]);
    }

    #[test]
    fn test_chart_options_default() {
        let opts = ChartOptions::default();
        assert_eq!(opts.width, 600);
        assert_eq!(opts.height, 400);
        assert!(opts.show_legend);
        assert!(opts.show_grid);
        assert!(opts.title.is_none());
        assert!(opts.subtitle.is_none());
        assert!(!opts.show_data_labels);
        assert!(opts.color_palette.is_none());
        assert!(opts.background_color.is_none());
    }

    #[test]
    fn test_chart_to_options() {
        let chart = Chart::new("c1", ChartType::Bar, "A1:B5", "Sheet1").with_title("Sales");
        let opts = chart.to_options();
        assert_eq!(opts.title, Some("Sales".into()));
        assert_eq!(opts.width, 600);
        assert_eq!(opts.height, 400);
    }
}
