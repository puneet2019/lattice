//! SVG chart rendering — dispatch to type-specific renderers.
//!
//! The `render_chart` function is the main entry point. It accepts a chart
//! type, data, and options and dispatches to the appropriate renderer in
//! the `types` module. The legacy `render_to_svg` function is kept for
//! backward compatibility.

use crate::chart::{ChartData, ChartOptions, ChartType};
use crate::svg::xml_escape;
use crate::types;
use crate::Chart;

/// Render chart data to an SVG string based on the given chart type.
///
/// This is the main entry point for chart rendering. It dispatches to
/// chart-type-specific renderers in the `types` module.
pub fn render_chart(chart_type: &ChartType, data: &ChartData, options: &ChartOptions) -> String {
    match chart_type {
        ChartType::Bar => types::bar::render(data, options),
        ChartType::Line => types::line::render(data, options),
        ChartType::Pie => types::pie::render(data, options),
        ChartType::Scatter => types::scatter::render(data, options),
        ChartType::Area => types::area::render(data, options),
        ChartType::Combo => types::combo::render(data, options),
        _ => placeholder_svg(chart_type, options),
    }
}

/// Render a chart definition (legacy API, kept for backward compatibility).
///
/// Returns a placeholder SVG. For full rendering, use `render_chart` with
/// extracted `ChartData` and `ChartOptions`.
pub fn render_to_svg(chart: &Chart) -> String {
    let title = chart.title.as_deref().unwrap_or("Untitled Chart");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <rect width="100%" height="100%" fill="#f5f5f5" stroke="#cccccc" rx="4"/>
  <text x="50%" y="30" text-anchor="middle" font-family="sans-serif" font-size="16" fill="#333333">{}</text>
  <text x="50%" y="50%" text-anchor="middle" font-family="sans-serif" font-size="12" fill="#999999">
    {} chart - data: {} (use render_chart for full rendering)
  </text>
</svg>"##,
        chart.width,
        chart.height,
        chart.width,
        chart.height,
        xml_escape(title),
        chart.chart_type,
        xml_escape(&chart.data_range)
    )
}

/// Placeholder SVG for chart types not yet implemented.
fn placeholder_svg(chart_type: &ChartType, options: &ChartOptions) -> String {
    let title = options.title.as_deref().unwrap_or("Chart");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <rect width="100%" height="100%" fill="#f5f5f5" stroke="#cccccc" rx="4"/>
  <text x="50%" y="50%" text-anchor="middle" font-family="sans-serif" font-size="14" fill="#999">{ct} chart &quot;{t}&quot; — not yet implemented</text>
</svg>"##,
        w = options.width,
        h = options.height,
        ct = chart_type,
        t = xml_escape(title),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{ChartData, DataSeries};

    #[test]
    fn test_render_to_svg_contains_title() {
        let chart = Chart::new("c1", ChartType::Bar, "A1:B5", "Sheet1").with_title("My Chart");
        let svg = render_to_svg(&chart);
        assert!(svg.contains("My Chart"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_render_chart_placeholder() {
        let data = ChartData {
            labels: vec!["A".into()],
            series: vec![DataSeries {
                name: "S1".into(),
                values: vec![10.0],
                color: None,
            }],
        };
        let opts = ChartOptions::default();
        // Combo is now implemented — verify it renders a real SVG
        let svg = render_chart(&ChartType::Combo, &data, &opts);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(!svg.contains("not yet implemented"));
    }
}
