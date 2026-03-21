//! Core chart types and definitions.

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
}
