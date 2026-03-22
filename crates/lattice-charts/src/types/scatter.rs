//! Scatter plot SVG rendering.
//!
//! Renders X-Y coordinate scatter plots with circle markers for each
//! data point, multiple series support, and optional linear regression
//! trendlines.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, format_axis_value, series_color, svg_axis_labels, svg_close,
    svg_open, svg_text, svg_title,
};

/// Render scatter plot data as an SVG string.
///
/// For scatter plots, the labels are treated as x-axis numeric positions.
/// If labels cannot be parsed as numbers, indices (0, 1, 2, ...) are used.
/// Each series contributes a set of (x, y) points rendered as circles.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    // Parse x-values from labels (fall back to indices)
    let x_values: Vec<f64> = data
        .labels
        .iter()
        .enumerate()
        .map(|(i, l)| l.parse::<f64>().unwrap_or(i as f64))
        .collect();

    // Compute x-axis scale
    let (x_min, x_max) = x_range(&x_values);
    let x_scale = compute_axis_scale(x_min, x_max);
    let x_range_val = x_scale.max - x_scale.min;

    // Compute y-axis scale from all series values
    let (y_min, y_max) = y_range_all(data);
    let y_scale = compute_axis_scale(y_min, y_max);
    let y_range_val = y_scale.max - y_scale.min;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Y-axis grid lines and tick labels
    if options.show_grid {
        for &tick in &y_scale.ticks {
            let frac = if y_range_val.abs() > f64::EPSILON {
                (tick - y_scale.min) / y_range_val
            } else {
                0.5
            };
            let y = margins.top + ph * (1.0 - frac);
            svg.push_str(&format!(
                r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#e0e0e0" stroke-width="1"/>"##,
                x1 = margins.left,
                x2 = margins.left + pw,
            ));
            svg.push('\n');
            svg.push_str(&svg_text(
                margins.left - 8.0,
                y + 4.0,
                "end",
                10,
                "#666666",
                &format_axis_value(tick),
            ));
            svg.push('\n');
        }
    }

    // X-axis tick labels
    for &tick in &x_scale.ticks {
        let frac = if x_range_val.abs() > f64::EPSILON {
            (tick - x_scale.min) / x_range_val
        } else {
            0.5
        };
        let x = margins.left + frac * pw;
        let y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(
            x,
            y,
            "middle",
            10,
            "#666666",
            &format_axis_value(tick),
        ));
        svg.push('\n');
    }

    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw scatter points for each series
    for (si, series) in data.series.iter().enumerate() {
        let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

        for (pi, &value) in series.values.iter().enumerate() {
            let xv = x_values.get(pi).copied().unwrap_or(pi as f64);

            let x_frac = if x_range_val.abs() > f64::EPSILON {
                (xv - x_scale.min) / x_range_val
            } else {
                0.5
            };
            let y_frac = if y_range_val.abs() > f64::EPSILON {
                (value - y_scale.min) / y_range_val
            } else {
                0.5
            };

            let px = margins.left + x_frac * pw;
            let py = margins.top + ph * (1.0 - y_frac);

            svg.push_str(&format!(
                r##"<circle cx="{px:.1}" cy="{py:.1}" r="4" fill="{color}" opacity="0.8"/>"##,
            ));
            svg.push('\n');
        }

        // Optional trendline (linear regression)
        if series.values.len() >= 2 {
            let xs: Vec<f64> = (0..series.values.len())
                .map(|i| x_values.get(i).copied().unwrap_or(i as f64))
                .collect();
            if let Some((slope, intercept)) = linear_regression(&xs, &series.values) {
                let trend_x1 = x_scale.min;
                let trend_x2 = x_scale.max;
                let trend_y1 = slope * trend_x1 + intercept;
                let trend_y2 = slope * trend_x2 + intercept;

                let px1 = margins.left;
                let px2 = margins.left + pw;
                let py1 = margins.top
                    + ph * (1.0
                        - if y_range_val.abs() > f64::EPSILON {
                            (trend_y1 - y_scale.min) / y_range_val
                        } else {
                            0.5
                        });
                let py2 = margins.top
                    + ph * (1.0
                        - if y_range_val.abs() > f64::EPSILON {
                            (trend_y2 - y_scale.min) / y_range_val
                        } else {
                            0.5
                        });

                svg.push_str(&format!(
                    r##"<line x1="{px1:.1}" y1="{py1:.1}" x2="{px2:.1}" y2="{py2:.1}" stroke="{color}" stroke-width="1" stroke-dasharray="5,3" opacity="0.5"/>"##,
                ));
                svg.push('\n');
            }
        }
    }

    // Axes
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        y1 = margins.top + ph,
        x2 = margins.left + pw,
        y2 = margins.top + ph,
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"<line x1="{x:.1}" y1="{y1:.1}" x2="{x:.1}" y2="{y2:.1}" stroke="#333333" stroke-width="1"/>"##,
        x = margins.left,
        y1 = margins.top,
        y2 = margins.top + ph,
    ));
    svg.push('\n');

    // Legend
    svg.push_str(&crate::svg::svg_legend(data, &margins, options));
    svg.push_str(svg_close());
    svg
}

