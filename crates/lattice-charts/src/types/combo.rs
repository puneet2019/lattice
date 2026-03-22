//! Combo chart SVG rendering.
//!
//! Renders combination charts that overlay bar and line series on the
//! same axes. The first data series is rendered as bars; all subsequent
//! series are rendered as lines with circle markers. When the secondary
//! series values differ significantly in scale from the primary, a
//! secondary y-axis is drawn on the right side.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    compute_axis_scale, data_range_with_zero, format_axis_value, series_color, svg_axis_labels,
    svg_close, svg_grid_lines, svg_legend, svg_open, svg_text, svg_title, xml_escape, Margins,
};

/// Scale divergence threshold — if the secondary series range differs from
/// the primary by more than this factor, a dual y-axis is used.
const DUAL_AXIS_THRESHOLD: f64 = 3.0;

/// Render combo chart data as an SVG string.
///
/// The first series is drawn as vertical bars. Remaining series are drawn
/// as lines with circle markers. If the value ranges diverge significantly,
/// a secondary y-axis is rendered on the right side for the line series.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let n_categories = data.labels.len().max(1);

    // Split into bar series (first) and line series (rest)
    let bar_series = data.series.first();
    let line_series = &data.series[1.min(data.series.len())..];

    // Compute bar (primary) y-axis scale
    let bar_data = ChartData {
        labels: data.labels.clone(),
        series: bar_series.into_iter().cloned().collect(),
    };
    let (bar_min, bar_max) = data_range_with_zero(&bar_data);
    let bar_scale = compute_axis_scale(bar_min, bar_max);
    let bar_y_range = bar_scale.max - bar_scale.min;

    // Compute line (secondary) y-axis scale
    let line_data = ChartData {
        labels: data.labels.clone(),
        series: line_series.to_vec(),
    };
    let (line_min, line_max) = data_range_with_zero(&line_data);
    let line_scale = compute_axis_scale(line_min, line_max);
    let line_y_range = line_scale.max - line_scale.min;

    // Determine whether to use dual axes
    let needs_dual_axis = !line_series.is_empty() && {
        let bar_range = bar_max - bar_min;
        let ln_range = line_max - line_min;
        if bar_range.abs() < f64::EPSILON || ln_range.abs() < f64::EPSILON {
            false
        } else {
            let ratio = bar_range / ln_range;
            ratio > DUAL_AXIS_THRESHOLD || ratio < 1.0 / DUAL_AXIS_THRESHOLD
        }
    };

    let mut svg = String::with_capacity(4096);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');

    // Grid lines (always from primary/bar scale)
    svg.push_str(&svg_grid_lines(&bar_scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // -- Draw bars (first series) --
    if let Some(series) = bar_series {
        let category_width = pw / n_categories as f64;
        let bar_gap = category_width * 0.15;
        let bar_width = category_width - bar_gap * 2.0;
        let color = series.color.as_deref().unwrap_or_else(|| series_color(0));

        let zero_frac = if bar_y_range.abs() > f64::EPSILON {
            (0.0 - bar_scale.min) / bar_y_range
        } else {
            0.5
        };
        let zero_y = margins.top + ph * (1.0 - zero_frac);

        for (ci, _label) in data.labels.iter().enumerate() {
            let value = series.values.get(ci).copied().unwrap_or(0.0);
            let frac = if bar_y_range.abs() > f64::EPSILON {
                (value - bar_scale.min) / bar_y_range
            } else {
                0.5
            };
            let bar_top = margins.top + ph * (1.0 - frac);
            let bar_x = margins.left + ci as f64 * category_width + bar_gap;
            let bar_h = (zero_y - bar_top).abs();
            let bar_y = bar_top.min(zero_y);

            svg.push_str(&format!(
                r#"<rect class="combo-bar" x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="1"/>"#,
                bx = bar_x,
                by = bar_y,
                bw = bar_width,
                bh = bar_h,
            ));
            svg.push('\n');
        }
    }

    // -- Draw line series (remaining series) --
    for (li, series) in line_series.iter().enumerate() {
        let si = li + 1; // overall series index (0 is the bar series)
        let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

        // Choose scale: if dual axis, lines use the secondary scale
        let (scale_ref, y_rng) = if needs_dual_axis {
            (&line_scale, line_y_range)
        } else {
            (&bar_scale, bar_y_range)
        };

        let mut points = Vec::with_capacity(n_categories);
        for (pi, &value) in series.values.iter().enumerate() {
            let x = margins.left
                + (pi as f64 + 0.5) * (pw / n_categories as f64);
            let frac = if y_rng.abs() > f64::EPSILON {
                (value - scale_ref.min) / y_rng
            } else {
                0.5
            };
            let y = margins.top + ph * (1.0 - frac);
            points.push((x, y));
        }

        // Polyline
        if points.len() >= 2 {
            let points_str: String = points
                .iter()
                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                r#"<polyline class="combo-line" points="{points_str}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>"#,
            ));
            svg.push('\n');
        }

        // Circle markers
        for &(x, y) in &points {
            svg.push_str(&format!(
                r##"<circle cx="{x:.1}" cy="{y:.1}" r="3.5" fill="{color}" stroke="#ffffff" stroke-width="1.5"/>"##,
            ));
            svg.push('\n');
        }
    }

    // X-axis labels
    let category_width = pw / n_categories as f64;
    for (ci, label) in data.labels.iter().enumerate() {
        let label_x = margins.left + (ci as f64 + 0.5) * category_width;
        let label_y = margins.top + ph + 18.0;
        svg.push_str(&svg_text(
            label_x,
            label_y,
            "middle",
            10,
            "#666666",
            &xml_escape(label),
        ));
        svg.push('\n');
    }

    // X-axis baseline
    let zero_frac = if bar_y_range.abs() > f64::EPSILON {
        (0.0 - bar_scale.min) / bar_y_range
    } else {
        0.5
    };
    let baseline_y = margins.top + ph * (1.0 - zero_frac);
    svg.push_str(&format!(
        r##"<line x1="{x1:.1}" y1="{y:.1}" x2="{x2:.1}" y2="{y:.1}" stroke="#333333" stroke-width="1"/>"##,
        x1 = margins.left,
        x2 = margins.left + pw,
        y = baseline_y,
    ));
    svg.push('\n');

    // Secondary y-axis (right side) when dual-axis mode
    if needs_dual_axis {
        let right_x = margins.left + pw;
        svg.push_str(&format!(
            r##"<line x1="{x:.1}" y1="{y1:.1}" x2="{x:.1}" y2="{y2:.1}" stroke="#333333" stroke-width="1"/>"##,
            x = right_x,
            y1 = margins.top,
            y2 = margins.top + ph,
        ));
        svg.push('\n');

        // Right-side tick labels
        for &tick in &line_scale.ticks {
            let frac = if line_y_range.abs() > f64::EPSILON {
                (tick - line_scale.min) / line_y_range
            } else {
                0.5
            };
            let y = margins.top + ph * (1.0 - frac);
            svg.push_str(&svg_text(
                right_x + 8.0,
                y + 4.0,
                "start",
                10,
                "#666666",
                &format_axis_value(tick),
            ));
            svg.push('\n');
        }
    }

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
            labels: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
            series: vec![
                DataSeries {
                    name: "Revenue".into(),
                    values: vec![100.0, 200.0, 150.0, 250.0],
                    color: Some("#4e79a7".into()),
                },
                DataSeries {
                    name: "Growth %".into(),
                    values: vec![10.0, 15.0, 8.0, 20.0],
                    color: Some("#e15759".into()),
                },
            ],
        }
    }

    fn dual_axis_data() -> ChartData {
        ChartData {
            labels: vec!["Jan".into(), "Feb".into(), "Mar".into()],
            series: vec![
                DataSeries {
                    name: "Revenue ($M)".into(),
                    values: vec![5000.0, 7000.0, 6000.0],
                    color: Some("#4e79a7".into()),
                },
                DataSeries {
                    name: "Conversion %".into(),
                    values: vec![2.5, 3.1, 2.8],
                    color: Some("#e15759".into()),
                },
            ],
        }
    }

    #[test]
    fn test_combo_chart_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_combo_chart_contains_bars() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("combo-bar"));
        // 4 categories = 4 bars for the first series
        let bar_count = svg.matches("combo-bar").count();
        assert_eq!(bar_count, 4);
    }

    #[test]
    fn test_combo_chart_contains_line() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("combo-line"));
        assert!(svg.contains("<polyline"));
    }

    #[test]
    fn test_combo_chart_contains_circle_markers() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // 4 data points for the line series
        let circle_count = svg.matches("<circle").count();
        assert_eq!(circle_count, 4);
    }

    #[test]
    fn test_combo_chart_x_labels() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("Q1"));
        assert!(svg.contains("Q4"));
    }

    #[test]
    fn test_combo_chart_with_title() {
        let opts = ChartOptions {
            title: Some("Revenue & Growth".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Revenue &amp; Growth"));
    }

    #[test]
    fn test_combo_chart_colors() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("#4e79a7")); // bar color
        assert!(svg.contains("#e15759")); // line color
    }

    #[test]
    fn test_combo_chart_dual_axis() {
        let svg = render(&dual_axis_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Dual axis should have right-side axis line and tick labels
        // The right-side axis is a vertical line at the right edge
        // and tick labels with text-anchor="start"
        assert!(svg.contains(r#"text-anchor="start""#));
    }

    #[test]
    fn test_combo_chart_single_series_no_line() {
        let data = ChartData {
            labels: vec!["A".into(), "B".into()],
            series: vec![DataSeries {
                name: "Only Bars".into(),
                values: vec![10.0, 20.0],
                color: None,
            }],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        // Bars present, no lines
        assert!(svg.contains("combo-bar"));
        assert!(!svg.contains("combo-line"));
        assert!(!svg.contains("<polyline"));
    }

    #[test]
    fn test_combo_chart_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_combo_chart_has_grid_and_baseline() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<line"));
        assert!(svg.contains(r##"stroke="#333333""##));
    }

    #[test]
    fn test_combo_chart_three_series() {
        let data = ChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            series: vec![
                DataSeries {
                    name: "Bars".into(),
                    values: vec![100.0, 200.0, 150.0],
                    color: None,
                },
                DataSeries {
                    name: "Line 1".into(),
                    values: vec![80.0, 180.0, 120.0],
                    color: None,
                },
                DataSeries {
                    name: "Line 2".into(),
                    values: vec![50.0, 90.0, 70.0],
                    color: None,
                },
            ],
        };
        let svg = render(&data, &ChartOptions::default());
        // 3 bars + 2 polylines + 6 circles
        assert_eq!(svg.matches("combo-bar").count(), 3);
        assert_eq!(svg.matches("combo-line").count(), 2);
        assert_eq!(svg.matches("<circle").count(), 6);
    }
}
