//! Bar chart SVG rendering.
//!
//! Renders vertical bar charts with support for grouped multiple series,
//! stacked bars, 100% normalized stacked bars, x-axis category labels,
//! y-axis scale with grid lines, and legends.

use crate::chart::{ChartData, ChartOptions};
use crate::svg::{
    Margins, compute_axis_scale, data_range_stacked, data_range_with_zero, format_data_label,
    series_color, svg_axis_labels, svg_close, svg_grid_lines, svg_legend, svg_open, svg_text,
    svg_title, xml_escape,
};

/// Render bar chart data as an SVG string.
///
/// Produces a vertical bar chart. The layout depends on `ChartOptions`:
/// - Default (not stacked): grouped bars side by side per category.
/// - `stacked = true`: bars stacked vertically, each series on top of
///   the previous one.
/// - `stacked = true, normalized = true`: 100% stacked, each stack
///   fills the full height and values are shown as percentages.
pub fn render(data: &ChartData, options: &ChartOptions) -> String {
    if options.stacked {
        render_stacked(data, options)
    } else {
        render_grouped(data, options)
    }
}

/// Render grouped (side-by-side) bars.
fn render_grouped(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    let (dmin, dmax) = data_range_with_zero(data);
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;

    let n_categories = data.labels.len().max(1);
    let n_series = data.series.len().max(1);
    let category_width = pw / n_categories as f64;
    let bar_gap = category_width * 0.1;
    let group_width = category_width - bar_gap * 2.0;
    let bar_width = group_width / n_series as f64;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw bars
    for (ci, label) in data.labels.iter().enumerate() {
        let cat_x = margins.left + ci as f64 * category_width;

        for (si, series) in data.series.iter().enumerate() {
            let value = series.values.get(ci).copied().unwrap_or(0.0);
            let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

            let frac = if y_range.abs() > f64::EPSILON {
                (value - scale.min) / y_range
            } else {
                0.5
            };
            let zero_frac = if y_range.abs() > f64::EPSILON {
                (0.0 - scale.min) / y_range
            } else {
                0.5
            };

            let bar_top = margins.top + ph * (1.0 - frac);
            let zero_y = margins.top + ph * (1.0 - zero_frac);
            let bar_x = cat_x + bar_gap + si as f64 * bar_width;
            let bar_h = (zero_y - bar_top).abs();
            let bar_y = bar_top.min(zero_y);

            svg.push_str(&format!(
                r#"<rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="1"/>"#,
                bx = bar_x,
                by = bar_y,
                bw = bar_width * 0.9,
                bh = bar_h,
            ));
            svg.push('\n');

            // Data label above bar
            if options.show_data_labels {
                let label_x = bar_x + bar_width * 0.45;
                let label_y = bar_y - 4.0;
                svg.push_str(&svg_text(
                    label_x,
                    label_y,
                    "middle",
                    10,
                    "#333333",
                    &format_data_label(value),
                ));
                svg.push('\n');
            }
        }

        // X-axis label
        let label_x = cat_x + category_width / 2.0;
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

    // Linear trendline per series (dashed line connecting bar tops)
    for (si, series) in data.series.iter().enumerate() {
        if series.values.len() >= 2 {
            let color = series.color.as_deref().unwrap_or_else(|| series_color(si));
            let xs: Vec<f64> = (0..series.values.len()).map(|i| i as f64).collect();
            if let Some((slope, intercept)) =
                super::scatter::linear_regression(&xs, &series.values)
            {
                // Evaluate at first and last category
                let trend_y1 = intercept;
                let trend_y2 = slope * (series.values.len() - 1) as f64 + intercept;

                let frac1 = if y_range.abs() > f64::EPSILON {
                    (trend_y1 - scale.min) / y_range
                } else {
                    0.5
                };
                let frac2 = if y_range.abs() > f64::EPSILON {
                    (trend_y2 - scale.min) / y_range
                } else {
                    0.5
                };

                let px1 = margins.left + category_width / 2.0;
                let py1 = margins.top + ph * (1.0 - frac1);
                let px2 = margins.left + (n_categories - 1) as f64 * category_width
                    + category_width / 2.0;
                let py2 = margins.top + ph * (1.0 - frac2);

                svg.push_str(&format!(
                    r##"<line x1="{px1:.1}" y1="{py1:.1}" x2="{px2:.1}" y2="{py2:.1}" stroke="{color}" stroke-width="1" stroke-dasharray="5,3" opacity="0.5"/>"##,
                ));
                svg.push('\n');
            }
        }
    }

    // X-axis baseline
    let zero_frac = if y_range.abs() > f64::EPSILON {
        (0.0 - scale.min) / y_range
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

    svg.push_str(&svg_legend(data, &margins, options));
    svg.push_str(svg_close());
    svg
}

/// Render stacked (or 100% normalized) bars.
fn render_stacked(data: &ChartData, options: &ChartOptions) -> String {
    let margins = Margins::for_options(options);
    let pw = margins.plot_width(options.width);
    let ph = margins.plot_height(options.height);

    // For normalized mode the y-axis is always 0..100%
    let (dmin, dmax) = if options.normalized {
        (0.0, 100.0)
    } else {
        data_range_stacked(data)
    };
    let scale = compute_axis_scale(dmin, dmax);
    let y_range = scale.max - scale.min;

    let n_categories = data.labels.len().max(1);
    let category_width = pw / n_categories as f64;
    let bar_gap = category_width * 0.1;
    let bar_width = category_width - bar_gap * 2.0;

    // Precompute per-category totals for normalized mode
    let category_totals: Vec<f64> = (0..n_categories)
        .map(|ci| {
            data.series
                .iter()
                .map(|s| s.values.get(ci).copied().unwrap_or(0.0).max(0.0))
                .sum::<f64>()
        })
        .collect();

    let mut svg = String::with_capacity(2048);
    svg.push_str(&svg_open(options));
    svg.push('\n');
    svg.push_str(&svg_title(options));
    svg.push('\n');
    svg.push_str(&svg_grid_lines(&scale, &margins, options));
    svg.push_str(&svg_axis_labels(options, &margins));

    // Draw stacked bars (bottom-up)
    for (ci, label) in data.labels.iter().enumerate() {
        let cat_x = margins.left + ci as f64 * category_width;
        let bar_x = cat_x + bar_gap;
        let total = category_totals[ci];

        let mut cumulative = 0.0_f64;

        for (si, series) in data.series.iter().enumerate() {
            let raw_value = series.values.get(ci).copied().unwrap_or(0.0).max(0.0);
            let color = series.color.as_deref().unwrap_or_else(|| series_color(si));

            // Determine the display value (raw or percentage)
            let value = if options.normalized && total > f64::EPSILON {
                (raw_value / total) * 100.0
            } else {
                raw_value
            };

            let base = cumulative;
            cumulative += value;

            // Map cumulative bottom/top to pixel positions
            let base_frac = if y_range.abs() > f64::EPSILON {
                (base - scale.min) / y_range
            } else {
                0.0
            };
            let top_frac = if y_range.abs() > f64::EPSILON {
                (cumulative - scale.min) / y_range
            } else {
                0.0
            };

            let base_y = margins.top + ph * (1.0 - base_frac);
            let top_y = margins.top + ph * (1.0 - top_frac);
            let bar_h = (base_y - top_y).abs().max(0.0);
            let bar_y = top_y.min(base_y);

            svg.push_str(&format!(
                r#"<rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="1"/>"#,
                bx = bar_x,
                by = bar_y,
                bw = bar_width * 0.9,
                bh = bar_h,
            ));
            svg.push('\n');

            // Data label centered inside the stacked segment
            if options.show_data_labels && bar_h > 12.0 {
                let label_x = bar_x + bar_width * 0.45;
                let label_y = bar_y + bar_h / 2.0 + 4.0;
                let display = if options.normalized {
                    format!("{:.0}%", value)
                } else {
                    format_data_label(raw_value)
                };
                svg.push_str(&svg_text(
                    label_x,
                    label_y,
                    "middle",
                    10,
                    "#ffffff",
                    &display,
                ));
                svg.push('\n');
            }
        }

        // X-axis label
        let label_x = cat_x + category_width / 2.0;
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
    let baseline_y = margins.top + ph;
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
            labels: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
            series: vec![DataSeries {
                name: "Revenue".into(),
                values: vec![100.0, 200.0, 150.0, 250.0],
                color: None,
            }],
        }
    }

    fn multi_series_data() -> ChartData {
        ChartData {
            labels: vec!["Jan".into(), "Feb".into(), "Mar".into()],
            series: vec![
                DataSeries {
                    name: "Product A".into(),
                    values: vec![30.0, 50.0, 40.0],
                    color: Some("#ff0000".into()),
                },
                DataSeries {
                    name: "Product B".into(),
                    values: vec![20.0, 35.0, 45.0],
                    color: Some("#0000ff".into()),
                },
            ],
        }
    }

    #[test]
    fn test_bar_chart_valid_svg() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_bar_chart_contains_bars() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<rect"));
        assert!(svg.contains("Q1"));
        assert!(svg.contains("Q4"));
    }

    #[test]
    fn test_bar_chart_with_title() {
        let opts = ChartOptions {
            title: Some("Sales Report".into()),
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        assert!(svg.contains("Sales Report"));
    }

    #[test]
    fn test_bar_chart_multi_series() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains("#0000ff"));
        assert!(svg.contains("Jan"));
        assert!(svg.contains("Mar"));
    }

    #[test]
    fn test_bar_chart_has_grid_lines() {
        let svg = render(&sample_data(), &ChartOptions::default());
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_bar_chart_empty_data() {
        let data = ChartData {
            labels: vec![],
            series: vec![],
        };
        let svg = render(&data, &ChartOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_stacked_bar_chart_valid_svg() {
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_stacked_bar_chart_has_bars() {
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        assert!(svg.contains("#ff0000"));
        assert!(svg.contains("#0000ff"));
        // Two series x 3 categories = 6 rects (plus background rect)
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 7,
            "expected at least 7 rects (1 background + 6 bars), got {rect_count}"
        );
    }

    #[test]
    fn test_stacked_bar_chart_single_category_width() {
        // In stacked mode, bars in the same category share the same x position
        // (they are stacked, not side-by-side).
        let opts = ChartOptions {
            stacked: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        // Extract x positions from bar <rect> elements (those with rx="1")
        let bar_xs: Vec<&str> = svg
            .lines()
            .filter(|l| l.contains("<rect") && l.contains("rx=\"1\""))
            .filter_map(|l| l.split("x=\"").nth(1))
            .filter_map(|s| s.split('"').next())
            .collect();
        // 2 series x 3 categories = 6 bars. Each pair in the same category
        // should share the same x.
        assert_eq!(bar_xs.len(), 6, "expected 6 bar rects");
        // Bars are drawn per-category (ci=0: si=0,si=1; ci=1: si=0,si=1; ...)
        assert_eq!(bar_xs[0], bar_xs[1], "stacked bars in category 0 same x");
        assert_eq!(bar_xs[2], bar_xs[3], "stacked bars in category 1 same x");
        assert_eq!(bar_xs[4], bar_xs[5], "stacked bars in category 2 same x");
    }

    #[test]
    fn test_normalized_bar_chart_valid_svg() {
        let opts = ChartOptions {
            stacked: true,
            normalized: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_normalized_bar_chart_y_axis_100() {
        // The y-axis should go up to 100 in normalized mode
        let opts = ChartOptions {
            stacked: true,
            normalized: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        // The scale should include 100
        assert!(svg.contains("100"));
    }

    #[test]
    fn test_stacked_bar_empty_data() {
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
    fn test_stacked_bar_with_data_labels() {
        let opts = ChartOptions {
            stacked: true,
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        // Data labels should contain the values from the series
        assert!(svg.contains("<text"));
    }

    #[test]
    fn test_normalized_bar_data_labels_show_percent() {
        let opts = ChartOptions {
            stacked: true,
            normalized: true,
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&multi_series_data(), &opts);
        // Labels should contain percentage symbol
        assert!(svg.contains("%"));
    }

    #[test]
    fn test_grouped_bar_with_data_labels() {
        let opts = ChartOptions {
            show_data_labels: true,
            ..ChartOptions::default()
        };
        let svg = render(&sample_data(), &opts);
        // Data labels should contain the values
        assert!(svg.contains("100"));
        assert!(svg.contains("200"));
        assert!(svg.contains("150"));
        assert!(svg.contains("250"));
    }

    #[test]
    fn test_grouped_bar_trendline() {
        let svg = render(&sample_data(), &ChartOptions::default());
        // The sample data has a trend, so a dashed trendline should be present
        assert!(
            svg.contains("stroke-dasharray=\"5,3\""),
            "bar chart should include a dashed trendline"
        );
    }

    #[test]
    fn test_bar_trendline_multi_series() {
        let svg = render(&multi_series_data(), &ChartOptions::default());
        // Two series = two trendlines
        let trendline_count = svg.matches("stroke-dasharray=\"5,3\"").count();
        assert_eq!(trendline_count, 2, "expected 2 trendlines for 2 series");
    }
}