/// Compute min/max for x-values.
fn x_range(values: &[f64]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in values {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    if min > max { (0.0, 1.0) } else { (min, max) }
}

/// Compute min/max y-values across all series.
fn y_range_all(data: &ChartData) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for s in &data.series {
        for &v in &s.values {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    if min > max { (0.0, 1.0) } else { (min, max) }
}

/// Simple linear regression: returns (slope, intercept) or None if degenerate.
fn linear_regression(xs: &[f64], ys: &[f64]) -> Option<(f64, f64)> {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let sum_x: f64 = xs[..n].iter().sum();
    let sum_y: f64 = ys[..n].iter().sum();
    let sum_xy: f64 = xs[..n].iter().zip(&ys[..n]).map(|(x, y)| x * y).sum();
    let sum_x2: f64 = xs[..n].iter().map(|x| x * x).sum();

    let denom = nf * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let slope = (nf * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / nf;
    Some((slope, intercept))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::DataSeries;

    fn sample_data() -> ChartData {
        ChartData {
            labels: vec!["1".into(), "2".into(), "3".into(), "4".into(), "5".into()],
            series: vec![DataSeries {
                name: "Observations".into(),
                values: vec![2.1, 4.0, 5.8, 8.2, 9.5],
                color: None,
            }],
        }
    }

    #[test]
    fn test_scatter_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_scatter_contains_circles() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert_eq!(svg.matches("<circle").count(), 5);
    }

    #[test]
    fn test_scatter_contains_trendline() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Trendline is a dashed line
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn test_scatter_with_title() {
        let opts = ChartOptions {
            title: Some("Correlation".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Correlation"));
    }

    #[test]
    fn test_scatter_axes() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // Should have axis lines
        let line_count = svg.matches("<line").count();
        assert!(line_count >= 2); // at least x and y axis
    }

    #[test]
    fn test_linear_regression_basic() {
        let xs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let (slope, intercept) = linear_regression(&xs, &ys).unwrap();
        assert!((slope - 2.0).abs() < 1e-10);
        assert!((intercept - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_regression_insufficient_data() {
        assert!(linear_regression(&[1.0], &[2.0]).is_none());
        assert!(linear_regression(&[], &[]).is_none());
    }

    #[test]
    fn test_scatter_non_numeric_labels() {
        let data = ChartData {
            labels: vec!["a".into(), "b".into(), "c".into()],
            series: vec![DataSeries {
                name: "S".into(),
                values: vec![10.0, 20.0, 30.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Falls back to indices 0, 1, 2
        assert_eq!(svg.matches("<circle").count(), 3);
    }
}
