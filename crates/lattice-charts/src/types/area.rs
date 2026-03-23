//! Area chart SVG rendering.
//!
//! Renders area charts with filled regions under lines, support for
//! stacked variants, and semi-transparent fills. Multiple series are
//! layered with opacity for visual distinction.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, data_range_stacked, data_range_with_zero, format_data_label,
    series_color, svg_axis_labels, svg_close, svg_grid_lines, svg_legend, svg_open, svg_text,
    svg_title, xml_escape,
};

/// Render area chart data as an SVG string.
///
/// Draws filled polygons under each series line. When `options.stacked`
/// is true, each series fills from the top of the previous series
/// instead of from the baseline. Areas use semi-transparent fills so
/// overlapping series remain visible.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    if options.stacked {
        render_stacked(data, options)
    } else {
        render_unstacked(data, options)
    }
}

/// Compute the x-position for a point at index `pi` among `n_points`.
fn point_x(pi: usize, n_points: usize, margins: &Margins, pw: f64) -> f64 {
    if n_points > 1 {
        margins.left + (pi as f64 / (n_points - 1) as f64) * pw
    } else {
        margins.left + pw / 2.0
    }
}

/// Map a data value to a pixel y-position within the plot area.
fn value_to_y(value: f64, scale_min: f64, y_range: f64, margins: &Margins, ph: f64) -> f64 {
    let frac = if y_range.abs() > f64::EPSILON {
        (value - scale_min) / y_range
    } else {
        0.5
    };
    margins.top + ph * (1.0 - frac)
}

/// Render unstacked (overlapping) area chart.
fn render_unstacked(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let (dmin, dmax) = data_range_with_zero(data);
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;
    let n_points = data.labels.len().max(1);

    let baseline_y = value_to_y(0.0, scale.min, y_range, &margins, ph);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    for (si, series) in data.series.iter().enumerate() {
        let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

        let mut points: Vec<(f64, f64)> = Vec::with_capacity(n_points);
        for (pi, &value) in series.values.iter().enumerate() {
            let x = point_x(pi, n_points, &margins, pw);
            let y = value_to_y(value, scale.min, y_range, &margins, ph);
            points.push((x, y));
        }

        if points.is_empty() {
            continue;
        }

        // Filled polygon from baseline through data points and back
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

        // Line on top of area
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

        // Data labels
        if options.show_data_labels {
            for (pi, &value) in series.values.iter().enumerate() {
                let x = point_x(pi, n_points, &margins, pw);
                let y = value_to_y(value, scale.min, y_range, &margins, ph);
                svg.push_str(&svg_text(
                    x,
                    y - 6.0,
                    "middle",
                    10,
                    "#333333",
                    &format_data_label(value),
                ));
                svg.push('\n');
            }
        }
    }

    render_x_labels(&mut svg, data, n_points, &margins, pw, ph);

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

/// Render stacked area chart.
///
/// Each series fills from the top of the previous series rather than from
/// the baseline. Series are drawn in order so the first series is at the
/// bottom and the last at the top.
fn render_stacked(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let (dmin, dmax) = data_range_stacked(data);
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;
    let n_points = data.labels.len().max(1);

    let baseline_y = value_to_y(0.0, scale.min, y_range, &margins, ph);

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Build cumulative sums per point-index for each series
    let n_series = data.series.len();
    // cumulative[si][pi] = sum of series 0..=si at point pi
    let mut cumulative: Vec<Vec<f64>> = Vec::with_capacity(n_series);
    for (si, series) in data.series.iter().enumerate() {
        let mut row = Vec::with_capacity(n_points);
        for pi in 0..n_points {
            let val = series.values.get(pi).copied().unwrap_or(0.0).max(0.0);
            let prev = if si > 0 {
                cumulative[si - 1].get(pi).copied().unwrap_or(0.0)
            } else {
                0.0
            };
            row.push(prev + val);
        }
        cumulative.push(row);
    }

    // Draw areas from bottom (series 0) to top (last series).
    // Each area fills between its cumulative line and the previous series'
    // cumulative line (or the baseline for series 0).
    for (si, series) in data.series.iter().enumerate() {
        let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

        // Top edge: cumulative[si]
        let top_points: Vec<(f64, f64)> = (0..n_points)
            .map(|pi| {
                let x = point_x(pi, n_points, &margins, pw);
                let y = value_to_y(cumulative[si][pi], scale.min, y_range, &margins, ph);
                (x, y)
            })
            .collect();

        // Bottom edge: cumulative[si-1] or baseline
        let bottom_points: Vec<(f64, f64)> = if si > 0 {
            (0..n_points)
                .map(|pi| {
                    let x = point_x(pi, n_points, &margins, pw);
                    let y = value_to_y(
                        cumulative[si - 1][pi],
                        scale.min,
                        y_range,
                        &margins,
                        ph,
                    );
                    (x, y)
                })
                .collect()
        } else {
            (0..n_points)
                .map(|pi| {
                    let x = point_x(pi, n_points, &margins, pw);
                    (x, baseline_y)
                })
                .collect()
        };

        if top_points.is_empty() {
            continue;
        }

        // Path: forward along top, backward along bottom, close
        let mut path_data = String::new();
        for (i, &(x, y)) in top_points.iter().enumerate() {
            if i == 0 {
                path_data.push_str(&format!("M {x:.1},{y:.1}"));
            } else {
                path_data.push_str(&format!(" L {x:.1},{y:.1}"));
            }
        }
        for &(x, y) in bottom_points.iter().rev() {
            path_data.push_str(&format!(" L {x:.1},{y:.1}"));
        }
        path_data.push_str(" Z");

        svg.push_str(&format!(
            r#"<path d="{path_data}" fill="{color}" opacity="0.55" stroke="none"/>"#,
        ));
        svg.push('\n');

        // Line along the top edge
        if top_points.len() >= 2 {
            let line_points: String = top_points
                .iter()
                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r#"<polyline points="{line_points}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round"/>"#,
            ));
            svg.push('\n');
        }

        // Data labels at the top edge
        if options.show_data_labels {
            for (pi, &(x, y)) in top_points.iter().enumerate() {
                let raw_value = series.values.get(pi).copied().unwrap_or(0.0);
                svg.push_str(&svg_text(
                    x,
                    y - 6.0,
                    "middle",
                    10,
                    "#333333",
                    &format_data_label(raw_value),
                ));
                svg.push('\n');
            }
        }
    }

    render_x_labels(&mut svg, data, n_points, &margins, pw, ph);

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

