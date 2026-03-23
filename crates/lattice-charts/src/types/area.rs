//! Area chart SVG rendering.
//!
//! Renders area charts with filled regions under lines, support for
//! stacked variants, and semi-transparent fills. Multiple series are
//! layered with opacity for visual distinction.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, data_range_with_zero, series_color, svg_axis_labels, svg_close,
    svg_grid_lines, svg_legend, svg_open, svg_text, svg_title, xml_escape,
};

/// Render area chart data as an SVG string.
///
/// Draws filled polygons under each series line. Areas use
/// semi-transparent fills so overlapping series remain visible.
/// The y-axis always includes zero for proper area rendering.
/// When `options.stacked` is true, series values are accumulated.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    if options.stacked {
        return render_stacked(data, options);
    }
    render_unstacked(data, options)
}

/// Render a stacked area chart where series values accumulate.
fn render_stacked(data: &ChartData, options: &ChartOptions) -> String {
    use crate::svg::compute_axis_scale;

    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let n_points = data.labels.len().max(1);

    // Compute cumulative stacked values per point.
    let n_series = data.series.len();
    // cumulative[si][pi] = sum of series 0..=si at point pi
    let mut cumulative: Vec<Vec<f64>> = Vec::with_capacity(n_series);
    for si in 0..n_series {
        let mut row = Vec::with_capacity(n_points);
        for pi in 0..n_points {
            let val = data.series[si].values.get(pi).copied().unwrap_or(0.0).max(0.0);
            let prev = if si > 0 { cumulative[si - 1][pi] } else { 0.0 };
            row.push(prev + val);
        }
        cumulative.push(row);
    }

    // Max stacked value for scale.
    let stacked_max = cumulative
        .last()
        .map(|row| row.iter().cloned().fold(0.0_f64, f64::max))
        .unwrap_or(0.0);

    let scale = compute_axis_scale(0.0, stacked_max);
    let y_range = scale.max - scale.min;
    let baseline_y = margins.top + ph;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    let x_for = |pi: usize| -> f64 {
        if n_points > 1 {
            margins.left + (pi as f64 / (n_points - 1) as f64) * pw
        } else {
            margins.left + pw / 2.0
        }
    };

    let y_for = |val: f64| -> f64 {
        let frac = if y_range.abs() > f64::EPSILON {
            (val - scale.min) / y_range
        } else {
            0.5
        };
        margins.top + ph * (1.0 - frac)
    };

    // Render from back (topmost series) to front so lower series draw on top.
    for si in (0..n_series).rev() {
        let color = data.series[si]
            .color
            .as_deref()
            .unwrap_or_else(|| series_color(si));

        // Upper boundary = cumulative[si]
        // Lower boundary = cumulative[si-1] or zero
        let mut path = String::new();
        // Forward along upper boundary
        for pi in 0..n_points {
            let x = x_for(pi);
            let y = y_for(cumulative[si][pi]);
            if pi == 0 {
                path.push_str(&format!("M {x:.1},{y:.1}"));
            } else {
                path.push_str(&format!(" L {x:.1},{y:.1}"));
            }
        }
        // Backward along lower boundary
        for pi in (0..n_points).rev() {
            let lower = if si > 0 { cumulative[si - 1][pi] } else { 0.0 };
            let x = x_for(pi);
            let y = y_for(lower);
            path.push_str(&format!(" L {x:.1},{y:.1}"));
        }
        path.push_str(" Z");

        svg.push_str(&format!(
            r#"<path d="{path}" fill="{color}" opacity="0.6" stroke="{color}" stroke-width="1.5"/>"#,
        ));
        svg.push('\n');
    }

    // X-axis labels
    for (pi, label) in data.labels.iter().enumerate() {
        let x = x_for(pi);
        let y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(x, y, "middle", 10, "#666666", &xml_escape(label)));
        svg.push('\n');
    }

    // X-axis baseline
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = baseline_y,
    ));
    svg.push('\n');

    svg.push_str(&svg_legend(data, &margins, options));
    svg.push_str(svg_close());
    svg
}

/// Render an unstacked (overlapping) area chart.
fn render_unstacked(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let (dmin, dmax) = data_range_with_zero(data);
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;
    let n_points = data.labels.len().max(1);

    // Baseline y-position (where value = 0)
    let zero_frac = if y_range.abs() > f64::EPSILON {
        (0.0 - scale.min) / y_range
    } else {
        0.5
    };
    let baseline_y = margins.top + ph * (1.0 - zero_frac);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw each series as a filled area (back to front)
    for (si, series) in data.series.iter().enumerate() {
        let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

        // Compute data points
        let mut points: Vec<(f64, f64)> = Vec::with_capacity(n_points);
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

        if points.is_empty() {
            continue;
        }

        // Build filled polygon: first point at baseline, up through data, back to baseline
        let first_x = points.first().map(|p| p.0).unwrap_or(margins.left);
        let last_x = points.last().map(|p| p.0).unwrap_or(margins.left + pw);

        let mut path_data = format!("M {first_x:.1},{baseline_y:.1}");
        for &(x, y) in &points {
            path_data.push_str(&format!(" L {x:.1},{y:.1}"));
        }
        path_data.push_str(&format!(" L {last_x:.1},{baseline_y:.1} Z"));

        svg.push_str(&format!(
            r#"<path d="{path_data}" fill="{color}" opacity="0.35" stroke="none"/>"#,
        ));
        svg.push('\n');

        // Draw the line on top of the area
        if points.len() >= 2 {
            let line_points: String = points
                .iter()
                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r#"<polyline points="{line_points}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round"/>"#,
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
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = baseline_y,
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
            labels: vec!["Jan".into(), "Feb".into(), "Mar".into(), "Apr".into()],
            series: vec![DataSeries {
                name: "Downloads".into(),
                values: vec![100.0, 250.0, 180.0, 300.0],
                color: None,
            }],
        }
    }

    fn multi_series_data() -> ChartData {
        ChartData {
            labels: vec!["Q1".into(), "Q2".into(), "Q3".into()],
            series: vec![
                DataSeries {
                    name: "Mobile".into(),
                    values: vec![40.0, 60.0, 55.0],
                    color: Some("#4e79a7".into()),
                },
                DataSeries {
                    name: "Desktop".into(),
                    values: vec![70.0, 50.0, 65.0],
                    color: Some("#f28e2b".into()),
                },
            ],
        }
    }

    #[test]
    fn test_area_chart_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_area_chart_contains_filled_path() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Filled area path
        assert!(svg.contains("<path"));
        assert!(svg.contains("opacity=\"0.35\""));
    }

    #[test]
    fn test_area_chart_contains_line_on_top() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<polyline"));
        assert!(svg.contains("stroke-width=\"2\""));
    }

    #[test]
    fn test_area_chart_x_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Jan"));
        assert!(svg.contains("Apr"));
    }

    #[test]
    fn test_area_chart_with_title() {
        let opts = ChartOptions {
            title: Some("Traffic Over Time".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Traffic Over Time"));
    }

    #[test]
    fn test_area_chart_multi_series() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        // Two filled areas and two polylines
        assert_eq!(svg.matches("<path").count(), 2);
        assert_eq!(svg.matches("<polyline").count(), 2);
        assert!(svg.contains("#4e79a7"));
        assert!(svg.contains("#f28e2b"));
    }

    #[test]
    fn test_area_chart_has_grid() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_area_chart_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_area_chart_no_grid() {
        let opts = ChartOptions {
            show_grid: false,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        // Should still have the baseline but fewer grid lines
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }
}
