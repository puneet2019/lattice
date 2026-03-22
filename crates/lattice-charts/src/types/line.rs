//! Line chart SVG rendering.
//!
//! Renders line charts with connected data points, circle markers at
//! each data point, multiple series with distinct colors, and grid lines.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, data_range, series_color, svg_axis_labels, svg_close,
    svg_grid_lines, svg_legend, svg_open, svg_text, svg_title, xml_escape,
};

/// Render line chart data as an SVG string.
///
/// Draws connected lines through data points for each series. Each data
/// point is marked with a small circle. Multiple series use distinct
/// colors from the palette or custom colors.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let (dmin, dmax) = data_range(data);
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;

    let n_points = data.labels.len().max(1);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw each series as a polyline + circle markers
    for (si, series) in data.series.iter().enumerate() {
        let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

        // Build polyline points
        let mut points = Vec::with_capacity(n_points);
        for (pi, &value) in series.values.iter().enumerate() {
            let x = if n_points > 1 {
                margins.left + (pi as f64 / (n_points - 1) as f64) * pw
            } else {
                margins.left + pw / 2.0
            };
            let frac = if y_range.abs() > f64::EPSILON {
                (value - scale.min) / y_range
            } else {
                0.5
            };
            let y = margins.top + ph * (1.0 - frac);
            points.push((x, y));
        }

        // Draw the line
        if points.len() >= 2 {
            let points_str: String = points
                .iter()
                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r##"<polyline points="{points_str}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>"##,
            ));
            svg.push('\n');
        }

        // Draw circle markers
        for &(x, y) in &points {
            svg.push_str(&format!(
                r##"<circle cx="{x:.1}" cy="{y:.1}" r="3.5" fill="{color}" stroke="#ffffff" stroke-width="1.5"/>"##,
            ));
            svg.push('\n');
        }
    }

    // X-axis labels
    for (pi, label) in data.labels.iter().enumerate() {
        let x = if n_points > 1 {
            margins.left + (pi as f64 / (n_points - 1) as f64) * pw
        } else {
            margins.left + pw / 2.0
        };
        let y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(x, y, "middle", 10, "#666666", &xml_escape(label)));
        svg.push('\n');
    }

    // X-axis baseline
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#cccccc" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = margins.top + ph,
    ));
    svg.push('\n');

    svg.push_str(&svg_legend(data, &margins, options));
    svg.push_str(svg_close());
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec![
                "Jan".into(),
                "Feb".into(),
                "Mar".into(),
                "Apr".into(),
                "May".into(),
            ],
            series: vec![DataSeries {
                name: "Temperature".into(),
                values: vec![5.0, 8.0, 15.0, 20.0, 25.0],
                color: None,
            }],
        }
    }

    fn multi_series_data() -> ChartData {
        ChartData {
            labels: vec!["Mon".into(), "Tue".into(), "Wed".into()],
            series: vec![
                DataSeries {
                    name: "CPU".into(),
                    values: vec![40.0, 65.0, 55.0],
                    color: Some("#e15759".into()),
                },
                DataSeries {
                    name: "Memory".into(),
                    values: vec![30.0, 45.0, 50.0],
                    color: Some("#4e79a7".into()),
                },
            ],
        }
    }

    #[test]
    fn test_line_chart_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_line_chart_contains_polyline() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("stroke-width=\"2\""));
    }

    #[test]
    fn test_line_chart_contains_circle_markers() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<circle"));
        // 5 data points = 5 circles
        let circle_count = svg.matches("<circle").count();
        assert_eq!(circle_count, 5);
    }

    #[test]
    fn test_line_chart_with_title() {
        let opts = ChartOptions {
            title: Some("Temperature Trend".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Temperature Trend"));
    }

    #[test]
    fn test_line_chart_x_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Jan"));
        assert!(svg.contains("May"));
    }

    #[test]
    fn test_line_chart_multi_series() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        assert!(svg.contains("#e15759"));
        assert!(svg.contains("#4e79a7"));
        // 2 series, each with 3 points = 2 polylines + 6 circles
        assert_eq!(svg.matches("<polyline").count(), 2);
        assert_eq!(svg.matches("<circle").count(), 6);
    }

    #[test]
    fn test_line_chart_single_point() {
        let data = ChartData {
            labels: vec!["Only".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![42.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Single point: no polyline (needs >= 2 points), but 1 circle
        assert_eq!(svg.matches("<circle").count(), 1);
    }
}