/// Render x-axis category labels.
fn render_x_labels(
    svg: &mut String,
    data: &ChartData,
    n_points: usize,
    margins: &Margins,
    pw: f64,
    ph: f64,
) {
    for (pi, label) in data.labels.iter().enumerate() {
        let x = point_x(pi, n_points, margins, pw);
        let y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(x, y, "middle", 10, "#666666", &xml_escape(label)));
        svg.push('\n');
    }
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
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    // --- Stacked area chart tests ---

    #[test]
    fn test_stacked_area_chart_valid_svg() {
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_stacked_area_has_paths_and_lines() {
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        // Two filled areas and two polylines (one per series)
        assert_eq!(svg.matches("<path").count(), 2);
        assert_eq!(svg.matches("<polyline").count(), 2);
    }

    #[test]
    fn test_stacked_area_higher_opacity() {
        // Stacked areas use 0.55 opacity (vs 0.35 for unstacked) since they
        // don't overlap visually.
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        assert!(svg.contains("opacity=\"0.55\""));
    }

    #[test]
    fn test_stacked_area_colors() {
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        assert!(svg.contains("#4e79a7"));
        assert!(svg.contains("#f28e2b"));
    }

    #[test]
    fn test_stacked_area_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&data, &opts);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_stacked_area_single_series() {
        // With a single series, stacked behaves the same as unstacked
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert_eq!(svg.matches("<path").count(), 1);
        assert_eq!(svg.matches("<polyline").count(), 1);
    }

    #[test]
    fn test_stacked_area_with_data_labels() {
        let opts = ChartOptions {
            stacked: true,
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        // Should contain data label text elements with values
        assert!(svg.contains("40"));
        assert!(svg.contains("60"));
        assert!(svg.contains("70"));
    }

    #[test]
    fn test_unstacked_area_with_data_labels() {
        let opts = ChartOptions {
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("100"));
        assert!(svg.contains("250"));
        assert!(svg.contains("300"));
    }
}
