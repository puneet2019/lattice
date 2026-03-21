//! Chart rendering — stub SVG output.
//!
//! Will generate SVG markup from chart definitions. The frontend
//! can display this directly or use D3.js for interactive rendering.

use crate::Chart;

/// Render a chart to an SVG string.
///
/// Currently returns a placeholder SVG. Will be implemented with
/// proper SVG generation in Phase 3.
pub fn render_to_svg(chart: &Chart) -> String {
    let title = chart.title.as_deref().unwrap_or("Untitled Chart");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <rect width="100%" height="100%" fill="#f5f5f5" stroke="#cccccc" rx="4"/>
  <text x="50%" y="30" text-anchor="middle" font-family="sans-serif" font-size="16" fill="#333333">{}</text>
  <text x="50%" y="50%" text-anchor="middle" font-family="sans-serif" font-size="12" fill="#999999">
    {} chart - data: {} (rendering not yet implemented)
  </text>
</svg>"##,
        chart.width,
        chart.height,
        chart.width,
        chart.height,
        title,
        chart.chart_type,
        chart.data_range
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChartType;

    #[test]
    fn test_render_to_svg_contains_title() {
        let chart = Chart::new("c1", ChartType::Bar, "A1:B5", "Sheet1").with_title("My Chart");
        let svg = render_to_svg(&chart);
        assert!(svg.contains("My Chart"));
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
